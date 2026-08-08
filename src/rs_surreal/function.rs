use crate::{NamedAttrMap, RefU64, SurlValue, SUL_DB};
use anyhow::Context;
use cached::proc_macro::cached;
use std::io::Read;
use std::path::PathBuf;
use surrealdb::Connection;
use surrealdb::engine::any::Any;
use surrealdb::Surreal;

const INST_META_COMPAT_SQL: &str = include_str!("inst_meta_compat.surql");
const ANC_U64_DEFINE: &str = "DEFINE FUNCTION OVERWRITE fn::anc_u64";

pub async fn define_common_functions() -> anyhow::Result<()> {
    define_common_functions_on(&SUL_DB).await
}

/// 与 [`define_common_functions`] 同一套脚本（CWD 下 `resource/surreal/*`，按目录
/// 顺序执行），但落在显式给定的句柄上。暂存库初始化与 mem↔fork 一致性套件
/// （ADR-017）都要在非 `SUL_DB` 的库上重放同一组 fn:: 定义，脚本来源必须唯一。
pub async fn define_common_functions_on(db: &Surreal<Any>) -> anyhow::Result<()> {
    let mut target_dir = std::fs::read_dir("resource/surreal")?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<PathBuf>, _>>()?;
    target_dir.sort();
    for file in target_dir {
        println!("载入surreal {}",file.file_name().unwrap().to_str().unwrap().to_string());
        let mut file = std::fs::File::open(file)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        db.query(content).await?;
    }
    if ensure_inst_meta_functions_on(db)
        .await
        .context("公共函数加载阶段：当前 SurrealDB database 的 inst_meta 兼容函数未就绪")?
    {
        println!("已从二进制内置定义补装缺失的 inst_meta 兼容函数");
    }
    Ok(())
}

fn inst_meta_definitions() -> (&'static str, &'static str) {
    let anc_at = INST_META_COMPAT_SQL
        .find(ANC_U64_DEFINE)
        .expect("内置 inst_meta_compat.surql 必须包含 fn::anc_u64");
    (&INST_META_COMPAT_SQL[..anc_at], &INST_META_COMPAT_SQL[anc_at..])
}

async fn function_exists<C: Connection>(
    db: &Surreal<C>,
    name: &str,
    probe: &str,
) -> anyhow::Result<bool> {
    let response = db
        .query(probe)
        .await
        .with_context(|| format!("探测当前 SurrealDB 的 {name} 失败"))?;
    match response.check() {
        Ok(_) => Ok(true),
        Err(error) if error.to_string().contains("does not exist") => Ok(false),
        Err(error) => Err(error).with_context(|| format!("执行 {name} 探针失败")),
    }
}

async fn define_checked<C: Connection>(
    db: &Surreal<C>,
    name: &str,
    sql: &str,
) -> anyhow::Result<()> {
    db.query(sql)
        .await
        .with_context(|| format!("向当前 SurrealDB 发送内置 {name} 定义失败"))?
        .check()
        .with_context(|| format!("当前 SurrealDB 拒绝内置 {name} 定义"))?;
    Ok(())
}

/// Ensure the two functions used by the materialized `inst_relate.anc` column.
///
/// Existing definitions are never overwritten. Missing definitions are loaded
/// from SQL compiled into `aios_core`, so an old deployment copy of
/// `resource/surreal/common.surql` cannot make DESI finalization fail later.
pub async fn ensure_inst_meta_functions_on<C: Connection>(
    db: &Surreal<C>,
) -> anyhow::Result<bool> {
    let refno_probe = "RETURN fn::refno_u64(type::thing('pe','1_1'));";
    let anc_probe = "RETURN fn::anc_u64(type::thing('pe','1_1'));";
    let mut installed = false;
    let (refno_define, anc_define) = inst_meta_definitions();

    if !function_exists(db, "fn::refno_u64", refno_probe).await? {
        define_checked(db, "fn::refno_u64", refno_define).await?;
        installed = true;
    }
    if !function_exists(db, "fn::anc_u64", anc_probe).await? {
        define_checked(db, "fn::anc_u64", anc_define).await?;
        installed = true;
    }

    let mut missing = Vec::new();
    if !function_exists(db, "fn::refno_u64", refno_probe).await? {
        missing.push("fn::refno_u64");
    }
    if !function_exists(db, "fn::anc_u64", anc_probe).await? {
        missing.push("fn::anc_u64");
    }
    if !missing.is_empty() {
        anyhow::bail!(
            "SurrealDB 内置兼容函数复检失败：当前 database 缺少 {}",
            missing.join(", ")
        );
    }
    Ok(installed)
}

