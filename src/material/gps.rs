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
use std::str::FromStr;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use tokio::task::{self, JoinHandle};

lazy_static::lazy_static! {
    static ref DZCL_CHINESE_FIELDS: HashMap<&'static str, &'static str> = {
        let mut map = HashMap::new();
        map.insert("id", "参考号");
        map.insert("code", "物项编码");
        map.insert("type", "品名");
        map.insert("radius", "外径/Φ");
        map.insert("length", "长度");
        map.insert("thick", "厚度");
        map.insert("count", "数量");
        map
    };
}

pub const FIELDS: [&str; 18] = [
    "参考号",
    "物项编码",
    "品名",
    "SCH/LB（主）",
    "SCH/LB（支）",
    "制造形式",
    "连接形式",
    "材料牌号",
    "材料标准",
    "规格标准",
    "RCC_M",
    "质保等级",
    "公称直径（主）",
    "公称直径（支）",
    "外径/Φ",
    "长度",
    "厚度",
    "数量",
];

const DZCL_DATA_FIELDS: [&str; 7] = ["id", "code", "type", "radius", "length", "thick", "count"];

const TABLE: &'static str = "给排水专业_大宗材料";

/// 给排水专业 大宗材料
pub async fn save_gps_material_dzcl(refno: RefU64) -> Vec<JoinHandle<()>> {
    let db = SUL_DB.clone();
    let mut handles = Vec::new();
    match get_gps_dzcl_material(db.clone(), vec![refno]).await {
        Ok((r, tubi_r)) => {
            if r.is_empty() {
                return handles;
            }
            let r_clone = r.clone();
            let tubi_r_clone = tubi_r.clone();
            let task = task::spawn(async move {
                match insert_into_table_with_chunks(&db, "material_gps_list", r_clone).await {
                    Ok(_) => {}
                    Err(e) => {
                        dbg!(&e.to_string());
                    }
                }
                match insert_into_table_with_chunks(&db, "material_gps_list_tubi", tubi_r_clone)
                    .await
                {
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
                                    &DZCL_DATA_FIELDS,
                                    &DZCL_CHINESE_FIELDS,
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
                            if !tubi_r.is_empty() {
                                match save_material_value_test(
                                    &pool,
                                    &TABLE,
                                    &DZCL_DATA_FIELDS,
                                    &DZCL_CHINESE_FIELDS,
                                    tubi_r,
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

/// 给排水 大宗材料
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialGpsDzclData {
    pub id: RefU64,
    pub code: String,
    pub noun: String,
    pub radius: Option<String>,
    pub length: Option<f32>,
    pub thick: Option<f32>,
    pub count: Option<f32>,
    #[serde(default)]
    pub version_tag: String,
}

impl MaterialGpsDzclData {
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("物项编码".to_string()).or_insert(self.code);
        map.entry("品名".to_string()).or_insert(self.noun);
        map.entry("外径/Φ".to_string())
            .or_insert(self.radius.unwrap_or("0.0".to_string()));
        map.entry("长度".to_string())
            .or_insert(self.count.unwrap_or(0.0).to_string());
        map.entry("厚度".to_string())
            .or_insert(self.thick.unwrap_or(0.0).to_string());
        map.entry("数量".to_string())
            .or_insert(self.count.unwrap_or(0.0).to_string());

        map
    }
}

/// 给排水 大宗材料
pub async fn get_gps_dzcl_material(
    db: Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<(Vec<HashMap<String, Value>>, Vec<HashMap<String, Value>>)> {
    let mut data = Vec::new();
    let mut tubi_data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno.into()).await? else {
            continue;
        };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("PIPE") {
                continue;
            };
        }
        // 查询 BEND 的数据
        let refnos = query_filter_deep_children(refno.into(), &["BEND"]).await?;
        if !refnos.is_empty() {
            let refnos_str = &refnos
                .into_iter()
                .map(|refno| refno.to_pe_key())
                .collect::<Vec<String>>()
                .join(",");
            let sql = format!(r#"return fn::gps_bend([{}]);"#, refnos_str);
            let mut response = db.query(&sql).await?;
            match response.take::<Vec<HashMap<String, Value>>>(0) {
                Ok(mut result) => {
                    data.append(&mut result);
                }
                Err(e) => {
                    dbg!(&sql);
                    dbg!(&e.to_string());
                }
            }
        }
        // 查询tubi的数据
        let refnos = query_filter_deep_children(refno.into(), &["BRAN"]).await?;
        if !refnos.is_empty() {
            let refnos_str = &refnos
                .into_iter()
                .map(|refno| refno.to_pe_key())
                .collect::<Vec<String>>()
                .join(",");
            let sql = format!(r#"return fn::gps_tubi([{}])"#, refnos_str);
            let mut response = db.query(&sql).await?;
            match response.take::<Vec<HashMap<String, Value>>>(0) {
                Ok(mut result) => {
                    tubi_data.append(&mut result);
                }
                Err(e) => {
                    dbg!(&sql);
                    dbg!(&e.to_string());
                }
            }
        }
        // 查询elbo的数据
        let refnos = query_filter_deep_children(refno.into(), &["ELBO"]).await?;
        if !refnos.is_empty() {
            let refnos_str = &refnos
                .into_iter()
                .map(|refno| refno.to_pe_key())
                .collect::<Vec<String>>()
                .join(",");
            let sql = format!(r#"return fn::gps_elbo([{}])"#, refnos_str);
            let mut response = db.query(&sql).await?;
            match response.take::<Vec<HashMap<String, Value>>>(0) {
                Ok(mut result) => {
                    data.append(&mut result);
                }
                Err(e) => {
                    dbg!(&sql);
                    dbg!(&e.to_string());
                }
            }
        }
        // 查询flan的数据
        let refnos = query_filter_deep_children(refno.into(), &["FLAN"]).await?;
        if !refnos.is_empty() {
            let refnos_str = &refnos
                .into_iter()
                .map(|refno| refno.to_pe_key())
                .collect::<Vec<String>>()
                .join(",");
            let sql = format!(r#"return fn::gps_flan([{}])"#, refnos_str);
            let mut response = db.query(&sql).await?;
            match response.take::<Vec<HashMap<String, Value>>>(0) {
                Ok(mut result) => {
                    data.append(&mut result);
                }
                Err(e) => {
                    dbg!(&sql);
                    dbg!(&e.to_string());
                }
            }
        }
        // 查询redu的数据
        let refnos = query_filter_deep_children(refno.into(), &["REDU"]).await?;
        if !refnos.is_empty() {
            let refnos_str = &refnos
                .into_iter()
                .map(|refno| refno.to_pe_key())
                .collect::<Vec<String>>()
                .join(",");
            let sql = format!(r#"return fn::gps_redu([{}])"#, refnos_str);
            let mut response = db.query(&sql).await?;
            match response.take::<Vec<HashMap<String, Value>>>(0) {
                Ok(mut result) => {
                    data.append(&mut result);
                }
                Err(e) => {
                    dbg!(&sql);
                    dbg!(&e.to_string());
                }
            }
        }
        // 查询tee的数据
        let refnos = query_filter_deep_children(refno.into(), &["TEE"]).await?;
        if !refnos.is_empty() {
            let refnos_str = &refnos
                .into_iter()
                .map(|refno| refno.to_pe_key())
                .collect::<Vec<String>>()
                .join(",");
            let sql = format!(r#"return fn::gps_tee([{}])"#, refnos_str);
            let mut response = db.query(&sql).await?;
            match response.take::<Vec<HashMap<String, Value>>>(0) {
                Ok(mut result) => {
                    data.append(&mut result);
                }
                Err(e) => {
                    dbg!(&sql);
                    dbg!(&e.to_string());
                }
            }
        }
    }
    Ok((data, tubi_data))
}

#[tokio::test]
async fn test_save_gps_material_dzcl() {
    init_test_surreal().await;
    let mut handles = Vec::new();
    let refno = RefU64::from_str("24383/66457").unwrap();
    let mut handle = save_gps_material_dzcl(refno).await;
    handles.append(&mut handle);
    futures::future::join_all(handles).await;
}
