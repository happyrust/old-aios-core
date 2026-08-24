#[cfg(feature = "sql")]
use super::query::create_table_sql;
#[cfg(feature = "sql")]
use super::query::save_material_value;
#[cfg(feature = "sql")]
use super::query::save_material_value_test;

use crate::SUL_DB;
use crate::aios_db_mgr::aios_mgr::AiosDBMgr;
use crate::init_test_surreal;
use crate::material::sb::MaterialTxTxsbData;
use crate::{
    RefU64, get_children_pes, get_pe, insert_into_table_with_chunks, query_filter_deep_children,
};
use anyhow::anyhow;
use serde_json::Value;
use std::collections::HashMap;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use tokio::task::{self, JoinHandle};

lazy_static::lazy_static! {
    static ref CHINESE_FIELDS: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("id", "参考号");
        m.insert("equi_name", "设备位号");
        m.insert("ptre_desc", "设备名称");
        m.insert("belong_factory", "所属厂房的编号");
        m.insert("room_code", "房间号");
        m.insert("x", "全局坐标X");
        m.insert("y", "全局坐标Y");
        m.insert("z", "全局坐标Z");
        m.insert("ptre_name", "设备型号");
        m
    };
}

const FIELDS: [&str; 9] = [
    "参考号",
    "设备位号",
    "设备名称",
    "所属厂房的编号",
    "房间号",
    "全局坐标X",
    "全局坐标Y",
    "全局坐标Z",
    "设备型号",
];

const DATA_FIELDS: [&str; 9] = [
    "id",
    "equi_name",
    "ptre_desc",
    "belong_factory",
    "room_code",
    "x",
    "y",
    "z",
    "ptre_name",
];

const TABLE: &'static str = "通信专业_通信系统";

/// 通信专业 通信设备
pub async fn save_tx_material_equi(refno: RefU64) -> Vec<JoinHandle<()>> {
    let db = SUL_DB.clone();
    let mut handles = Vec::new();
    match get_tx_txsb_list_material(db.clone(), vec![refno]).await {
        Ok(r) => {
            if r.is_empty() {
                return handles;
            }
            let r_clone = r.clone();
            let task = task::spawn(async move {
                match insert_into_table_with_chunks(&db, "material_tx_list", r_clone).await {
                    Ok(_) => {}
                    Err(e) => {
                        dbg!(&e.to_string());
                    }
                }
            });
            handles.push(task);
            #[cfg(feature = "sql")]
            {
                let Ok(pool) = AiosDBMgr::get_project_pool().await else {
                    dbg!("无法连接到数据库");
                    return handles;
                };
                let task = task::spawn(async move {
                    match create_table_sql(&pool, &TABLE, &FIELDS).await {
                        Ok(_) => {
                            if !r.is_empty() {
                                match save_material_value_test(
                                    &pool,
                                    &TABLE,
                                    &DATA_FIELDS,
                                    &CHINESE_FIELDS,
                                    r,
                                )
                                .await
                                {
                                    Ok(_) => {}
                                    Err(e) => {
                                        dbg!(&e.to_string());
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            dbg!(&e.to_string());
                        }
                    }
                });
                handles.push(task);
            }
        }
        Err(e) => {
            dbg!(e.to_string());
        }
    }
    handles
}

/// 通信专业 通信设备
pub async fn get_tx_txsb_list_material(
    db: Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<HashMap<String, Value>>> {
    let mut data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno.into()).await? else {
            continue;
        };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("E") {
                continue;
            };
        }
        // 查询 EQUI 的数据
        let refnos = query_filter_deep_children(refno.into(), &["ELCONN"]).await?;
        let refnos_str = &refnos
            .into_iter()
            .map(|refno| refno.to_pe_key())
            .collect::<Vec<String>>()
            .join(",");
        let sql = format!(r#"return fn::tx_sb([{}])"#, refnos_str);
        let mut response = db.query(&sql).await?;
        match response.take::<Vec<HashMap<String, Value>>>(0) {
            Ok(mut result) => {
                data.append(&mut result);
            }
            Err(e) => {
                dbg!(e.to_string());
                return Err(anyhow!(sql));
            }
        }
    }
    Ok(data)
}

#[tokio::test]
async fn test_save_tx_material_equi() {
    init_test_surreal().await;
    let mut handles = Vec::new();
    let refno = RefU64::from("pe:24384/24828");
    handles.append(&mut save_tx_material_equi(refno).await);
    futures::future::join_all(handles).await;
}
