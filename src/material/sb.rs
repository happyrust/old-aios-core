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
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use surrealdb::sql::Thing;
use tokio::task::{self, JoinHandle};

lazy_static::lazy_static! {
    static ref CHINESE_FIELDS: HashMap<&'static str, &'static str> = {
        let mut map = HashMap::new();
        map.insert("id", "参考号");
        map.insert("name", "设备位号");
        map.insert("room_code", "所在房间");
        map.insert("length", "轨道长度");
        map.insert("pos", "安装标高");
        map
    };
}

const FIELDS: [&str; 5] = ["参考号", "设备位号", "所在房间", "轨道长度", "安装标高"];

const DATA_FIELDS: [&str; 5] = ["id", "name", "room_code", "length", "pos"];

const TABLE: &'static str = "设备专业_大宗材料";

/// 设备专业 大宗材料
pub async fn save_sb_material_dzcl(refno: RefU64) -> Vec<JoinHandle<()>> {
    let db = SUL_DB.clone();
    let mut handles = Vec::new();
    match get_sb_dzcl_list_material(db.clone(), vec![refno]).await {
        Ok(r) => {
            if r.is_empty() {
                return handles;
            }
            let r_clone = r.clone();
            let task = task::spawn(async move {
                match insert_into_table_with_chunks(&db, "material_sb_list", r_clone).await {
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
                    match create_table_sql(&pool, TABLE, &FIELDS).await {
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

/// 设备专业 通信系统
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialTxTxsbData {
    pub id: RefU64,
    pub equi_name: String,
    pub ptre_desc: String,
    pub belong_factory: String,
    pub room_code: Option<String>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub z: Option<f32>,
    pub ptre_name: String,
    #[serde(default)]
    pub version_tag: String,
}

impl MaterialTxTxsbData {
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("设备位号".to_string()).or_insert(self.equi_name);
        map.entry("设备名称".to_string()).or_insert(self.ptre_desc);
        map.entry("所属厂房的编号".to_string())
            .or_insert(self.belong_factory);
        map.entry("房间号".to_string())
            .or_insert(self.room_code.unwrap_or("".to_string()));
        map.entry("全局坐标X".to_string())
            .or_insert(self.x.unwrap_or(0.0).to_string());
        map.entry("全局坐标Y".to_string())
            .or_insert(self.y.unwrap_or(0.0).to_string());
        map.entry("全局坐标Z".to_string())
            .or_insert(self.z.unwrap_or(0.0).to_string());
        map.entry("设备型号".to_string()).or_insert(self.ptre_name);

        map
    }
}

/// 设备专业 大宗材料
pub async fn get_sb_dzcl_list_material(
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
            if !pe.name.contains("EQUI") {
                continue;
            };
        }
        // 查询 EQUI 的数据
        let refnos = query_filter_deep_children(refno.into(), &["EQUI"]).await?;
        let refnos_str = &refnos
            .into_iter()
            .map(|refno| refno.to_pe_key())
            .collect::<Vec<String>>()
            .join(",");
        let sql = format!(r#"return fn::eq_dz([{}])"#, refnos_str);
        let mut response = db.query(sql).await?;
        match response.take::<Vec<HashMap<String, Value>>>(0) {
            Ok(mut result) => {
                data.append(&mut result);
            }
            Err(e) => {
                dbg!(&e.to_string());
            }
        }
    }
    Ok(data)
}

fn filter_equi_children(datas: Vec<Vec<Vec<Thing>>>) -> Vec<Vec<String>> {
    let mut result = Vec::new();
    for data in datas {
        let filtered_data: Vec<Vec<String>> = data
            .into_iter()
            .filter(|inner_vec| inner_vec.iter().all(|s| s.tb == "BOX"))
            .filter(|inner_vec| {
                let count = inner_vec.iter().count();
                count == 3 || count == 4
            })
            .map(|vec| {
                vec.iter()
                    .map(|thing| format!("BOX:{}", thing.id.to_string()))
                    .collect::<Vec<String>>()
            })
            .collect();
        if !filtered_data.is_empty() {
            result.push(filtered_data[0].clone())
        }
    }
    result
}

/// 设备专业 大宗材料
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialSbListData {
    pub id: RefU64,
    pub name: String,
    pub pos: Option<f32>,
    pub length: Option<f32>,
    pub room_code: Option<String>,
    pub boxs: Vec<Vec<Thing>>,
    #[serde(default)]
    pub version_tag: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialSubeData {
    pub refno: RefU64,
    pub pos: Option<f32>,
    pub length: Option<f32>,
}

impl MaterialSbListData {
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("设备位号".to_string()).or_insert(self.name);
        map.entry("所在房间".to_string())
            .or_insert(self.room_code.unwrap_or("".to_string()));
        map.entry("轨道长度".to_string())
            .or_insert(self.length.unwrap_or(0.0).to_string());
        map.entry("安装标高".to_string())
            .or_insert(self.pos.unwrap_or(0.0).to_string());
        map
    }
}

#[tokio::test]
async fn test_save_sb_material_equi() {
    init_test_surreal().await;
    let mut handles = Vec::new();
    let refno = RefU64::from("24384/24828");
    handles.append(&mut save_sb_material_dzcl(refno).await);
    futures::future::join_all(handles).await;
}
