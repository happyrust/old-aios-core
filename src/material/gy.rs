#[cfg(feature = "sql")]
use super::query::create_table_sql;
#[cfg(feature = "sql")]
use super::query::{save_material_data_to_mysql, save_two_material_data_to_mysql};
use crate::aios_db_mgr::aios_mgr::{self, AiosDBMgr};
use crate::material::define_core_material_surreal_funtions;
#[cfg(feature = "sql")]
use crate::material::query::save_material_value_test;
use crate::{
    RefU64, SUL_DB, get_pe, init_test_surreal, insert_into_table_with_chunks,
    query_filter_deep_children,
};
use anyhow::anyhow;
use lazy_static::lazy_static;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use tokio::task::{self, JoinHandle};

const DZ_COLUMNS: [&str; 17] = [
    "参考号",
    "编码",
    "类型",
    "部件",
    "公称直径（主）",
    "公称直径（支）",
    "材料",
    "RCC_M",
    "SCH/LB（主）",
    "SCH/LB（支）",
    "制造形式",
    "连接形式",
    "标准",
    "单重（kg）",
    "总重（kg）",
    "数量",
    "单位",
];

lazy_static! {
    static ref DZ_CHINESE_FIELDS: HashMap<&'static str, &'static str> = {
        let mut map = HashMap::new();
        map.entry("id").or_insert("参考号");
        map.entry("code").or_insert("编码");
        map.entry("noun").or_insert("类型");
        map.entry("count").or_insert("数量");
        map.entry("length").or_insert("数量");
        map
    };
    static ref EQUI_CHINESE_FIELDS: HashMap<&'static str, &'static str> = {
        let mut map = HashMap::new();
        map.entry("id").or_insert("参考号");
        map.entry("name").or_insert("设备位号");
        map.entry("room_code").or_insert("所在房间号");
        map.entry("nozz_name").or_insert("管口号");
        map.entry("nozz_pos").or_insert("管口坐标");
        map.entry("nozz_cref").or_insert("相连管道编号");
        map
    };
    static ref VALV_CHINESE_FIELDS: HashMap<&'static str, &'static str> = {
        let mut map = HashMap::new();
        map.entry("id").or_insert("参考号");
        map.entry("valv_name").or_insert("阀门位号");
        map.entry("room_code").or_insert("所在房间号");
        map.entry("valv_belong").or_insert("阀门归属");
        map.entry("valv_length").or_insert("阀门长度");
        map.entry("valv_weight").or_insert("阀门重量");
        map.entry("valv_x").or_insert("阀门重心X");
        map.entry("valv_y").or_insert("阀门重心Y");
        map.entry("valv_z").or_insert("阀门重心Z");
        map.entry("valv_supp").or_insert("是否阀门支架");
        map
    };
}

const TABLE: &'static str = "工艺布置专业_大宗材料";

const EQ_FIELDS: [&'static str; 6] = [
    "参考号",
    "设备位号",
    "所在房间号",
    "管口号",
    "管口坐标",
    "相连管道编号",
];
const EQ_TABLE: &'static str = "工艺布置专业_设备清单";

const EQ_DATA_FIELDS: [&'static str; 6] = [
    "id",
    "name",
    "room_code",
    "nozz_name",
    "nozz_pos",
    "nozz_cref",
];

const VALVE_FIELDS: [&'static str; 10] = [
    "参考号",
    "阀门位号",
    "所在房间号",
    "阀门归属",
    "阀门长度",
    "阀门重量",
    "阀门重心X",
    "阀门重心Y",
    "阀门重心Z",
    "是否阀门支架",
];

const VALVE_DATA_FIELDS: [&'static str; 10] = [
    "id",
    "valv_name",
    "room_code",
    "valv_belong",
    "valv_length",
    "valv_weight",
    "valv_x",
    "valv_y",
    "valv_z",
    "valv_supp",
];

const VALVE_TABLE: &'static str = "工艺布置专业_阀门清单";

