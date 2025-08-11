#[cfg(feature = "sql")]
use super::query::create_table_sql;
#[cfg(feature = "sql")]
use super::query::save_material_value;
use crate::aios_db_mgr::aios_mgr::AiosDBMgr;
use crate::init_test_surreal;
use crate::SUL_DB;
use crate::{
    get_pe, insert_into_table_with_chunks, query_ele_filter_deep_children, NamedAttrValue, RefU64,
};
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use std::collections::HashMap;
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use tokio::task::{self, JoinHandle};

/// 通风专业 风管管段
pub async fn save_tf_material_hvac(
    refno: RefU64,
) -> Vec<JoinHandle<()>> {
    let mut handles = Vec::new();
    let db = SUL_DB.clone();
    match get_tf_hvac_material(&db, vec![refno]).await {
        Ok(r) => {
            if r.is_empty() {
                return handles;
            }
            let r_clone = r.clone();
            let task = task::spawn(async move {
                match insert_into_table_with_chunks(&db, "material_hvac_pipe", r_clone).await {
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
                    let table_name = "通风专业_风管管段清单".to_string();
                    let filed = vec![
                        "参考号",
                        "管段编号",
                        "材质",
                        "压力等级",
                        "风管宽度",
                        "风管高度",
                        "风管壁厚",
                        "风管面积",
                        "风管重量",
                        "加强筋型材",
                        "加强筋长度",
                        "加强筋重量",
                        "法兰规格",
                        "法兰长度",
                        "法兰重量",
                        "垫圈类型",
                        "垫圈长度",
                        "螺栓数量",
                        "其它材料类型",
                        "其它材料数量",
                        "螺杆",
                        "螺母数量",
                        "所在房间号",
                    ];
                    match create_table_sql(&pool, &table_name, &filed).await {
                        Ok(_) => {
                            if !r.is_empty() {
                                match save_material_value(&pool, &table_name, &filed, r).await {
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

/// 通风 风管管段
#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialTfHavcList {
    #[serde_as(as = "DisplayFromStr")]
    pub id: RefU64,
    #[serde(default)]
    pub bolt_qty: i32,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub duct_area: Vec<f32>,
    #[serde(default)]
    pub duct_weight: f32,
    #[serde(default)]
    pub fl_len: f32,
    #[serde(default)]
    pub fl_type: String,
    #[serde(default)]
    pub fl_wei: f32,
    #[serde(default)]
    pub height: String,
    #[serde(default)]
    pub length: f32,
    #[serde(default)]
    pub material: Vec<String>,
    #[serde(default)]
    pub nut_qty: i32,
    #[serde(default)]
    pub other_qty: String,
    #[serde(default)]
    pub other_type: String,
    #[serde(default)]
    pub pressure: String,
    #[serde(default)]
    pub room_no: Option<String>,
    #[serde(default)]
    pub seg_code: String,
    #[serde(default)]
    pub stif_len: f32,
    #[serde(default)]
    pub stif_sctn: String,
    #[serde(default)]
    pub stif_wei: f32,
    #[serde(default)]
    pub stud: String,
    #[serde(default)]
    pub sub_code: String,
    #[serde(default)]
    pub system: String,
    #[serde(default)]
    pub wall_thk: Vec<f32>,
    #[serde(default)]
    pub washer_len: f32,
    #[serde(default)]
    pub washer_type: Vec<String>,
    #[serde(default)]
    pub washer_qty: i32,
    #[serde(default)]
    pub width: String,
}

impl MaterialTfHavcList {
    //// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("描述".to_string()).or_insert(self.description);
        map.entry("管段编号".to_string()).or_insert(self.seg_code);
        // map.entry("子项号".to_string()).or_insert(self.sub_code);
        map.entry("材质".to_string())
            .or_insert(serde_json::to_string(&self.material).unwrap_or("[]".to_string()));
        map.entry("压力等级".to_string()).or_insert(self.pressure);
        // map.entry("风管长度".to_string())
        //     .or_insert(self.length.to_string());
        map.entry("风管宽度".to_string()).or_insert(self.width);
        map.entry("风管高度".to_string()).or_insert(self.height);
        map.entry("风管壁厚".to_string())
            .or_insert(serde_json::to_string(&self.wall_thk).unwrap_or("[]".to_string()));
        map.entry("风管面积".to_string())
            .or_insert(serde_json::to_string(&self.duct_area).unwrap_or("[]".to_string()));
        map.entry("风管重量".to_string())
            .or_insert(self.duct_weight.to_string());
        map.entry("加强筋型材".to_string())
            .or_insert(self.stif_sctn);
        map.entry("加强筋长度".to_string())
            .or_insert(self.stif_len.to_string());
        map.entry("加强筋重量".to_string())
            .or_insert(self.stif_wei.to_string());
        map.entry("法兰规格".to_string()).or_insert(self.fl_type);
        map.entry("法兰长度".to_string())
            .or_insert(self.fl_len.to_string());
        map.entry("法兰重量".to_string())
            .or_insert(self.fl_wei.to_string());
        map.entry("垫圈类型".to_string())
            .or_insert(serde_json::to_string(&self.washer_type).unwrap_or("[]".to_string()));
        map.entry("垫圈长度".to_string())
            .or_insert(self.washer_len.to_string());
        map.entry("螺栓数量".to_string())
            .or_insert(self.bolt_qty.to_string());
        map.entry("其它材料类型".to_string())
            .or_insert(self.other_type);
        map.entry("其它材料数量".to_string())
            .or_insert(self.other_qty);
        map.entry("螺杆".to_string()).or_insert(self.stud);
        map.entry("螺母数量".to_string())
            .or_insert(self.nut_qty.to_string());
        map.entry("所在房间号".to_string())
            .or_insert(self.room_no.unwrap_or("".to_string()));
        // map.entry("系统".to_string()).or_insert(self.system);
        map
    }
}

/// 通风 风管管段
#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialTfHavcTapeList {
    #[serde_as(as = "DisplayFromStr")]
    pub id: RefU64,
    #[serde(default)]
    pub bolt_qty: Vec<f32>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub duct_area: Vec<f32>,
    #[serde(default)]
    pub duct_weight: f32,
    #[serde(default)]
    pub fl_len: f32,
    #[serde(default)]
    pub fl_type: String,
    #[serde(default)]
    pub fl_wei: f32,
    #[serde(default)]
    pub height: Vec<String>,
    #[serde(default)]
    pub length: f32,
    #[serde(default)]
    pub material: Vec<String>,
    #[serde(default)]
    pub nut_qty: Vec<i32>,
    #[serde(default)]
    pub other_qty: String,
    #[serde(default)]
    pub other_type: String,
    #[serde(default)]
    pub pressure: String,
    #[serde(default)]
    pub room_no: Option<String>,
    #[serde(default)]
    pub seg_code: String,
    #[serde(default)]
    pub stif_len: f32,
    #[serde(default)]
    pub stif_sctn: String,
    #[serde(default)]
    pub stif_wei: f32,
    #[serde(default)]
    pub stud: String,
    #[serde(default)]
    pub sub_code: String,
    #[serde(default)]
    pub system: String,
    #[serde(default)]
    pub wall_thk: Vec<f32>,
    #[serde(default)]
    pub washer_len: f32,
    #[serde(default)]
    pub washer_type: Vec<String>,
    #[serde(default)]
    pub washer_qty: Vec<i32>,
    #[serde(default)]
    pub width: Vec<String>,
}

impl MaterialTfHavcTapeList {
    //// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        // map.entry("描述".to_string()).or_insert(self.description);
        map.entry("管段编号".to_string()).or_insert(self.seg_code);
        // map.entry("子项号".to_string()).or_insert(self.sub_code);
        map.entry("材质".to_string())
            .or_insert(serde_json::to_string(&self.material).unwrap_or("[]".to_string()));
        map.entry("压力等级".to_string()).or_insert(self.pressure);
        // map.entry("风管长度".to_string())
        //     .or_insert(self.length.to_string());
        map.entry("风管宽度".to_string())
            .or_insert(serde_json::to_string(&self.width).unwrap_or("[]".to_string()));
        map.entry("风管高度".to_string())
            .or_insert(serde_json::to_string(&self.height).unwrap_or("[]".to_string()));
        map.entry("风管壁厚".to_string())
            .or_insert(serde_json::to_string(&self.wall_thk).unwrap_or("[]".to_string()));
        map.entry("风管面积".to_string())
            .or_insert(serde_json::to_string(&self.duct_area).unwrap_or("[]".to_string()));
        map.entry("风管重量".to_string())
            .or_insert(self.duct_weight.to_string());
        map.entry("加强筋型材".to_string())
            .or_insert(self.stif_sctn);
        map.entry("加强筋长度".to_string())
            .or_insert(self.stif_len.to_string());
        map.entry("加强筋重量".to_string())
            .or_insert(self.stif_wei.to_string());
        map.entry("法兰规格".to_string()).or_insert(self.fl_type);
        map.entry("法兰长度".to_string())
            .or_insert(self.fl_len.to_string());
        map.entry("法兰重量".to_string())
            .or_insert(self.fl_wei.to_string());
        map.entry("垫圈类型".to_string())
            .or_insert(serde_json::to_string(&self.washer_type).unwrap_or("[]".to_string()));
        map.entry("垫圈长度".to_string())
            .or_insert(self.washer_len.to_string());
        map.entry("螺栓数量".to_string())
            .or_insert(serde_json::to_string(&self.bolt_qty).unwrap_or("[]".to_string()));
        map.entry("其它材料类型".to_string())
            .or_insert(self.other_type);
        map.entry("其它材料数量".to_string())
            .or_insert(self.other_qty);
        map.entry("螺杆".to_string()).or_insert(self.stud);
        map.entry("螺母数量".to_string())
            .or_insert(serde_json::to_string(&self.nut_qty).unwrap_or("[]".to_string()));
        map.entry("螺母数量_2".to_string())
            .or_insert(serde_json::to_string(&self.washer_qty).unwrap_or("[]".to_string()));
        map.entry("所在房间号".to_string())
            .or_insert(self.room_no.unwrap_or("".to_string()));
        // map.entry("系统".to_string()).or_insert(self.system);
        map
    }
}

/// 通风 风管管段
#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialTfHavcFlexList {
    #[serde_as(as = "DisplayFromStr")]
    pub id: RefU64,
    #[serde(default)]
    pub bolt_qty: Vec<f32>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub duct_area: Vec<f32>,
    #[serde(default)]
    pub duct_weight: f32,
    #[serde(default)]
    pub fl_len: f32,
    #[serde(default)]
    pub fl_type: String,
    #[serde(default)]
    pub fl_wei: f32,
    #[serde(default)]
    pub height: String,
    #[serde(default)]
    pub length: f32,
    #[serde(default)]
    pub material: Vec<String>,
    #[serde(default)]
    pub nut_qty: Vec<i32>,
    #[serde(default)]
    pub other_qty: String,
    #[serde(default)]
    pub other_type: String,
    #[serde(default)]
    pub pressure: String,
    #[serde(default)]
    pub room_no: Option<String>,
    #[serde(default)]
    pub seg_code: String,
    #[serde(default)]
    pub stif_len: f32,
    #[serde(default)]
    pub stif_sctn: String,
    #[serde(default)]
    pub stif_wei: f32,
    #[serde(default)]
    pub stud: String,
    #[serde(default)]
    pub sub_code: String,
    #[serde(default)]
    pub system: String,
    #[serde(default)]
    pub wall_thk: Vec<f32>,
    #[serde(default)]
    pub washer_len: f32,
    #[serde(default)]
    pub washer_type: Vec<String>,
    #[serde(default)]
    pub washer_qty: Vec<i32>,
    #[serde(default)]
    pub width: String,
}

impl MaterialTfHavcFlexList {
    //// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("描述".to_string()).or_insert(self.description);
        map.entry("管段编号".to_string()).or_insert(self.seg_code);
        // map.entry("子项号".to_string()).or_insert(self.sub_code);
        map.entry("材质".to_string())
            .or_insert(serde_json::to_string(&self.material).unwrap_or("[]".to_string()));
        map.entry("压力等级".to_string()).or_insert(self.pressure);
        // map.entry("风管长度".to_string())
        //     .or_insert(self.length.to_string());
        map.entry("风管宽度".to_string()).or_insert(self.width);
        map.entry("风管高度".to_string()).or_insert(self.height);
        map.entry("风管壁厚".to_string())
            .or_insert(serde_json::to_string(&self.wall_thk).unwrap_or("[]".to_string()));
        map.entry("风管面积".to_string())
            .or_insert(serde_json::to_string(&self.duct_area).unwrap_or("[]".to_string()));
        map.entry("风管重量".to_string())
            .or_insert(self.duct_weight.to_string());
        map.entry("加强筋型材".to_string())
            .or_insert(self.stif_sctn);
        map.entry("加强筋长度".to_string())
            .or_insert(self.stif_len.to_string());
        map.entry("加强筋重量".to_string())
            .or_insert(self.stif_wei.to_string());
        map.entry("法兰规格".to_string()).or_insert(self.fl_type);
        map.entry("法兰长度".to_string())
            .or_insert(self.fl_len.to_string());
        map.entry("法兰重量".to_string())
            .or_insert(self.fl_wei.to_string());
        map.entry("垫圈类型".to_string())
            .or_insert(serde_json::to_string(&self.washer_type).unwrap_or("[]".to_string()));
        map.entry("垫圈长度".to_string())
            .or_insert(self.washer_len.to_string());
        map.entry("螺栓数量".to_string())
            .or_insert(serde_json::to_string(&self.bolt_qty).unwrap_or("[]".to_string()));
        map.entry("其它材料类型".to_string())
            .or_insert(self.other_type);
        map.entry("其它材料数量".to_string())
            .or_insert(self.other_qty);
        map.entry("螺杆".to_string()).or_insert(self.stud);
        map.entry("螺母数量".to_string())
            .or_insert(serde_json::to_string(&self.nut_qty).unwrap_or("[]".to_string()));
        map.entry("所在房间号".to_string())
            .or_insert(self.room_no.unwrap_or("".to_string()));
        // map.entry("系统".to_string()).or_insert(self.system);
        map
    }
}

/// 通风 风管管段
pub async fn get_tf_hvac_material(
    db: &Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
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
            let refnos = query_ele_filter_deep_children(
                refno.into(),
                &[
                    "BEND", "BRCO", "CAP", "FLEX", "OFST", "STIF", "STRT", "TAPE", "THRE", "TRNS","TEE"
                ],
            )
                .await?;
            // STRT
            let strts = refnos
                .iter()
                .filter(|x| x.noun == "STRT".to_string())
                .map(|x| x.refno.into())
                .collect::<Vec<_>>();
            dbg!("STRT");
            let mut result = get_tf_hvac_strt_data(db, strts).await?;
            data.append(&mut result);
            // TAPE
            let tapes = refnos
                .iter()
                .filter(|x| x.noun == "TAPE".to_string())
                .map(|x| x.refno.into())
                .collect::<Vec<_>>();
            dbg!("TAPE");
            let mut result = get_tf_hvac_tape_data(db, tapes).await?;
            data.append(&mut result);
            // FLEX
            let tapes = refnos
                .iter()
                .filter(|x| x.noun == "FLEX".to_string())
                .map(|x| x.refno.into())
                .collect::<Vec<_>>();
            dbg!("FLEX");
            let mut result = get_tf_hvac_flex_data(db, tapes).await?;
            data.append(&mut result);
            // BEND
            let bends = refnos
                .iter()
                .filter(|x| x.noun == "BEND".to_string())
                .map(|x| x.refno.into())
                .collect::<Vec<_>>();
            dbg!("BEND");
            let mut result = get_tf_hvac_bend_data(db, bends).await?;
            data.append(&mut result);
            // OFST
            let bends = refnos
                .iter()
                .filter(|x| x.noun == "OFST".to_string())
                .map(|x| x.refno.into())
                .collect::<Vec<_>>();
            dbg!("OFST");
            let mut result = get_tf_hvac_ofst_data(db, bends).await?;
            data.append(&mut result);
            // TRNS
            let trnses = refnos
                .iter()
                .filter(|x| x.noun == "TRNS".to_string())
                .map(|x| x.refno.into())
                .collect::<Vec<_>>();
            dbg!("TRNS");
            let mut result = get_tf_hvac_trns_data(db, trnses).await?;
            data.append(&mut result);
            // BRCO
            let brcos = refnos
                .iter()
                .filter(|x| x.noun == "BRCO".to_string())
                .map(|x| x.refno.into())
                .collect::<Vec<_>>();
            dbg!("BRCO");
            let mut result = get_tf_hvac_brco_data(db, brcos).await?;
            data.append(&mut result);
            // THRE
            let brcos = refnos
                .iter()
                .filter(|x| x.noun == "THRE".to_string())
                .map(|x| x.refno.into())
                .collect::<Vec<_>>();
            dbg!("THRE");
            let mut result = get_tf_hvac_thre_data(db, brcos).await?;
            data.append(&mut result);
            // TEE
            let tees = refnos
                .iter()
                .filter(|x| x.noun == "TEE".to_string())
                .map(|x| x.refno.into())
                .collect::<Vec<_>>();
            dbg!("TEE");
            let mut result = get_tf_hvac_tee_data(db, tees).await?;
            data.append(&mut result);
            // CAP
            let caps = refnos
                .iter()
                .filter(|x| x.noun == "CAP".to_string())
                .map(|x| x.refno.into())
                .collect::<Vec<_>>();
            dbg!("CAP");
            let mut result = get_tf_hvac_cap_data(db, caps).await?;
            data.append(&mut result);
            // STIF
            let caps = refnos
                .iter()
                .filter(|x| x.noun == "STIF".to_string())
                .map(|x| x.refno.into())
                .collect::<Vec<_>>();
            dbg!("STIF");
            let mut result = get_tf_hvac_stif_data(db, caps).await?;
            data.append(&mut result);
        }
    }
    Ok(data)
}

/// 获取 通风 风管管段 strt 的数据
async fn get_tf_hvac_strt_data(
    db: &Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
    if refnos.is_empty() {
        return Ok(vec![]);
    };
    let mut data = Vec::new();
    let refnos = refnos
        .into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!("return fn::fggd_strt([{}]);", refnos);
    let mut response = db.query(&sql).await?;
    match response.take::<Vec<HashMap<String, NamedAttrValue>>>(0) {
        Ok(result) => {
            let mut r = change_hvac_result_to_map(result);
            data.append(&mut r);
        }
        Err(e) => {
            dbg!(&sql);
            println!("Error: {}", e);
        }
    }
    Ok(data)
}

/// 获取 通风 风管管段 tape 的数据
async fn get_tf_hvac_tape_data(
    db: &Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
    if refnos.is_empty() {
        return Ok(vec![]);
    };
    let mut data = Vec::new();
    let refnos = refnos
        .into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!("return fn::fggd_tape([{}]);", refnos);
    let mut response = db.query(&sql).await?;
    match response.take::<Vec<HashMap<String, NamedAttrValue>>>(0) {
        Ok(result) => {
            let mut r = change_hvac_result_to_map(result);
            data.append(&mut r);
        }
        Err(e) => {
            dbg!(&sql);
            println!("Error: {}", e);
        }
    }
    Ok(data)
}

/// 获取 通风 风管管段 flex 的数据
async fn get_tf_hvac_flex_data(
    db: &Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
    if refnos.is_empty() {
        return Ok(vec![]);
    };
    let mut data = Vec::new();
    let refnos = refnos
        .into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>()
        .join(",");

    let sql = format!("return fn::fggd_flex([{}]);", refnos);

    let mut response = db.query(&sql).await?;
    match response.take::<Vec<HashMap<String, NamedAttrValue>>>(0) {
        Ok(result) => {
            let mut r = change_hvac_result_to_map(result);
            data.append(&mut r);
        }
        Err(e) => {
            dbg!(&sql);
            println!("Error: {}", e);
        }
    }
    Ok(data)
}

/// 获取 通风 风管管段 bend 的数据
async fn get_tf_hvac_bend_data(
    db: &Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
    if refnos.is_empty() {
        return Ok(vec![]);
    };
    let mut data = Vec::new();
    let refnos = refnos
        .into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!("return fn::fggd_bend([{}]);", refnos);
    let mut response = db.query(&sql).await?;
    match response.take::<Vec<HashMap<String, NamedAttrValue>>>(0) {
        Ok(result) => {
            let mut r = change_hvac_result_to_map(result);
            data.append(&mut r);
        }
        Err(e) => {
            dbg!(&sql);
            println!("Error: {}", e);
        }
    }
    Ok(data)
}

/// 将通风材料表单的数据转化为hashmap，并将key改为对应的中文
fn change_hvac_result_to_map(
    input: Vec<HashMap<String, NamedAttrValue>>,
) -> Vec<HashMap<String, String>> {
    let chinese_name_map = get_hvac_chinese_name_map();
    let mut result = Vec::new();
    for i in input {
        let mut map = HashMap::new();
        for (k, v) in i {
            let Some(chinese) = chinese_name_map.get(&k) else {
                continue;
            };
            map.entry(chinese.to_string())
                .or_insert(v.get_val_as_string());
        }
        map.entry("version_tag".to_string()).or_insert("".to_string());
        result.push(map);
    }
    result
}

/// 获取通风材料表单字段对应的中文名
fn get_hvac_chinese_name_map() -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.entry("id".to_string())
        .or_insert("参考号".to_string());

    // map.entry("desc".to_string())
    //     .or_insert("描述".to_string());

    map.entry("bran_seg_code".to_string())
        .or_insert("管段编号".to_string());

    // map.entry("sub_code".to_string())
    //     .or_insert("子项号".to_string());

    map.entry("material".to_string())
        .or_insert("材质".to_string());

    map.entry("pressure_level".to_string())
        .or_insert("压力等级".to_string());

    // map.entry("l".to_string())
    //     .or_insert("风管长度".to_string());

    map.entry("w".to_string())
        .or_insert("风管宽度".to_string());

    map.entry("h".to_string())
        .or_insert("风管高度".to_string());

    map.entry("x".to_string())
        .or_insert("坐标X".to_string());

    map.entry("y".to_string())
        .or_insert("坐标Y".to_string());

    map.entry("z".to_string())
        .or_insert("坐标Z".to_string());

    map.entry("thickness".to_string())
        .or_insert("风管壁厚".to_string());

    map.entry("area".to_string())
        .or_insert("风管面积".to_string());

    map.entry("weight".to_string())
        .or_insert("风管重量".to_string());

    map.entry("stif_sctn".to_string())
        .or_insert("加强筋型材".to_string());

    map.entry("stif_len".to_string())
        .or_insert("加强筋长度".to_string());

    map.entry("stif_wei".to_string())
        .or_insert("加强筋重量".to_string());

    map.entry("flan_steel".to_string())
        .or_insert("法兰规格".to_string());

    map.entry("flan_length".to_string())
        .or_insert("法兰长度".to_string());

    map.entry("flan_weight".to_string())
        .or_insert("法兰重量".to_string());

    map.entry("gask_size".to_string())
        .or_insert("垫圈类型".to_string());

    map.entry("shim_width".to_string())
        .or_insert("垫圈长度".to_string());

    map.entry("bolt_count".to_string())
        .or_insert("螺栓数量".to_string());

    map.entry("other_material_type".to_string())
        .or_insert("其它材料类型".to_string());

    map.entry("other_material_count".to_string())
        .or_insert("其它材料数量".to_string());

    map.entry("screw".to_string())
        .or_insert("螺杆".to_string());

    map.entry("nut_count".to_string())
        .or_insert("螺母数量".to_string());

    map.entry("gask_count".to_string())
        .or_insert("垫片数量".to_string());

    map.entry("room_num".to_string())
        .or_insert("所在房间号".to_string());

    // map.entry("system_name".to_string())
    //     .or_insert("系统".to_string());

    map
}

/// 获取 通风 风管管段 ofst 的数据
async fn get_tf_hvac_ofst_data(
    db: &Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
    if refnos.is_empty() {
        return Ok(vec![]);
    };
    let mut data = Vec::new();
    let refnos = refnos
        .into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!("return fn::fggd_ofst([{}]);", refnos);
    let mut response = db.query(&sql).await?;
    match response.take::<Vec<HashMap<String, NamedAttrValue>>>(0) {
        Ok(result) => {
            let mut r = change_hvac_result_to_map(result);
            data.append(&mut r);
        }
        Err(e) => {
            dbg!(&sql);
            println!("Error: {}", e);
        }
    }
    Ok(data)
}

/// 获取 通风 风管管段 trns 的数据
async fn get_tf_hvac_trns_data(
    db: &Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
    if refnos.is_empty() {
        return Ok(vec![]);
    };
    let mut data = Vec::new();
    let refnos = refnos
        .into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "return fn::fggd_TRNS([{}]);",
        refnos
    );
    let mut response = db.query(&sql).await?;
    match response.take::<Vec<HashMap<String, NamedAttrValue>>>(0) {
        Ok(result) => {
            let mut r = change_hvac_result_to_map(result);
            data.append(&mut r);
        }
        Err(e) => {
            dbg!(&sql);
            println!("Error: {}", e);
        }
    }
    Ok(data)
}

