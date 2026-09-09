//! ADR-053 direct 读路由：生成期数据读取不查 SurrealDB，改由调用方注入的 provider
//! 从 E3D 库文件现场取数。
//!
//! 结构照 [`super::staging`] 拉：task-local 上下文 + 被接线的读入口在函数开头分流。
//! 两者的区别只有一处，但很要紧 —— staging 换的是**同一套 SurrealQL 打在哪个库上**，
//! direct 换的是**数据从哪来**，文件侧根本没有 SurrealQL。所以 staging 能有一个
//! `data_db()` 让计算体不知道自己在哪个世界，direct 不能：每个收口函数都得显式把
//! 自己的语义交给 provider 的一个方法。
//!
//! ## fail loud
//!
//! 这是本模块最硬的一条纪律，也是它设计成现在这样的原因。
//!
//! [`DirectReadProvider`] 的每个方法都有默认实现，默认实现返回 [`unsupported`]。
//! 于是「provider 没实现这个查询」和「报错」是同一件事，**不需要任何一个接线点记得
//! 去写 fail loud 分支** —— 忘写路由会让查询走回 Surreal（那是零回归要的行为），
//! 但只要路由写了、provider 没实现，就一定报错。
//!
//! 绝不静默回落 Surreal：direct 上下文在场时回落，等于让双跑对拍读到同一个库，
//! 两边产物当然一致 —— 对拍假绿比不对拍更糟，因为它给出的是「已验证」的结论。
//!
//! ## 与 staging 互斥（ADR-053 R6）
//!
//! 生成期不在暂存窗口内，两个上下文没有合法的同时在场场景。真同时在场了，
//! 「读暂存库」与「读文件」谁赢将由每个收口函数的分支顺序决定 —— 那是靠代码书写顺序
//! 决定语义，[`with_direct_reads`] 因此直接拒绝进入。

use std::collections::BTreeSet;
use std::sync::Arc;

use glam::{DVec3, Vec3};

use crate::pe::SPdmsElement;
use crate::{NamedAttrMap, RefnoEnum};

/// 生成期读数据的供给面：**定义在 aios_core，实现在 gen-model**。
///
/// 这个方向是 ADR-053 定的：aios_core 不能反过来依赖读取器（e3d-io），
/// 否则底层库要跟着上层的解析栈走。
///
/// ## 只有叶子读在这里
///
/// 收口函数分两种。**叶子**自己打 SurrealQL，它们在这里各占一个方法。**复合**只是
/// 再调别的收口函数（`get_or_create_cata_context` 第一行就是 `get_named_attmap`；
/// `get_world_transform` 走 `get_ancestor_attmaps` / `query_ancestor_refnos` /
/// `get_spline_pts`；`query_multi_filter_deep_children` 循环 `query_filter_deep_children`），
/// 它们**不在这里**：叶子路由好了，复合自然就对。
///
/// 把复合也搬进 provider 会要求实现方重写一遍组合逻辑，于是同一套「先取谁再取谁」
/// 在两侧各存一份 —— 那正是 ADR-053 R1（语义漂移）说的东西。
///
/// ## 三条契约语义
///
/// 见 `gen-model/docs/specs/direct-mode-query-surface.md` §6.5，实测数字在那里。摘要：
///
/// 1. **children 族返回的顺序是语义**。BRAN 的成员序就是管路走向。必须用记录自带的
///    成员表原序，**不得排序、不得从索引反向重建**。实测 `ams8000_0001` 上有 6 个元素
///    的成员序不等于 refno 序 —— 集合一致会让对拍绿，顺序错了模型才错。
/// 2. **`get_cat_refno` 的引用链 82% 跨库**（实测 6461/7876 指向其他库）。跨库定位是
///    实现方的事，但实现方必须做得到。对照：owner 链不跨库，祖先上溯单库句柄就够。
/// 3. **时点按 dbnum pin `applied_sesno`**（ADR-053 Q3）。读文件最新态会与 DB 模式分叉，
///    对拍随即失去意义。
#[async_trait::async_trait]
pub trait DirectReadProvider: Send + Sync + std::fmt::Debug {
    /// 供日志与断言用的名字，例如 `direct(ams8000@263)`。
    fn label(&self) -> &str;

