#[cfg(feature = "sql")]
use super::query::create_table_sql;
#[cfg(feature = "sql")]
use super::query::save_material_value;
#[cfg(feature = "sql")]
use super::query::save_material_value_test;

use crate::SUL_DB;
use crate::aios_db_mgr::aios_mgr::AiosDBMgr;
use crate::{
    RefU64, get_children_pes, get_pe, insert_into_table_with_chunks, query_filter_deep_children,
};
use crate::{RefnoEnum, init_test_surreal};
use calamine::{RangeDeserializerBuilder, Reader, Xls, open_workbook};
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use tokio::task::{self, JoinHandle};

lazy_static::lazy_static!(
    static ref DQ_CHINESE_FIELDS: HashMap<&'static str, &'static str> = {
        let mut map = HashMap::new();
        map.insert("id", "参考号");
        map.insert("num", "机组号");
        map.insert("project_num", "子项号");
        map.insert("project_name", "子项名称");
        map.insert("major", "专业");
        map.insert("room_code", "房间号");
        map.insert("name", "托盘段号");
        map.insert("pos", "托盘标高");
        map.insert("bran_type", "托盘类型");
        map.insert("supp_name", "托盘支吊架名称");
        map.insert("stander_num", "标准号");
        map.insert("item_num", "物项编号");
        map.insert("material", "材质");
        map.insert("width", "托盘宽度mm");
        map.insert("height", "托盘高度mm");
        map.insert("size_num", "规格型号");
        map.insert("b_painting", "是否刷漆");
        map.insert("painting_color", "刷漆颜色");
        map.insert("b_cover", "有无盖板");
        map.insert("b_partition", "有无隔板");
        map.insert("partition_num", "隔板编号");
        map.insert("spre", "spre");
        map.insert("catr", "catr");
        map.insert("horizontal_or_vertical", "水平/竖向");
        map.insert("unit", "单位");
        map.insert("count", "数量");
        // map.insert("length", "数量");
        map
    };
);

const FIELDS: [&'static str; 25] = [
    "参考号",
    "机组号",
    "子项号",
    "子项名称",
    "专业",
    "房间号",
    "托盘段号",
    "托盘标高",
    "托盘类型",
    "托盘支吊架名称",
    "标准号",
    "物项编号",
    "材质",
    "托盘宽度mm",
    "托盘高度mm",
    "规格型号",
    "是否刷漆",
    "刷漆颜色",
    "有无盖板",
    "有无隔板",
    "隔板编号",
    "共用隔板的托盘序号",
    "水平/竖向",
    "单位",
    "数量",
];

const BRAN_DATA_FIELDS: [&'static str; 24] = [
    "id",
    "num",
    "project_num",
    "project_name",
    "major",
    "room_code",
    "name",
    "pos",
    "bran_type",
    "supp_name",
    "stander_num",
    "item_num",
    "material",
    "width",
    "height",
    "size_num",
    "b_painting",
    "painting_color",
    "b_cover",
    "b_partition",
    "partition_num",
    "horizontal_or_vertical",
    "unit",
    "count",
];

const DQ_TABLE_NAME: &'static str = "电气专业_托盘及接地";

