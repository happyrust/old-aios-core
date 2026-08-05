//! ADR-017 暂存与写回：常驻暂存实例句柄与「批次执行上下文」读路由点。
//!
//! 稳态增量窗口的计算期间，读入口要看到「持久层提交态 + 本窗口暂存写入」组成的
//! 窗口新态。路由采用 task-local 上下文：窗口计算的整棵调用树跑在
//! [`with_staging_reads`] 的作用域里，被接线的读入口（`get_pe` /
//! `get_named_attmap` / `get_world_transform` 等）在上下文在场时改查暂存库。
//!
//! fail-closed 纪律：上下文在场时被接线的读**只**打暂存库，任何 miss / 错误都
//! 原样上抛，绝不静默回落持久层——回落等于把旧世界的数据缝进新世界的计算，
//! 是静默错模型（R1）。miss 的兜底（定点解析 / 点查拷入）由上层显式执行后重试，
//! 不在路由层发生。
//!
//! 控制面（水位、attempts、queue_control、durable pending）不走读路由：那些
//! 模块直接持有 `SUL_DB`，按 ADR-017 ④ 永远直读直写持久层。

use once_cell::sync::Lazy;
use surrealdb::engine::any::Any;
use surrealdb::Surreal;

/// 进程常驻的嵌入式暂存实例句柄（`mem://`），与 `SUL_DB` / `SECOND_SUL_DB` 并列。
///
/// 由应用启动时连接一次（gen-model 的暂存生命周期模块负责 connect 与
/// database 生命周期管理）；这里只提供静态句柄本身。
pub static STAGE_DB: Lazy<Surreal<Any>> = Lazy::new(Surreal::init);

tokio::task_local! {
    static STAGING_READS: StagingReadContext;
}

/// 批次执行上下文：一个提交单元的暂存库句柄 + 供日志与断言用的库名。
#[derive(Clone)]
pub struct StagingReadContext {
    db: Surreal<Any>,
    label: String,
}

impl StagingReadContext {
    /// `db` 必须已 `use_ns`/`use_db` 到该提交单元的 staging database；
    /// `label` 是该库名（`staging_{dbnum}_{window_id}`）。
    pub fn new(db: Surreal<Any>, label: impl Into<String>) -> Self {
        Self {
            db,
            label: label.into(),
        }
    }

    pub fn db(&self) -> &Surreal<Any> {
        &self.db
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

impl std::fmt::Debug for StagingReadContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StagingReadContext")
            .field("label", &self.label)
            .finish()
    }
}

/// 在暂存读上下文中运行一个 future：作用域内（含 `.await` 链上的全部调用）
/// 被接线的读入口路由到 `ctx.db()`。作用域退出即还原，不会泄漏到其他任务；
/// 嵌套时以最内层为准。
pub async fn with_staging_reads<F>(ctx: StagingReadContext, fut: F) -> F::Output
where
    F: std::future::Future,
{
    STAGING_READS.scope(ctx, fut).await
}

/// 当前 task 的暂存读上下文（无则 `None`）。
///
/// 被接线的读入口以它分流；上下文在场时结果**不得**写进进程级 `#[cached]`
/// 缓存——那些缓存只属于持久层世界，键里没有「世界」这一维，混写会把暂存值
/// 泄漏给持久层读者（反之亦然）。
pub fn active_staging_reads() -> Option<StagingReadContext> {
    STAGING_READS.try_with(|ctx| ctx.clone()).ok()
}

/// `tokio::spawn` 不继承 task-local；生成链的子任务统一从这里派生，避免上下文
/// 在并行边界丢失后静默回到持久层。
pub fn spawn_with_staging_reads<F>(future: F) -> tokio::task::JoinHandle<F::Output>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    let context = active_staging_reads();
    tokio::spawn(async move {
        match context {
            Some(context) => with_staging_reads(context, future).await,
            None => future.await,
        }
    })
}