    async fn get_named_attmap(&self, refno: RefnoEnum) -> anyhow::Result<NamedAttrMap> {
        Err(unsupported("get_named_attmap", refno))
    }

    async fn get_type_name(&self, refno: RefnoEnum) -> anyhow::Result<String> {
        Err(unsupported("get_type_name", refno))
    }

    /// CATR / SPRE / PRTREF 链 1–3 跳，收口 SCOM / SPRF / SFIT / JOIN。见契约 2。
    async fn get_cat_refno(&self, refno: RefnoEnum) -> anyhow::Result<Option<RefnoEnum>> {
        Err(unsupported("get_cat_refno", refno))
    }

    /// 见契约 1：原序。
    async fn get_children_pes(&self, refno: RefnoEnum) -> anyhow::Result<Vec<SPdmsElement>> {
        Err(unsupported("get_children_pes", refno))
    }

    /// 见契约 1：原序。
    async fn get_children_refnos(&self, refno: RefnoEnum) -> anyhow::Result<Vec<RefnoEnum>> {
        Err(unsupported("get_children_refnos", refno))
    }

    /// 见契约 1：原序。
    async fn get_children_named_attmaps(
        &self,
        refno: RefnoEnum,
    ) -> anyhow::Result<Vec<NamedAttrMap>> {
        Err(unsupported("get_children_named_attmaps", refno))
    }

    async fn get_ancestor_types(&self, refno: RefnoEnum) -> anyhow::Result<Vec<String>> {
        Err(unsupported("get_ancestor_types", refno))
    }

    /// `get_world_transform` 的底座之一。
    async fn query_ancestor_refnos(&self, refno: RefnoEnum) -> anyhow::Result<Vec<RefnoEnum>> {
        Err(unsupported("query_ancestor_refnos", refno))
    }

    /// `get_world_transform` 的底座之二：祖先链 POS/ORI 折叠的原料。
    async fn get_ancestor_attmaps(&self, refno: RefnoEnum) -> anyhow::Result<Vec<NamedAttrMap>> {
        Err(unsupported("get_ancestor_attmaps", refno))
    }

    /// `get_world_transform` 的底座之三。
    async fn get_spline_pts(&self, refno: RefnoEnum) -> anyhow::Result<Vec<DVec3>> {
        Err(unsupported("get_spline_pts", refno))
    }

    /// 见契约 1：过滤不改变原序。
    async fn query_filter_children(
        &self,
        refno: RefnoEnum,
        nouns: &[&str],
    ) -> anyhow::Result<Vec<RefnoEnum>> {
        let _ = nouns;
        Err(unsupported("query_filter_children", refno))
    }

    /// 见契约 1：过滤不改变原序。
    async fn query_filter_children_atts(
        &self,
        refno: RefnoEnum,
        nouns: &[&str],
    ) -> anyhow::Result<Vec<NamedAttrMap>> {
        let _ = nouns;
        Err(unsupported("query_filter_children_atts", refno))
    }

    async fn query_filter_ancestors(
        &self,
        refno: RefnoEnum,
        nouns: &[&str],
    ) -> anyhow::Result<Vec<RefnoEnum>> {
        let _ = nouns;
        Err(unsupported("query_filter_ancestors", refno))
    }

    /// 见契约 1：深度优先，每层按成员原序。
    async fn query_deep_children_refnos(&self, refno: RefnoEnum) -> anyhow::Result<Vec<RefnoEnum>> {
        Err(unsupported("query_deep_children_refnos", refno))
    }

    /// 见契约 1：深度优先，每层按成员原序。
    async fn query_filter_deep_children(
        &self,
        refno: RefnoEnum,
        nouns: &[&str],
    ) -> anyhow::Result<Vec<RefnoEnum>> {
        let _ = nouns;
        Err(unsupported("query_filter_deep_children", refno))
    }

