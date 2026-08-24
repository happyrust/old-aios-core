#[cfg(feature = "sql")]
use super::query::create_table_sql;
#[cfg(feature = "sql")]
use super::query::save_material_value;
#[cfg(feature = "sql")]
use super::query::save_material_value_test;

use crate::SUL_DB;
use crate::aios_db_mgr::aios_mgr::AiosDBMgr;
use crate::init_test_surreal;
use crate::{RefU64, get_pe, insert_into_table_with_chunks, query_filter_deep_children};
use anyhow::anyhow;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use tokio::task::{self, JoinHandle};

lazy_static::lazy_static! {
    static ref CHINESE_FIELDS: HashMap<&'static str, &'static str> = {
        let mut map = HashMap::new();
        map.insert("id", "参考号");
        map.insert("name", "阀门位号");
        map.insert("room_code", "所在房间号");
        map.insert("bran_name", "阀门归属");
        map.insert("valv_size", "阀门尺寸");
        map.insert("material", "阀门材质");
        map.insert("valv_use", "阀门功能");
        map
    };
}

const FIELDS: [&str; 7] = [
    "参考号",
    "阀门位号",
    "所在房间号",
    "阀门归属",
    "阀门尺寸",
    "阀门材质",
    "阀门功能",
];

const TABLE: &'static str = "暖通专业_阀门清单";

const DATA_FIELDS: [&str; 7] = [
    "id",
    "name",
    "room_code",
    "bran_name",
    "valv_size",
    "material",
    "valv_use",
];

/// 暖通专业 大宗材料
pub async fn save_nt_material_dzcl(refno: RefU64) -> Vec<JoinHandle<()>> {
    let db = SUL_DB.clone();
    let mut handles = Vec::new();
    match get_nt_valv_list_material(db.clone(), vec![refno]).await {
        Ok(r) => {
            if r.is_empty() {
                return handles;
            }
            let r_clone = r.clone();
            let task = task::spawn(async move {
                match insert_into_table_with_chunks(&db, "material_nt_valv", r_clone).await {
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

/// 暖通 阀门清单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialNtValvData {
    pub id: RefU64,
    pub name: String,
    pub room_code: Option<String>,
    pub bran_name: String,
    pub valv_size: Vec<f32>,
    pub material: String,
    pub valv_use: String,
    #[serde(default)]
    pub version_tag: String,
}

impl MaterialNtValvData {
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("阀门位号".to_string()).or_insert(self.name);
        map.entry("所在房间号".to_string())
            .or_insert(self.room_code.unwrap_or("".to_string()));
        map.entry("阀门归属".to_string()).or_insert(self.bran_name);
        map.entry("阀门尺寸".to_string())
            .or_insert(serde_json::to_string(&self.valv_size).unwrap_or("[]".to_string()));
        map.entry("阀门材质".to_string()).or_insert(self.material);
        map.entry("阀门功能".to_string()).or_insert(self.valv_use);
        map
    }
}

/// 暖通 阀门清单
pub async fn get_nt_valv_list_material(
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
            if !pe.name.contains("HVAC") {
                continue;
            };
        }
        // 查询 DAMP 的数据
        let refnos = query_filter_deep_children(refno.into(), &["DAMP"]).await?;
        let refnos_str = &refnos
            .into_iter()
            .map(|refno| refno.to_pe_key())
            .collect::<Vec<String>>()
            .join(",");
        let sql = format!(r#"return fn::nt_valv([{}])"#, refnos_str);
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
async fn test_save_nt_material_dzcl() {
    init_test_surreal().await;
    let mut handles = Vec::new();
    let refno = RefU64::from("24381/57021");
    handles.append(&mut save_nt_material_dzcl(refno).await);
    futures::future::join_all(handles).await;
}