/// 工艺专业 大宗材料
pub async fn save_gy_material_dzcl(refno: RefU64) -> Vec<JoinHandle<()>> {
    let db = SUL_DB.clone();
    let mut handles = vec![];
    match get_gy_dzcl(db.clone(), vec![refno]).await {
        Ok((r, tubi_r)) => {
            let r_clone = r.clone();
            let tubi_r_clone = tubi_r.clone();
            let db = db.clone();
            let task: JoinHandle<()> = task::spawn(async move {
                if !r_clone.is_empty() {
                    let _ = insert_into_table_with_chunks(&db, "material_gy_list", r_clone).await;
                }
                if !tubi_r_clone.is_empty() {
                    let _ =
                        insert_into_table_with_chunks(&db, "material_gy_list_tubi", tubi_r_clone)
                            .await;
                }
            });
            handles.push(task);
            #[cfg(feature = "sql")]
            {
                let Ok(pool) = AiosDBMgr::get_project_pool().await else {
                    dbg!("无法连接到数据库");
                    return vec![];
                };
                let task = task::spawn(async move {
                    match create_table_sql(&pool, &TABLE, &DZ_COLUMNS).await {
                        Ok(_) => {
                            // 保存到数据库
                            if !r.is_empty() {
                                let data_field_1 = vec!["id", "code", "noun"];
                                let data_field_2 = vec!["id", "code", "noun", "length"];
                                match save_two_material_data_to_mysql(
                                    &TABLE,
                                    &DZ_CHINESE_FIELDS,
                                    &data_field_1,
                                    r,
                                    &data_field_2,
                                    tubi_r,
                                    &pool,
                                )
                                .await
                                {
                                    Ok(_) => {}
                                    Err(e) => {
                                        dbg!(e.to_string());
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

/// 工艺专业 设备清单
pub async fn save_gy_material_equi(refno: RefU64) -> Vec<JoinHandle<()>> {
    let mut handles = vec![];
    let db = SUL_DB.clone();
    match get_gy_equi_list(db.clone(), vec![refno]).await {
        Ok(r) => {
            let r_clone = r.clone();
            let task = task::spawn(async move {
                match insert_into_table_with_chunks(&db, "material_gy_equi", r_clone).await {
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
                    return vec![];
                };
                let task = task::spawn(async move {
                    match create_table_sql(&pool, &EQ_TABLE, &EQ_FIELDS).await {
                        Ok(_) => {
                            if !r.is_empty() {
                                match save_material_value_test(
                                    &pool,
                                    &EQ_TABLE,
                                    &EQ_DATA_FIELDS,
                                    &EQUI_CHINESE_FIELDS,
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

/// 工艺专业 阀门清单
pub async fn save_gy_material_valv(refno: RefU64) -> Vec<JoinHandle<()>> {
    let db = SUL_DB.clone();
    let mut handles = vec![];
    match get_gy_valv_list(db.clone(), vec![refno]).await {
        Ok(r) => {
            let r_clone = r.clone();
            let task = task::spawn(async move {
                match insert_into_table_with_chunks(&db, "material_gy_valv", r_clone).await {
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
                    return vec![];
                };
                let task = task::spawn(async move {
                    match create_table_sql(&pool, &VALVE_TABLE, &VALVE_FIELDS).await {
                        Ok(_) => {
                            if !r.is_empty() {
                                match save_material_value_test(
                                    &pool,
                                    &VALVE_TABLE,
                                    &VALVE_DATA_FIELDS,
                                    &VALV_CHINESE_FIELDS,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialGyDataBend {
    pub id: RefU64,
    pub code: String,
    pub noun: String,
    pub count: f32,
    #[serde(default)]
    pub version_tag: String,
}

impl MaterialGyDataBend {
    //// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("编码".to_string()).or_insert(self.code);
        map.entry("部件".to_string()).or_insert(self.noun);
        map.entry("数量".to_string())
            .or_insert(self.count.to_string());
        map
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialGyData {
    pub id: RefU64,
    pub code: String,
    pub noun: String,
    #[serde(default)]
    pub version_tag: String,
}

impl MaterialGyData {
    //转成行数据
    pub fn convert_to_row(self) -> Vec<String> {
        vec![self.id.to_pdms_str(), self.code, self.noun]
    }

    /// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("编码".to_string()).or_insert(self.code);
        map.entry("类型".to_string()).or_insert(self.noun);
        map
    }

    pub fn into_yk_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("编码".to_string()).or_insert(self.code);
        map.entry("品名".to_string()).or_insert(self.noun);
        map
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialGyValvList {
    pub id: RefU64,
    pub valv_name: String,
    pub room_code: Option<String>,
    pub valv_belong: String,
    pub valv_length: Option<f32>,
    pub valv_weight: Option<f32>,
    pub valv_x: Option<f32>,
    pub valv_y: Option<f32>,
    pub valv_z: Option<f32>,
    pub valv_supp: String,
    #[serde(default)]
    pub version_tag: String,
}

impl MaterialGyValvList {
    //// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("阀门位号".to_string())
            .or_insert(self.valv_name.to_string());
        map.entry("所在房间号".to_string())
            .or_insert(self.room_code.unwrap_or("".to_string()));
        map.entry("阀门归属".to_string())
            .or_insert(self.valv_belong.to_string());
        // 没有的给个默认值
        let valv_length = self.valv_length.map_or("0".to_string(), |x| x.to_string());
        map.entry("阀门长度".to_string())
            .or_insert(valv_length.to_string());
        let valv_weight = self.valv_weight.map_or("0".to_string(), |x| x.to_string());
        map.entry("阀门重量".to_string())
            .or_insert(valv_weight.to_string());
        let valv_x = self.valv_x.map_or("0".to_string(), |x| x.to_string());
        map.entry("阀门重心X".to_string())
            .or_insert(valv_x.to_string());
        let valv_y = self.valv_y.map_or("0".to_string(), |x| x.to_string());
        map.entry("阀门重心Y".to_string())
            .or_insert(valv_y.to_string());
        let valv_z = self.valv_z.map_or("0".to_string(), |x| x.to_string());
        map.entry("阀门重心Z".to_string())
            .or_insert(valv_z.to_string());

        map.entry("是否阀门支架".to_string())
            .or_insert(self.valv_supp.to_string());
        map
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialGyEquiList {
    pub id: RefU64,
    pub name: String,
    pub room_code: Option<String>,
    pub nozz_name: Vec<String>,
    pub nozz_pos: Vec<Vec<f32>>,
    pub nozz_cref: Vec<String>,
    #[serde(default)]
    pub version_tag: String,
}

impl MaterialGyEquiList {
    //// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("设备位号".to_string())
            .or_insert(self.name.to_string());
        map.entry("所在房间号".to_string())
            .or_insert(self.room_code.unwrap_or("".to_string()));
        map.entry("管口号".to_string())
            .or_insert(serde_json::to_string(&self.nozz_name).unwrap_or("[]".to_string()));
        map.entry("管口坐标".to_string())
            .or_insert(serde_json::to_string(&self.nozz_pos).unwrap_or("[]".to_string()));
        map.entry("相连管道编号".to_string())
            .or_insert(serde_json::to_string(&self.nozz_cref).unwrap_or("[]".to_string()));

        map
    }
}

///查询工艺大宗材料数据
///
/// 返回值 0: 除tubi外其他数据,  1:tubi的数据
pub async fn get_gy_dzcl(
    db: Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<(
    Vec<HashMap<String, serde_json::Value>>,
    Vec<HashMap<String, serde_json::Value>>,
)> {
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
        // 查询bend的数据
        let refnos = query_filter_deep_children(refno.into(), &["BEND"]).await?;
        let refnos_str = &refnos
            .into_iter()
            .map(|refno| refno.to_pe_key())
            .collect::<Vec<String>>()
            .join(",");
        let sql = format!(r#"return fn::gy_bend([{}])"#, refnos_str);
        let mut response = db.query(&sql).await?;
        match response.take::<Vec<HashMap<String, serde_json::Value>>>(0) {
            Ok(mut result) => {
                data.append(&mut result);
            }
            Err(e) => {
                dbg!(e.to_string());
                return Err(anyhow!(sql));
            }
        }
        // 查询tubi数据
        let refnos = query_filter_deep_children(refno.into(), &["BRAN"]).await?;
        let refnos_str = &refnos
            .into_iter()
            .map(|refno| refno.to_pe_key())
            .collect::<Vec<String>>()
            .join(",");
        let sql = format!(r#"return fn::gy_tubi([{}])"#, refnos_str);
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

        // 查询 elbo,tee,flan,gask,olet,redu,cap,couplig
        let refnos = query_filter_deep_children(
            refno.into(),
            &["ELBO", "TEE", "FLAN", "GASK", "OLET", "REDU", "CAP", "COUP"],
        )
        .await?;
        let refnos_str = &refnos
            .into_iter()
            .map(|refno| refno.to_pe_key())
            .collect::<Vec<String>>()
            .join(",");
        let sql = format!(r#"return fn::gy_part([{}])"#, refnos_str);
        let mut response = db.query(&sql).await?;
        match response.take::<Vec<HashMap<String, serde_json::Value>>>(0) {
            Ok(mut result) => {
                data.append(&mut result);
            }
            Err(e) => {
                dbg!(e.to_string());
                return Err(anyhow!(sql));
            }
        }
    }
    Ok((data, tubi_data))
}

/// 查询工艺阀门清单数据
pub async fn get_gy_valv_list(
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
            if !pe.name.contains("PIPE") {
                continue;
            };
        }
        // 查询阀门的数据
        let refnos = query_filter_deep_children(refno.into(), &["VALV", "INST"]).await?;
        let refnos_str = &refnos
            .into_iter()
            .map(|refno| refno.to_pe_key())
            .collect::<Vec<String>>()
            .join(",");
        let sql = format!(r#"return fn::gy_valve([{}])"#, refnos_str);
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

/// 查询工艺设备清单数据
pub async fn get_gy_equi_list(
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
            if !pe.name.contains("PIPE") {
                continue;
            };
        }
        // 查询设备的数据
        let refnos = query_filter_deep_children(refno.into(), &["EQUI"]).await?;
        let refnos_str = &refnos
            .into_iter()
            .map(|refno| refno.to_pe_key())
            .collect::<Vec<String>>()
            .join(",");
        let sql = format!(r#"return fn::gy_equip([{}])"#, refnos_str);
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

#[tokio::test]
async fn test_gy_bend() {
    let _ = init_test_surreal().await;
    let mut handles = vec![];
    if let Err(e) = define_core_material_surreal_funtions(SUL_DB.clone()).await {
        dbg!(e.to_string());
        return;
    }
    let refno = RefU64::from_str("24383/66478").unwrap();
    let mut handle = save_gy_material_dzcl(refno).await;
    handles.append(&mut handle);
    let refno = RefU64::from_str("24384/24775").unwrap();
    let mut handle = save_gy_material_equi(refno).await;
    handles.append(&mut handle);
    let refno = RefU64::from_str("24383/66457").unwrap();
    let mut handle = save_gy_material_valv(refno).await;
    handles.append(&mut handle);
    futures::future::join_all(handles).await;
}
