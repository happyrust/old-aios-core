use std::io::Read;
use std::path::PathBuf;
use crate::{NamedAttrMap, RefU64, SurlValue, SUL_DB};
use cached::proc_macro::cached;
use surrealdb::engine::any::Any;
use surrealdb::Surreal;

pub async fn define_common_functions() -> anyhow::Result<()> {
    define_common_functions_on(&SUL_DB).await
}

/// 与 [`define_common_functions`] 同一套脚本（CWD 下 `resource/surreal/*`，按目录
/// 顺序执行），但落在显式给定的句柄上。暂存库初始化与 mem↔fork 一致性套件
/// （ADR-017）都要在非 `SUL_DB` 的库上重放同一组 fn:: 定义，脚本来源必须唯一。
pub async fn define_common_functions_on(db: &Surreal<Any>) -> anyhow::Result<()> {
    let target_dir = std::fs::read_dir("resource/surreal")?.into_iter()
        .map(|entry| {
            let entry = entry.unwrap();
            entry.path()
        }).collect::<Vec<PathBuf>>();
    for file in target_dir {
        println!("载入surreal {}",file.file_name().unwrap().to_str().unwrap().to_string());
        let mut file = std::fs::File::open(file)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        db.query(content).await?;
    }
    Ok(())
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
