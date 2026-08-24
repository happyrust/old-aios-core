#[cfg(feature = "sql")]
use super::query::create_table_sql;
#[cfg(feature = "sql")]
use super::query::save_material_value;
#[cfg(feature = "sql")]
use super::query::save_material_value_test;
use crate::SUL_DB;
use crate::aios_db_mgr::PdmsDataInterface;
use crate::aios_db_mgr::aios_mgr::{self, AiosDBMgr};
use crate::init_test_surreal;
use crate::material::get_refnos_belong_major;
use crate::material::gy::MaterialGyData;
use crate::pe::SPdmsElement;
use crate::{
    RefU64, RefnoEnum, get_pe, insert_into_table_with_chunks, query_filter_ancestors,
    query_filter_deep_children,
};
use anyhow::anyhow;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use tokio::task::{self, JoinHandle};

lazy_static::lazy_static!(
    static ref YK_DZCL_CHINESE_FIELDS: HashMap<&'static str, &'static str> = {
        let mut map = HashMap::new();
        map.insert("id", "参考号");
        map.insert("code", "编码");
        map.insert("type", "品名");
        map
    };

    static ref YK_INST_CHINESE_FIELDS: HashMap<&'static str, &'static str> = {
        let mut map = HashMap::new();
        map.insert("id", "参考号");
        map.insert("name", "传感器标识");
        map.insert("pipe_name", "对应根阀编号");
        map.insert("room_code", "房间号");
        map
    };

    static ref YK_EQUI_CHINESE_FIELDS: HashMap<&'static str, &'static str> = {
        let mut map = HashMap::new();
        map.insert("id", "参考号");
        map.insert("equi_name", "仪控设备位号");
        map.insert("room_code", "所在房间号");
        map.insert("pos", "设备绝对标高");
        map.insert("floor_height", "设备相对楼板标高");
        map
    };
);

const FIELDS: [&str; 12] = [
    "参考号",
    "编码",
    "品名",
    "规格",
    "连接形式",
    "材料",
    "RCC-M",
    "质保等级",
    "抗震等级",
    "备注",
    "内部编码",
    "公称直径",
];

const YK_DZCL_DATA_FIELDS: [&str; 3] = ["id", "code", "type"];

const DZCL_TABLE: &'static str = "仪控专业_大宗材料";

const PIPE_TABLE: &'static str = "仪控专业_仪表管道";

const EQUI_TABLE: &'static str = "仪控专业_设备清单";

const PIPE_FIELDS: [&str; 4] = ["参考号", "传感器标识", "对应根阀编号", "房间号"];

const PIPE_DATA_FIELDS: [&str; 4] = ["id", "name", "pipe_name", "room_code"];

const EQUI_FIELDS: [&str; 5] = [
    "参考号",
    "仪控设备位号",
    "所在房间号",
    "设备绝对标高",
    "设备相对楼板标高",
];

const EQUI_DATA_FIELDS: [&str; 5] = ["id", "equi_name", "room_code", "pos", "floor_height"];