/// 获取 通风 风管管段 brco 的数据
async fn get_tf_hvac_brco_data(
    db: &Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
    if refnos.is_empty() {
        return Ok(vec![]);
    };
    let mut data = Vec::new();
    let refnos = refnos
        .into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "return fn::fggd_BRCO([{}]);",
        refnos
    );
    let mut response = db.query(&sql).await?;
    match response.take::<Vec<HashMap<String, NamedAttrValue>>>(0) {
        Ok(result) => {
            let mut r = change_hvac_result_to_map(result);
            data.append(&mut r);
        }
        Err(e) => {
            dbg!(&sql);
            println!("Error: {}", e);
        }
    }
    Ok(data)
}

/// 获取 通风 风管管段 thre 的数据
async fn get_tf_hvac_thre_data(
    db: &Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
    if refnos.is_empty() {
        return Ok(vec![]);
    };
    let mut data = Vec::new();
    let refnos = refnos
        .into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "return fn::fggd_THRE([{}]);",
        refnos
    );
    let mut response = db.query(&sql).await?;
    match response.take::<Vec<HashMap<String, NamedAttrValue>>>(0) {
        Ok(result) => {
            let mut r = change_hvac_result_to_map(result);
            data.append(&mut r);
        }
        Err(e) => {
            dbg!(&sql);
            println!("Error: {}", e);
        }
    }
    Ok(data)
}