#[cfg(test)]
mod inst_meta_compat_tests {
    use super::*;
    use surrealdb::engine::local::{Db, Mem};

    async fn mem_db(name: &str) -> Surreal<Db> {
        let db = Surreal::new::<Mem>(()).await.expect("create mem db");
        db.use_ns("inst_meta_compat")
            .use_db(name)
            .await
            .expect("select mem db");
        db
    }

    #[tokio::test]
    async fn missing_inst_meta_functions_are_installed_and_idempotent() {
        let db = mem_db("missing").await;

        assert!(
            ensure_inst_meta_functions_on(&db)
                .await
                .expect("install missing functions")
        );
        assert!(
            !ensure_inst_meta_functions_on(&db)
                .await
                .expect("second ensure is a no-op")
        );

        let mut response = db
            .query(
                "RETURN fn::refno_u64(type::thing('pe','1_2'));\
                 RETURN fn::anc_u64(type::thing('pe','__missing__'));",
            )
            .await
            .expect("probe installed functions")
            .check()
            .expect("installed functions execute");
        assert_eq!(
            response.take::<Option<i64>>(0).expect("refno result"),
            Some(4_294_967_298)
        );
        assert_eq!(
            response.take::<Vec<i64>>(1).expect("ancestor result"),
            Vec::<i64>::new()
        );
    }

    #[tokio::test]
    async fn existing_inst_meta_functions_are_not_overwritten() {
        let db = mem_db("existing").await;
        db.query(
            "DEFINE FUNCTION OVERWRITE fn::refno_u64($r: record) { RETURN 7; };\
             DEFINE FUNCTION OVERWRITE fn::anc_u64($r: record) { RETURN [8]; };",
        )
        .await
        .expect("define custom functions")
        .check()
        .expect("custom definitions execute");

        assert!(
            !ensure_inst_meta_functions_on(&db)
                .await
                .expect("existing functions remain")
        );

        let mut response = db
            .query(
                "RETURN fn::refno_u64(type::thing('pe','1_2'));\
                 RETURN fn::anc_u64(type::thing('pe','1_2'));",
            )
            .await
            .expect("call custom functions")
            .check()
            .expect("custom functions still execute");
        assert_eq!(
            response.take::<Option<i64>>(0).expect("custom refno"),
            Some(7)
        );
        assert_eq!(response.take::<Vec<i64>>(1).expect("custom anc"), vec![8]);
    }

    #[tokio::test]
    async fn checked_define_surfaces_statement_errors() {
        let db = mem_db("invalid_definition").await;
        let error = define_checked(
            &db,
            "fn::broken",
            "THROW 'broken embedded definition';",
        )
        .await
        .expect_err("invalid embedded definition must fail during loading");

        assert!(
            error.to_string().contains("当前 SurrealDB 拒绝内置 fn::broken 定义"),
            "unexpected error: {error:#}"
        );
    }
}

/// 定义数据库编号事件
/// 
/// 当创建新的 pe 记录时,会触发此事件来更新 dbnum_info_table 表中的信息
/// 
/// # 错误
/// 
/// 如果数据库操作失败,将返回错误
pub async fn define_dbnum_event() -> anyhow::Result<()> {
    define_dbnum_event_on(&SUL_DB).await
}

/// [`define_dbnum_event`] 的显式句柄版（ADR-017 暂存库初始化用）。
pub async fn define_dbnum_event_on(db: &Surreal<Any>) -> anyhow::Result<()> {
    db
        .query(r#"
        DEFINE EVENT OVERWRITE update_dbnum_event ON pe WHEN $event = "CREATE" OR $event = "UPDATE" OR $event = "DELETE" THEN {
            -- 获取当前记录的 dbnum
            LET $dbnum = $value.dbnum;
            LET $id = record::id($value.id);
            let $ref_0 = array::at($id, 0);
            let $ref_1 = array::at($id, 1);
            let $is_delete = $value.deleted and $event = "UPDATE";
            let $max_sesno = if $after.sesno > $before.sesno?:0 { $after.sesno } else { $before.sesno };
            -- 根据事件类型处理  type::thing("dbnum_info_table", $ref_0)
            IF $event = "CREATE"   {
                UPSERT type::thing('dbnum_info_table', $ref_0) SET
                    dbnum = $dbnum,
                    count = count?:0 + 1,
                    sesno = $max_sesno,
                    max_ref1 = $ref_1;
            } ELSE IF $event = "DELETE" OR $is_delete  {
                UPSERT type::thing('dbnum_info_table', $ref_0) SET
                    count = count - 1,
                    sesno = $max_sesno,
                    max_ref1 = $ref_1
                WHERE count > 0;
            };
        };
        "#)
        .await?;
    Ok(())
}