    /// `query_deep_children_refnos_filter_spre` 的**源模型那一半**：深层 children 里
    /// `SPRE` 或 `CATR` 非空的那些。
    ///
    /// 收口函数还有一个 `filter: bool` 参数，问的是「其中哪些还没有 `inst_relate` /
    /// `tubi_relate`」—— 那是产物，只在 Surreal 里，所以**不在这个签名上**，
    /// 由收口函数在 provider 返回之后自己加。同 [`deep_versioned_children_by_noun`]。
    ///
    /// [`deep_versioned_children_by_noun`]: DirectReadProvider::deep_versioned_children_by_noun
    async fn query_deep_children_refnos_filter_spre(
        &self,
        refno: RefnoEnum,
    ) -> anyhow::Result<Vec<RefnoEnum>> {
        Err(unsupported("query_deep_children_refnos_filter_spre", refno))
    }

    async fn query_single_by_paths(
        &self,
        refno: RefnoEnum,
        paths: &[&str],
        fields: &[&str],
    ) -> anyhow::Result<NamedAttrMap> {
        let _ = (paths, fields);
        Err(unsupported("query_single_by_paths", refno))
    }

    async fn fetch_loops_and_height(
        &self,
        refno: RefnoEnum,
    ) -> anyhow::Result<(Vec<Vec<Vec3>>, f32)> {
        Err(unsupported("fetch_loops_and_height", refno))
    }

    /// `query_multi_deep_versioned_children_filter_inst` 的**源模型那一半**。
    ///
    /// 那个收口函数是混合读：SQL 里同时问「深层 children 里哪些是这些 noun」（源模型）
    /// 和「其中哪些还没有 `->inst_relate` / `->tubi_relate`」（产物）。产物只在 Surreal 里，
    /// 文件侧读不到，所以 provider 只交源模型那一半，产物过滤留在收口函数里。
    ///
    /// **整体交给 provider 会让「已生成过」判定失效**，表现为重复生成或漏生成，
    /// 而两模式产物 hash 仍然一致 —— 对拍看不出来。规格 §2.3 记的就是这条。
    async fn deep_versioned_children_by_noun(
        &self,
        refnos: &[RefnoEnum],
        nouns: &[&str],
    ) -> anyhow::Result<BTreeSet<RefnoEnum>> {
        let _ = nouns;
        Err(unsupported_batch("deep_versioned_children_by_noun", refnos))
    }

    /// `query_group_by_cata_hash` 的**源模型那一半**：每个 refno 的 cata_hash。
    ///
    /// 同 [`deep_versioned_children_by_noun`]，产物侧（按 hash 复用已生成几何）
    /// 留在收口函数里。
    ///
    /// 参数刻意是 `&[RefnoEnum]` 而不是收口函数现在的
    /// `impl IntoIterator<Item = &RefnoEnum>`：`impl Trait` 参数不能进 dyn-safe 的 trait，
    /// 收口函数先 collect 再交过来。
    ///
    /// [`deep_versioned_children_by_noun`]: DirectReadProvider::deep_versioned_children_by_noun
    async fn cata_hash_of(&self, refnos: &[RefnoEnum]) -> anyhow::Result<Vec<(RefnoEnum, String)>> {
        Err(unsupported_batch("cata_hash_of", refnos))
    }
}

/// direct 上下文内一个 provider 没实现的查询。
///
/// 是错误而不是 `None`，也不是回落：调用方分不清「文件里没有这个元素」和
/// 「这条查询还没接」，而这两件事一个是数据、一个是缺口。
pub fn unsupported(query: &str, refno: RefnoEnum) -> anyhow::Error {
    anyhow::anyhow!(
        "direct read is not implemented for {query}({refno}); \
         the direct provider must answer it or the generation must not run in a direct context \
         — falling back to SurrealDB here would make the db/direct comparison agree by \
         reading the same database twice"
    )
}

/// [`unsupported`] 的批量版。
pub fn unsupported_batch(query: &str, refnos: &[RefnoEnum]) -> anyhow::Error {
    anyhow::anyhow!(
        "direct read is not implemented for {query} over {} refnos (first: {}); \
         the direct provider must answer it or the generation must not run in a direct context",
        refnos.len(),
        refnos
            .first()
            .map_or_else(|| "none".to_string(), |refno| refno.to_string())
    )
}