/// 获取 通风 风管管段 tee 的数据
async fn get_tf_hvac_tee_data(
    db: &Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
    if refnos.is_empty() {
        return Ok(vec![]);
    };
    let mut data = Vec::new();
    let refnos = refnos
        .into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "return fn::fggd_tee([{}]);",
        refnos
    );
    let mut response = db.query(&sql).await?;
    match response.take::<Vec<HashMap<String, NamedAttrValue>>>(0) {
        Ok(result) => {
            let mut r = change_hvac_result_to_map(result);
            data.append(&mut r);
        }
        Err(e) => {
            dbg!(&sql);
            println!("Error: {}", e);
        }
    }
    Ok(data)
}

/// 获取 通风 风管管段 cap 的数据
async fn get_tf_hvac_cap_data(
    db: &Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
    if refnos.is_empty() {
        return Ok(vec![]);
    };
    let mut data = Vec::new();
    let refnos = refnos
        .into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "return fn::fggd_CAP([{}]);",
        refnos
    );
    let mut response = db.query(&sql).await?;
    match response.take::<Vec<HashMap<String, NamedAttrValue>>>(0) {
        Ok(result) => {
            let mut r: Vec<HashMap<String, String>> = change_hvac_result_to_map(result);
            data.append(&mut r);
        }
        Err(e) => {
            dbg!(&sql);
            println!("Error: {}", e);
        }
    }
    Ok(data)
}