/// 电气专业 托盘及接地
pub async fn save_dq_material(refno: RefU64) -> Vec<JoinHandle<()>> {
    let db = SUL_DB.clone();
    let mut handles = vec![];
    match get_dq_bran_list(db.clone(), vec![refno]).await {
        Ok(mut r) => {
            let mut str_r = r.clone();
            let material_data = read_dq_material_excel().unwrap_or_default();
            get_dq_value_from_material(&material_data, &mut r);
            get_dq_value_from_material(&material_data, &mut str_r);

            let r_clone = r.clone();
            let str_r_clone = str_r.clone();
            let db_clone = db.clone();
            if !r_clone.is_empty() {
                let task = task::spawn(async move {
                    match insert_into_table_with_chunks(&db_clone, "material_elec_list", r_clone)
                        .await
                    {
                        Ok(_) => {}
                        Err(e) => {
                            dbg!(&e.to_string());
                        }
                    }
                });
                handles.push(task);
            }
            if !str_r_clone.is_empty() {
                let task = task::spawn(async move {
                    match insert_into_table_with_chunks(&db, "material_elec_list", str_r_clone)
                        .await
                    {
                        Ok(_) => {}
                        Err(e) => {
                            dbg!(&e.to_string());
                        }
                    }
                });
                handles.push(task);
            }
            #[cfg(feature = "sql")]
            {
                let Ok(pool) = AiosDBMgr::get_project_pool().await else {
                    dbg!("无法连接到数据库");
                    return handles;
                };
                let task = task::spawn(async move {
                    match create_table_sql(&pool, &DQ_TABLE_NAME, &FIELDS).await {
                        Ok(_) => {
                            if !r.is_empty() {
                                match save_material_value_test(
                                    &pool,
                                    &DQ_TABLE_NAME,
                                    &BRAN_DATA_FIELDS,
                                    &DQ_CHINESE_FIELDS,
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

/// 读取电气专业材料表
fn read_dq_material_excel() -> anyhow::Result<HashMap<String, Vec<DqMaterial>>> {
    let mut map = HashMap::new();
    let mut workbook: Xls<_> = open_workbook("resource/电气专业大宗材料属性对应关系表.xls")?;
    let range = workbook.worksheet_range("电气专业大宗材料属性对应关系表")?;

    let mut iter = RangeDeserializerBuilder::new().from_range(&range)?;

    while let Some(result) = iter.next() {
        let value: DqMaterial = result?;
        if value.catr_name.is_some() {
            map.entry(value.catr_name.clone().unwrap())
                .or_insert_with(Vec::new)
                .push(value);
        }
    }
    Ok(map)
}

// 将材料表的 标准号 和 单位 填入其中
fn get_dq_value_from_material(
    material_data: &HashMap<String, Vec<DqMaterial>>,
    mut value: &mut Vec<HashMap<String, Value>>,
) {
    for mut d in value.iter_mut() {
        for foreign_key in ["spre", "catr"] {
            if d.contains_key(foreign_key)
                && material_data
                    .contains_key(&d.get(foreign_key).unwrap().to_string().replace("\"", ""))
            {
                let material_value = material_data
                    .get(&d.get(foreign_key).unwrap().to_string().replace("\"", ""))
                    .unwrap();
                if material_value.is_empty() {
                    continue;
                };
                let value = material_value[0].clone();
                d.entry("stander_num".to_string())
                    .or_insert(Value::String(value.stander_num.unwrap_or("".to_string())));
                d.entry("unit".to_string())
                    .or_insert(Value::String(value.unit.unwrap_or("".to_string())));
                d.entry("item_num".to_string())
                    .or_insert(Value::String(value.code.unwrap_or("".to_string())));
            }
        }
    }
}

// 将材料表的标准号 物项编号 和单位填入其中
fn get_dq_value_from_material_stru(
    material_data: &HashMap<String, Vec<DqMaterial>>,
    mut value: &mut Vec<MaterialDqMaterialListStru>,
) {
    for mut d in value.iter_mut() {
        if d.spre.is_some() && material_data.contains_key(&d.spre.clone().unwrap()) {
            let material_value = material_data.get(&d.spre.clone().unwrap()).unwrap();
            if material_value.is_empty() {
                continue;
            };
            let value = material_value[0].clone();
            d.stander_num = value.stander_num;
            d.unit = value.unit;
            d.item_num = value.code;
        } else if d.catr.is_some() && material_data.contains_key(&d.catr.clone().unwrap()) {
            let material_value = material_data.get(&d.catr.clone().unwrap()).unwrap();
            if material_value.is_empty() {
                continue;
            };
            let value = material_value[0].clone();
            d.stander_num = value.stander_num;
            d.unit = value.unit;
            d.item_num = value.code;
        }
    }
}

/// 电气 托盘及接地
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialDqMaterialList {
    pub id: RefU64,
    pub num: Option<String>,
    pub project_num: Option<String>,
    pub project_name: Option<String>,
    pub major: String,
    pub room_code: Option<String>,
    pub name: String,
    pub pos: Option<f32>,
    pub bran_type: Option<String>,
    pub material: Option<String>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub size_num: Option<f32>,
    pub b_painting: Option<String>,
    pub painting_color: Option<String>,
    pub b_cover: Option<String>,
    pub b_partition: Option<String>,
    pub partition_num: Option<String>,
    pub spre: Option<String>,
    pub catr: Option<String>,
    pub horizontal_or_vertical: Option<String>,
    #[serde(default)]
    pub stander_num: Option<String>,
    #[serde(default)]
    pub item_num: Option<String>,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub count: Option<f32>,
    #[serde(default)]
    pub version_tag: String,
}

impl MaterialDqMaterialList {
    //// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("机组号".to_string())
            .or_insert(self.num.unwrap_or("".to_string()));
        map.entry("子项号".to_string())
            .or_insert(self.project_num.unwrap_or("".to_string()));
        map.entry("元件等级名称".to_string())
            .or_insert(self.spre.clone().unwrap_or("".to_string()));
        map.entry("子项名称".to_string())
            .or_insert(self.project_name.unwrap_or("".to_string()));
        map.entry("专业".to_string()).or_insert(self.major);
        map.entry("房间号".to_string())
            .or_insert(self.room_code.unwrap_or("".to_string()));
        map.entry("托盘段号".to_string()).or_insert(self.name);
        map.entry("托盘标高".to_string())
            .or_insert(self.pos.unwrap_or(0.0).to_string());
        map.entry("托盘类型".to_string())
            .or_insert(self.bran_type.unwrap_or("".to_string()));
        map.entry("材质".to_string())
            .or_insert(self.material.unwrap_or("".to_string()));
        map.entry("托盘宽度mm".to_string())
            .or_insert(self.width.unwrap_or(0.0).to_string());
        map.entry("托盘高度mm".to_string())
            .or_insert(self.height.unwrap_or(0.0).to_string());
        map.entry("规格型号".to_string())
            .or_insert(self.size_num.unwrap_or(0.0).to_string());
        map.entry("是否刷漆".to_string())
            .or_insert(self.b_painting.unwrap_or("".to_string()));
        map.entry("刷漆颜色".to_string())
            .or_insert(self.painting_color.unwrap_or("".to_string()));
        map.entry("有无盖板".to_string())
            .or_insert(self.b_cover.unwrap_or("".to_string()));
        map.entry("有无隔板".to_string())
            .or_insert(self.b_partition.unwrap_or("".to_string()));
        map.entry("隔板编号".to_string())
            .or_insert(self.partition_num.unwrap_or("".to_string()));
        map.entry("spre".to_string())
            .or_insert(self.spre.unwrap_or("".to_string()));
        map.entry("catr".to_string())
            .or_insert(self.catr.unwrap_or("".to_string()));
        map.entry("水平/竖向".to_string())
            .or_insert(self.horizontal_or_vertical.unwrap_or("".to_string()));
        map.entry("标准号".to_string())
            .or_insert(self.stander_num.unwrap_or("".to_string()));
        map.entry("物项编号".to_string())
            .or_insert(self.item_num.unwrap_or("".to_string()));
        map.entry("单位".to_string())
            .or_insert(self.unit.unwrap_or("".to_string()));
        map.entry("数量".to_string())
            .or_insert(self.count.unwrap_or(0.0).to_string());
        map
    }
}

/// 电气 托盘及接地
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialDqMaterialListStru {
    pub id: RefU64,
    pub num: Option<String>,
    pub project_num: Option<String>,
    pub project_name: Option<String>,
    pub major: String,
    pub room_code: Option<String>,
    pub pos: Option<f32>,
    pub material: Option<String>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub supp_name: Option<String>,
    pub size_num: Option<String>,
    pub b_painting: Option<String>,
    pub painting_color: Option<String>,
    pub b_cover: Option<String>,
    pub b_partition: Option<String>,
    pub partition_num: Option<String>,
    pub spre: Option<String>,
    pub catr: Option<String>,
    pub horizontal_or_vertical: Option<String>,
    #[serde(default)]
    pub stander_num: Option<String>,
    #[serde(default)]
    pub item_num: Option<String>,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub count: Option<String>,
    #[serde(default)]
    pub version_tag: String,
}

impl MaterialDqMaterialListStru {
    //// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("机组号".to_string())
            .or_insert(self.num.unwrap_or("".to_string()));
        map.entry("子项号".to_string())
            .or_insert(self.project_num.unwrap_or("".to_string()));
        map.entry("元件等级名称".to_string())
            .or_insert(self.spre.clone().unwrap_or("".to_string()));
        map.entry("子项名称".to_string())
            .or_insert(self.project_name.unwrap_or("".to_string()));
        map.entry("专业".to_string()).or_insert(self.major);
        map.entry("房间号".to_string())
            .or_insert(self.room_code.unwrap_or("".to_string()));
        map.entry("托盘标高".to_string())
            .or_insert(self.pos.unwrap_or(0.0).to_string());
        map.entry("材质".to_string())
            .or_insert(self.material.unwrap_or("".to_string()));
        map.entry("托盘支吊架名称".to_string())
            .or_insert(self.supp_name.unwrap_or("".to_string()));
        map.entry("托盘宽度mm".to_string())
            .or_insert(self.width.unwrap_or(0.0).to_string());
        map.entry("托盘高度mm".to_string())
            .or_insert(self.height.unwrap_or(0.0).to_string());
        map.entry("规格型号".to_string())
            .or_insert(self.size_num.unwrap_or("".to_string()));
        map.entry("是否刷漆".to_string())
            .or_insert(self.b_painting.unwrap_or("".to_string()));
        map.entry("刷漆颜色".to_string())
            .or_insert(self.painting_color.unwrap_or("".to_string()));
        map.entry("有无隔板".to_string())
            .or_insert(self.b_partition.unwrap_or("".to_string()));
        map.entry("隔板编号".to_string())
            .or_insert(self.partition_num.unwrap_or("".to_string()));
        map.entry("spre".to_string())
            .or_insert(self.spre.unwrap_or("".to_string()));
        map.entry("catr".to_string())
            .or_insert(self.catr.unwrap_or("".to_string()));
        map.entry("水平/竖向".to_string())
            .or_insert(self.horizontal_or_vertical.unwrap_or("".to_string()));
        map.entry("标准号".to_string())
            .or_insert(self.stander_num.unwrap_or("".to_string()));
        map.entry("物项编号".to_string())
            .or_insert(self.item_num.unwrap_or("".to_string()));
        map.entry("单位".to_string())
            .or_insert(self.unit.unwrap_or("".to_string()));
        map
    }
}

/// 查询电气 托盘及接地 托盘
pub async fn get_dq_bran_list(
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
            if !pe.name.contains("ELEC") {
                continue;
            };
        }
        // 查询电气托盘的数据
        let refnos = query_filter_deep_children(refno.into(), &["BRAN"]).await?;
        if !refnos.is_empty() {
            let refnos_str = &refnos
                .into_iter()
                .map(|refno| refno.to_pe_key())
                .collect::<Vec<String>>()
                .join(",");
            let sql = format!(r#"return fn::dq_bran([{}])"#, refnos_str);
            let mut response = db.query(&sql).await?;
            match response.take::<Vec<HashMap<String, Value>>>(0) {
                Ok(mut result) => {
                    data.append(&mut result);
                }
                Err(e) => {
                    dbg!(&sql);
                    dbg!(&e);
                }
            }
        }
        // 查询电气支吊架的数据
        let refnos = query_filter_deep_children(refno.into(), &["STRU"]).await?;
        if !refnos.is_empty() {
            let refnos_str = &refnos
                .into_iter()
                .map(|refno| refno.to_pe_key())
                .collect::<Vec<String>>()
                .join(",");
            let sql = format!(r#"return fn::dq_stru([{}])"#, refnos_str);
            let mut response = db.query(&sql).await?;
            match response.take::<Vec<HashMap<String, Value>>>(0) {
                Ok(mut result) => {
                    data.append(&mut result);
                }
                Err(e) => {
                    dbg!(&sql);
                    dbg!(&e);
                }
            }
        }
        // 电缆及接地
        let refnos = query_filter_deep_children(refno.into(), &["GENSEC"]).await?;
        if !refnos.is_empty() {
            let refnos_str = &refnos
                .into_iter()
                .map(|refno| refno.to_pe_key())
                .collect::<Vec<String>>()
                .join(",");
            let sql = format!(r#"return fn::dq_gensec([{}])"#, refnos_str);
            let mut response = db.query(&sql).await?;
            match response.take::<Vec<HashMap<String, Value>>>(0) {
                Ok(mut result) => {
                    data.append(&mut result);
                }
                Err(e) => {
                    dbg!(&sql);
                    dbg!(&e);
                }
            }
        }
    }
    Ok(data)
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DqMaterial {
    #[serde(rename = "元件等级名称")]
    pub catr_name: Option<String>,
    #[serde(rename = "标准号")]
    pub stander_num: Option<String>,
    #[serde(rename = "物项代码")]
    pub code: Option<String>,
    #[serde(rename = "名称")]
    pub name: Option<String>,
    #[serde(rename = "DESIGNATION")]
    pub desc: Option<String>,
    #[serde(rename = "单位")]
    pub unit: Option<String>,
    #[serde(rename = "数量")]
    pub count: Option<String>,
    #[serde(rename = "质保等级")]
    pub level: Option<String>,
}

#[tokio::test]
async fn test_save_dq_material() {
    init_test_surreal().await;
    let mut handles = vec![];
    let mut handle = save_dq_material(RefU64::from("24384/25674")).await;
    handles.append(&mut handle);
    futures::future::join_all(handles).await;
}