tokio::task_local! {
    static DIRECT_READS: DirectReadContext;
}

/// 生成期执行上下文：一个 provider + 供日志与断言用的名字。
#[derive(Clone)]
pub struct DirectReadContext {
    provider: Arc<dyn DirectReadProvider>,
}

impl DirectReadContext {
    pub fn new(provider: Arc<dyn DirectReadProvider>) -> Self {
        Self { provider }
    }

    pub fn provider(&self) -> &dyn DirectReadProvider {
        self.provider.as_ref()
    }

    pub fn label(&self) -> &str {
        self.provider.label()
    }
}

impl std::fmt::Debug for DirectReadContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirectReadContext")
            .field("label", &self.label())
            .finish()
    }
}

/// 在 direct 读上下文中运行一个 future：作用域内（含 `.await` 链上的全部调用）
/// 被接线的读入口改问 provider。作用域退出即还原；嵌套时以最内层为准。
///
/// # 错误
///
/// 暂存读上下文已经在场时拒绝进入（ADR-053 R6）。生成期不在暂存窗口内，
/// 两者同时在场没有合法场景；真同时在场了，哪个赢将取决于每个收口函数里两个
/// `if let` 的书写顺序 —— 那不是语义，那是巧合。
pub async fn with_direct_reads<F>(ctx: DirectReadContext, fut: F) -> anyhow::Result<F::Output>
where
    F: std::future::Future,
{
    if let Some(staging) = super::staging::active_staging_reads() {
        anyhow::bail!(
            "cannot enter direct reads ({}) inside the staging read context ({}): \
             one answers from the E3D files and the other from the staging database, and which \
             one wins would be decided by the order two `if let`s happen to be written in",
            ctx.label(),
            staging.label()
        );
    }
    Ok(DIRECT_READS.scope(ctx, fut).await)
}

/// 当前 task 的 direct 读上下文（无则 `None`）。
///
/// 被接线的读入口以它分流。**上下文在场时结果不得写进进程级 `#[cached]` 缓存** ——
/// 那些缓存的键里没有「世界」这一维，混写会把文件侧的值泄漏给查库的读者，反之亦然。
/// 这条与 staging 同源，理由一模一样。
pub fn active_direct_reads() -> Option<DirectReadContext> {
    DIRECT_READS.try_with(|ctx| ctx.clone()).ok()
}