/// 获取 通风 风管管段 stif 的数据
async fn get_tf_hvac_stif_data(
    db: &Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
    if refnos.is_empty() {
        return Ok(vec![]);
    };
    let mut data = Vec::new();
    let refnos = refnos
        .into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "return fn::fggd_STIF([{}]);",
        refnos
    );
    let mut response = db.query(&sql).await?;
    match response.take::<Vec<HashMap<String, NamedAttrValue>>>(0) {
        Ok(result) => {
            let mut r = change_hvac_result_to_map(result);
            data.append(&mut r);
        }
        Err(e) => {
            dbg!(&sql);
            println!("Error: {}", e);
        }
    }
    Ok(data)
}


/// 声明通风专业定义的方法
pub async fn define_tf_surreal_functions(db: &Surreal<Any>) -> anyhow::Result<()> {
    let path = "rs_surreal/material_list/tf";
    let files = std::fs::read_dir(path)?;
    for file in files {
        let file = file?;
        let path = file.path();
        if !path.file_name().unwrap().to_str().unwrap().ends_with(".surql") {
            continue;
        }
        let content = std::fs::read_to_string(path)?;
        db.query(content).await.unwrap();
    }
    Ok(())
}

#[tokio::test]
async fn test_define_tf_surreal_functions() {
    init_test_surreal().await;
    define_tf_surreal_functions(&SUL_DB).await.unwrap();
}