/// 仪控专业 大宗材料
pub async fn save_yk_material_dzcl(refno: RefU64) -> Vec<JoinHandle<()>> {
    let db = SUL_DB.clone();
    let mut handles = Vec::new();
    match get_yk_dzcl_list(db.clone(), vec![refno]).await {
        Ok(r) => {
            if r.is_empty() {
                return handles;
            }
            let r_clone = r.clone();
            let task = task::spawn(async move {
                match insert_into_table_with_chunks(&db, "material_inst_list", r_clone).await {
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
                    match create_table_sql(&pool, &DZCL_TABLE, &FIELDS).await {
                        Ok(_) => {
                            if !r.is_empty() {
                                match save_material_value_test(
                                    &pool,
                                    &DZCL_TABLE,
                                    &YK_DZCL_DATA_FIELDS,
                                    &YK_DZCL_CHINESE_FIELDS,
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
            dbg!(&e.to_string());
        }
    }
    handles
}

/// 仪控专业 仪表管道
pub async fn save_yk_material_pipe(refno: RefU64) -> Vec<JoinHandle<()>> {
    let db = SUL_DB.clone();
    let mut handles = Vec::new();
    match get_yk_inst_pipe(db.clone(), vec![refno]).await {
        Ok(r) => {
            if r.is_empty() {
                return handles;
            }
            let r_clone = r.clone();
            let task = task::spawn(async move {
                match insert_into_table_with_chunks(&db, "material_inst_pipe", r_clone).await {
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
                    match create_table_sql(&pool, &PIPE_TABLE, &PIPE_FIELDS).await {
                        Ok(_) => {
                            if !r.is_empty() {
                                match save_material_value_test(
                                    &pool,
                                    &PIPE_TABLE,
                                    &PIPE_DATA_FIELDS,
                                    &YK_INST_CHINESE_FIELDS,
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

/// 仪控专业 设备清单
pub async fn save_yk_material_equi(refno: RefU64) -> Vec<JoinHandle<()>> {
    let db = SUL_DB.clone();
    let mut handles = Vec::new();
    match get_yk_equi_list_material(db.clone(), vec![refno]).await {
        Ok(r) => {
            if r.is_empty() {
                return handles;
            }
            let r_clone = r.clone();
            let task = task::spawn(async move {
                match insert_into_table_with_chunks(&db, "material_inst_equi", r_clone).await {
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
                    match create_table_sql(&pool, &EQUI_TABLE, &EQUI_FIELDS).await {
                        Ok(_) => {
                            if !r.is_empty() {
                                match save_material_value_test(
                                    &pool,
                                    &EQUI_TABLE,
                                    &EQUI_DATA_FIELDS,
                                    &YK_EQUI_CHINESE_FIELDS,
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

/// 查询仪控 大宗材料
pub async fn get_yk_dzcl_list(
    db: Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<HashMap<String, serde_json::Value>>> {
    let mut data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno.into()).await? else {
            continue;
        };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("INST") {
                continue;
            };
        }
        // 查询bend的数据
        let refnos =
            query_filter_deep_children(refno.into(), &["VALV", "TEE", "COUP", "INST", "BEND"])
                .await?;
        let refnos_str = &refnos
            .into_iter()
            .map(|refno| refno.to_pe_key())
            .collect::<Vec<String>>()
            .join(",");
        let sql = format!(r#"return fn::yk_dzcl([{}])"#, refnos_str);
        let mut response = db.query(sql).await?;
        let mut result: Vec<HashMap<String, Value>> = response.take(0)?;
        data.append(&mut result);
    }
    Ok(data)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialYkInstData {
    pub id: RefU64,
    pub name: String,
    pub pipe_name: Option<String>,
    pub room_code: Option<String>,
    #[serde(default)]
    pub version_tag: String,
}

impl MaterialYkInstData {
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("传感器标识".to_string()).or_insert(self.name);
        map.entry("对应根阀编号".to_string())
            .or_insert(self.pipe_name.unwrap_or("".to_string()));
        map.entry("房间号".to_string())
            .or_insert(self.room_code.unwrap_or("".to_string()));
        map
    }
}

/// 仪控 仪表管道
pub async fn get_yk_inst_pipe(
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
            if !pe.name.contains("INST") {
                continue;
            };
        }
        // 查询 inst 的数据
        let refnos = query_filter_deep_children(refno.into(), &["INST"]).await?;
        if refnos.is_empty() {
            return Ok(vec![]);
        }
        let refnos_str = &refnos
            .into_iter()
            .map(|refno| refno.to_pe_key())
            .collect::<Vec<String>>()
            .join(",");
        let sql = format!(r#"return fn::yk_ybgd([{}])"#, refnos_str);
        let mut response = db.query(&sql).await?;
        match response.take(0) {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialYkEquiListData {
    pub id: RefU64,
    pub equi_name: String,
    pub room_code: Option<String>,
    pub pos: Option<f32>,
    pub floor_height: Option<f32>,
    #[serde(default)]
    pub version_tag: String,
}

impl MaterialYkEquiListData {
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("仪控设备位号".to_string())
            .or_insert(self.equi_name);
        map.entry("所在房间号".to_string())
            .or_insert(self.room_code.unwrap_or("".to_string()));
        map.entry("设备绝对标高".to_string())
            .or_insert(self.pos.unwrap_or(0.0).to_string());
        map.entry("设备相对楼板标高".to_string())
            .or_insert(self.floor_height.unwrap_or(0.0).to_string());
        map
    }
}

/// 仪控 设备清单
pub async fn get_yk_equi_list_material(
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
            if !pe.name.contains("INST") {
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
        let sql = format!(r#"return fn::yk_equi([{}])"#, refnos_str);
        let mut response = db.query(&sql).await?;
        match response.take(0) {
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

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
struct BelongGyValvResponse {
    pub id: RefnoEnum,
    pub noun: String,
    pub name: String,
    pub site: bool,
    pub valv: Vec<String>,
}

/// 查找仪控管段所属工艺管道的根阀
pub async fn query_yk_bran_belong_gy_valv_name(
    mut bran: RefU64,
    aios_mgr: &AiosDBMgr,
) -> anyhow::Result<Option<(String, String)>> {
    for _ in 0..5 {
        let mut sql = format!("select id,noun ,name?:'' as name, string::contains(fn::find_ancestor_type(id,'SITE').name?:'','PIPE') as site,
                    <-pe_owner.in.filter(|$x| $x.noun == 'VALV').name as valv from {}.refno.HREF", bran.to_pe_key());
        let mut resp = SUL_DB.query(&sql).await?;
        let r = resp.take::<Vec<BelongGyValvResponse>>(0)?;
        // 没有填href，返回的就是空
        if r.is_empty() {
            return Ok(None);
        };
        let r = r[0].clone();
        // 判断 href 是 bran还是 tee等管件
        match r.noun.as_str() {
            // 如果是bran,判断是否属于工艺专业，若属于，看该bran下是否有valv,有则返回valv的name
            "BRAN" => {
                if r.site {
                    if !r.valv.is_empty() {
                        return Ok(Some((r.noun.clone(), r.valv[0].clone())));
                    } else {
                        // 如果是工艺管道，则返回PIPE的nmae，赋值到 仪表管编号
                        let owner_name = aios_mgr.get_name(r.id.refno()).await?;
                        return Ok(Some(("PIPE".to_string(), owner_name)));
                    }
                } else {
                    // 不是工艺专业，则继续找href
                    bran = r.id.refno();
                }
            }
            "NOZZ" => {
                if r.site {
                    // 如果是NOZZ，则返回EQUI的nmae，赋值到 仪表管编号
                    let owner_name = aios_mgr.get_name(r.id.refno()).await?;
                    return Ok(Some(("EQUI".to_string(), owner_name)));
                } else {
                    // 不是工艺专业，则继续找href
                    bran = r.id.refno();
                }
            }
            // 如果是tee等管件，判断是否是工艺专业，如果是则返回None
            _ => {
                if r.site {
                    return Ok(None);
                } else {
                    // 不是工艺专业，则继续找href
                    bran = r.id.refno();
                }
            }
        }
    }
    Ok(None)
}

/// 查找仪控管段所属工艺管道或设备
pub async fn query_yk_bran_belong_gy_pipe_name(
    mut bran: RefU64,
    aios_mgr: &AiosDBMgr,
) -> anyhow::Result<Option<SPdmsElement>> {
    loop {
        // 获取href
        let Some(href) = aios_mgr.get_foreign_attr(bran, "HREF").await? else {
            break;
        };
        let Some(href_refno) = href.get_refno() else {
            break;
        };
        // 判断 href 是 bran还是nozz
        match href.get_type_str() {
            // 如果是nozz，直接返回所属equi
            "NOZZ" => {
                let equi = query_filter_ancestors(href_refno, &["EQUI"]).await?;
                if equi.is_empty() {
                    break;
                };
                return get_pe(equi[0]).await;
            }
            // 如果是bran，判断该bran是否属于工艺专业，若是则返回对应的pipe，若不是，则继续向上找href
            "BRAN" => {
                let major = get_refnos_belong_major(&vec![href_refno.into()]).await?;
                let Some(major) = major.get(&href_refno.into()) else {
                    break;
                };
                if major.major == "T".to_string() {
                    return get_pe(href.get_owner()).await;
                } else {
                    bran = href_refno.refno();
                }
            }
            &_ => {
                break;
            }
        }
    }
    Ok(None)
}

#[tokio::test]
async fn test_query_yk_bran_belong_gy_pipe_name() -> anyhow::Result<()> {
    let aios_mgr = AiosDBMgr::init_from_db_option().await?;
    let refno = RefU64::from_str("24381/177397").unwrap();
    let r = query_yk_bran_belong_gy_valv_name(refno, &aios_mgr).await?;
    dbg!(&r);
    let r = query_yk_bran_belong_gy_pipe_name(refno, &aios_mgr).await?;
    dbg!(&r);
    Ok(())
}

#[tokio::test]
async fn test_save_yk_material_dzcl() -> anyhow::Result<()> {
    init_test_surreal().await;
    let mut handles = Vec::new();
    let refno = RefU64::from_str("24381/177374").unwrap();
    let mut r = save_yk_material_dzcl(refno).await;
    handles.append(&mut r);
    let refno = RefU64::from_str("24383/66457").unwrap();
    let mut r = save_yk_material_pipe(refno).await;
    handles.append(&mut r);
    let refno = RefU64::from_str("24381/101420").unwrap();
    let mut r = save_yk_material_equi(refno).await;
    handles.append(&mut r);
    futures::future::join_all(handles).await;
    Ok(())
}