/// `tokio::spawn` 不继承 task-local；生成链的子任务统一从这里派生，避免上下文在并行
/// 边界丢失后**静默回到 SurrealDB** —— 那是本模块唯一一种不报错就能发生的回落。
pub fn spawn_with_direct_reads<F>(future: F) -> tokio::task::JoinHandle<F::Output>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    let context = active_direct_reads();
    tokio::spawn(async move {
        match context {
            Some(context) => DIRECT_READS.scope(context, future).await,
            None => future.await,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Stub;

    #[async_trait::async_trait]
    impl DirectReadProvider for Stub {
        fn label(&self) -> &str {
            "stub"
        }

        async fn get_type_name(&self, _refno: RefnoEnum) -> anyhow::Result<String> {
            Ok("BRAN".into())
        }
    }

    fn ctx() -> DirectReadContext {
        DirectReadContext::new(Arc::new(Stub))
    }

    #[tokio::test]
    async fn the_context_is_visible_inside_its_scope_and_gone_outside() {
        assert!(active_direct_reads().is_none());
        let seen = with_direct_reads(ctx(), async { active_direct_reads().is_some() })
            .await
            .unwrap();
        assert!(seen);
        assert!(active_direct_reads().is_none());
    }

    /// **把默认实现改成回落 SurrealDB 就会红。** 一个 provider 没接的查询必须是错误，
    /// 不是一个看起来正常的答案。
    #[tokio::test]
    async fn a_query_the_provider_does_not_implement_is_an_error() {
        let refno = RefnoEnum::from("4000000001/20");
        let implemented =
            with_direct_reads(
                ctx(),
                async move { ctx().provider().get_type_name(refno).await },
            )
            .await
            .unwrap();
        assert_eq!(implemented.unwrap(), "BRAN");

        let missing = with_direct_reads(ctx(), async move {
            ctx().provider().get_named_attmap(refno).await
        })
        .await
        .unwrap();
        let error = missing.unwrap_err().to_string();
        assert!(
            error.contains("get_named_attmap") && error.contains("not implemented"),
            "{error}"
        );
    }

    /// 每个 D 档收口函数都真的接了 direct 分支 —— 源码顺序断言，因为「忘了接线」
    /// 编译不出错、跑起来也不报错，只是**静默回到 SurrealDB**，而那正是
    /// [`with_direct_reads`] 的文档里说的、唯一一种不报错就能发生的回落。
    ///
    /// 清单与 `gen-model/docs/specs/direct-mode-query-surface.md` §2 的 D 档同步；
    /// 那张表加一行，这里就要加一行。
    #[test]
    fn every_routed_collector_asks_the_provider_first() {
        const ROUTED: &[(&str, &str)] = &[
            ("query.rs", "pub async fn get_named_attmap("),
            ("query.rs", "pub async fn get_type_name("),
            ("query.rs", "pub async fn get_cat_refno("),
            ("query.rs", "pub async fn get_children_pes("),
            ("query.rs", "pub async fn get_children_refnos("),
            ("query.rs", "pub async fn get_children_named_attmaps("),
            ("query.rs", "pub async fn get_ancestor_types("),
            ("query.rs", "pub async fn get_ancestor_attmaps("),
            ("query.rs", "pub async fn query_ancestor_refnos("),
            ("query.rs", "pub async fn query_filter_children("),
            ("query.rs", "pub async fn query_filter_children_atts("),
            ("query.rs", "pub async fn query_single_by_paths("),
            ("query.rs", "pub async fn query_group_by_cata_hash("),
            ("graph.rs", "pub async fn query_deep_children_refnos("),
            ("graph.rs", "pub async fn query_filter_deep_children("),
            ("graph.rs", "pub async fn query_filter_deep_children_atts("),
            (
                "graph.rs",
                "pub async fn query_deep_children_refnos_filter_spre(",
            ),
            (
                "graph.rs",
                "pub async fn query_multi_deep_versioned_children_filter_inst(",
            ),
            ("geom.rs", "pub async fn fetch_loops_and_height("),
            ("spatial.rs", "pub async fn get_spline_pts("),
            ("spatial.rs", "pub async fn get_world_transform("),
        ];

        for (file, signature) in ROUTED {
            let source = std::fs::read_to_string(format!("src/rs_surreal/{file}"))
                .unwrap_or_else(|error| panic!("read src/rs_surreal/{file}: {error}"));
            let at = source
                .find(signature)
                .unwrap_or_else(|| panic!("{file} no longer declares `{signature}`"));
            let body = &source[at..];
            let direct = body.find("active_direct_reads()").unwrap_or(usize::MAX);
            assert!(
                direct != usize::MAX,
                "{file}: `{signature}` has no direct branch, so a generation running in a \
                 direct context reads SurrealDB instead — silently"
            );
            // 两个上下文都在场时谁赢，由这个顺序决定。定成 direct 优先并钉住，
            // 免得它退回成「取决于两个 if let 恰好谁写在前面」。
            if let Some(staging) = body.find("active_staging_reads()") {
                assert!(
                    direct < staging,
                    "{file}: `{signature}` checks staging before direct; direct must win so the \
                     precedence is a decision rather than a coincidence of line order"
                );
            }
        }
    }

    /// ADR-053 R6.
    #[tokio::test]
    async fn direct_reads_refuse_to_nest_inside_staging_reads() {
        let db = surrealdb::engine::any::connect("mem://").await.unwrap();
        db.use_ns("test").use_db("r6").await.unwrap();
        let staging = super::super::staging::StagingReadContext::new(db, "staging_r6");

        let refused = super::super::staging::with_staging_reads(staging, async {
            with_direct_reads(ctx(), async {}).await
        })
        .await;

        let error = refused.unwrap_err().to_string();
        assert!(error.contains("staging read context"), "{error}");
    }
}
