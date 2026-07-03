use bevy_ecs::prelude::Resource;
use futures::StreamExt;
use glam::Vec3;
use serde::{Deserialize, Serialize};
use serde_json::error::Category::Data;
use std::collections::{HashMap, HashSet};

use crate::data_center::AttrValue::{AttrFloat, AttrStrArray, AttrString};
use crate::metadata_manager::FileBytes;
use crate::schema::generate_basic_versioned_schema;
use crate::types::*;
use bevy_ecs::prelude::Component;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataCenterProject {
    // #[serde(rename = "packageCode")]
    // pub package_code: String,
    #[serde(rename = "projectCode")]
    pub project_code: String,
    pub owner: String,
    pub instances: Vec<DataCenterInstance>,
}

impl DataCenterProject {
    /// 转化为新标准元数据格式
    pub fn into_new_type(
        self,
        code_book: &HashMap<String, CodeBookMapping>,
    ) -> DataCenterProjectNewType {
        let new_instance = self
            .instances
            .into_iter()
            .filter_map(|i| i.into_new_type(code_book))
            .collect::<Vec<_>>();
        DataCenterProjectNewType {
            project_code: self.project_code,
            owner: self.owner,
            instances: new_instance,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataCenterProjectNewType {
    // #[serde(rename = "packageCode")]
    // pub package_code: String,
    #[serde(rename = "projectCode")]
    pub project_code: String,
    pub owner: String,
    pub instances: Vec<DataCenterInstanceNewType>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataCenterProjectHH {
    // #[serde(rename = "packageCode")]
    // pub package_code: String,
    #[serde(rename = "projectCode")]
    pub project_code: String,
    pub owner: String,
    pub instances: Vec<DataCenterInstanceHH>,
}

impl DataCenterProject {
    pub fn convert_package_code() -> String {
        Uuid::new_v4().to_string()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Hash)]
pub struct DataCenterProjectWithRelations {
    #[serde(rename = "projectCode")]
    pub project_code: String,
    pub owner: String,
    pub instances: Vec<DataCenterInstance>,
    pub relations: Vec<DataCenterRelations>,
}

impl DataCenterProjectWithRelations {
    pub fn convert_package_code() -> String {
        Uuid::new_v4().to_string()
    }

    pub fn into_new_type(
        self,
        code_book: &HashMap<String, CodeBookMapping>,
    ) -> DataCenterProjectWithRelationsNewType {
        let new_instance = self
            .instances
            .into_iter()
            .filter_map(|i| i.into_new_type(code_book))
            .collect::<Vec<_>>();
        DataCenterProjectWithRelationsNewType {
            project_code: self.project_code,
            owner: self.owner,
            instances: new_instance,
            relations: self.relations,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataCenterProjectWithRelationsNewType {
    #[serde(rename = "projectCode")]
    pub project_code: String,
    pub owner: String,
    pub instances: Vec<DataCenterInstanceNewType>,
    pub relations: Vec<DataCenterRelations>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataCenterProjectWithRelationsHH {
    // #[serde(rename = "packageCode")]
    // pub package_code: String,
    #[serde(rename = "projectCode")]
    pub project_code: String,
    pub owner: String,
    pub instances: Vec<DataCenterInstanceHH>,
    pub relations: Vec<DataCenterRelationsHH>,
}

// #[derive(Serialize, Deserialize, Clone, Debug, Default)]
// pub struct DataCenterInstance {
//     #[serde(rename = "objectModelCode")]
//     pub object_model_code: String,
//     #[serde(rename = "instanceCode")]
//     pub instance_code: String,
//     #[serde(serialize_with = "serialize_option_string_with_default")]
//     pub operate: Option<String>,
//     pub version: String,
//     pub attributes: Vec<DataCenterAttr>,
// }

// #[cfg(not(feature = "hd"))]
#[derive(Serialize, Deserialize, Clone, Debug, Default, Hash)]
pub struct DataCenterInstance {
    #[serde(rename = "objectModelCode")]
    pub object_model_code: String,
    #[serde(rename = "instanceCode")]
    pub instance_code: String,
    #[serde(serialize_with = "serialize_option_string_with_default")]
    pub operate: Option<String>,
    // #[cfg(not(feature = "hd"))]
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub operate: Option<String>,
    pub version: String,
    pub attributes: Vec<DataCenterAttr>,
}

impl DataCenterInstance {
    pub fn into_new_type(
        self,
        code_book: &HashMap<String, CodeBookMapping>,
    ) -> Option<DataCenterInstanceNewType> {
        if let Some(new_object_code_map) = code_book.get(&self.object_model_code) {
            // 替换属性编码
            let mut attr = Vec::new();
            for value in self.attributes {
                if let Some(new_code) = new_object_code_map
                    .attr_mapping
                    .get(&value.attribute_model_code)
                {
                    attr.push(DataCenterAttr {
                        attribute_model_code: new_code.clone(),
                        value: value.value,
                    });
                } else {
                    println!(
                        "未发现密码本中: {} 对应的新编码，值为: {}",
                        value.attribute_model_code, value.value
                    );
                }
            }
            // 增加 工厂对象类编码 属性
            attr.push(DataCenterAttr {
                attribute_model_code: "PBS_COD".to_string(),
                value: self.object_model_code,
            });
            return Some(DataCenterInstanceNewType {
                object_model_code: new_object_code_map.new_object_code.clone(),
                operate: self.operate,
                version: self.version,
                attributes: attr,
            });
        }
        None
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Hash)]
pub struct DataCenterInstanceNewType {
    #[serde(rename = "objectModelCode")]
    pub object_model_code: String,
    #[serde(serialize_with = "serialize_option_string_with_default")]
    pub operate: Option<String>,
    // #[cfg(not(feature = "hd"))]
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub operate: Option<String>,
    pub version: String,
    pub attributes: Vec<DataCenterAttr>,
}

/// 代码映射结果结构
/// 外层HashMap: key=旧对象类编码, value=(新对象类编码, 属性映射表)
/// 内层HashMap: key=旧属性编码, value=新属性编码
#[derive(Debug, Clone)]
pub struct CodeBookMapping {
    pub new_object_code: String,
    pub attr_mapping: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataCenterInstanceHH {
    #[serde(rename = "objectModelCode")]
    pub object_model_code: String,
    #[serde(rename = "instanceCode")]
    pub instance_code: String,
    #[serde(serialize_with = "serialize_option_string_with_default")]
    pub operate: Option<String>,
    pub version: String,
    pub attributes: Vec<DataCenterAttr>,
}

fn serialize_option_string_with_default<S>(
    value: &Option<String>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match value {
        Some(v) => serializer.serialize_str(v),
        None => serializer.serialize_str("draft"), // default value
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Hash)]
pub struct DataCenterRelations {
    pub version: String,
    #[serde(rename = "objectModelCode")]
    pub object_model_code: String,
    #[serde(rename = "instanceCode")]
    pub instance_code: String,
    #[serde(serialize_with = "serialize_option_string_with_default")]
    pub operate: Option<String>,
    #[serde(rename = "startObjectCode")]
    pub start_object_code: String,
    #[serde(rename = "startInstanceCode")]
    pub start_instance_code: String,
    #[serde(rename = "endObjectCode")]
    pub end_object_code: String,
    #[serde(rename = "endInstanceCode")]
    pub end_instance_code: String,
    pub attributes: Vec<u8>,
}

impl DataCenterRelations {
    pub fn new(start_instance: &DataCenterInstance, end_instance: &DataCenterInstance) -> Self {
        DataCenterRelations {
            version: start_instance.version.clone(),
            object_model_code: "RELAPOPO".to_string(),
            instance_code: format!("RELAPOPO {}", start_instance.instance_code),
            operate: None,
            start_object_code: start_instance.object_model_code.clone(),
            start_instance_code: start_instance.instance_code.clone(),
            end_object_code: end_instance.object_model_code.clone(),
            end_instance_code: end_instance.instance_code.clone(),
            attributes: vec![],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataCenterRelationsHH {
    pub version: String,
    #[serde(rename = "objectModelCode")]
    pub object_model_code: String,
    #[serde(rename = "instanceCode")]
    pub instance_code: String,
    #[serde(serialize_with = "serialize_option_string_with_default")]
    pub operate: Option<String>,
    #[serde(rename = "startObjectCode")]
    pub start_object_code: String,
    #[serde(rename = "startInstanceCode")]
    pub start_instance_code: String,
    #[serde(rename = "endObjectCode")]
    pub end_object_code: String,
    #[serde(rename = "endInstanceCode")]
    pub end_instance_code: String,
    pub attributes: Vec<u8>,
}

impl DataCenterRelationsHH {
    pub fn new(start_instance: &DataCenterInstanceHH, end_instance: &DataCenterInstanceHH) -> Self {
        DataCenterRelationsHH {
            version: start_instance.version.clone(),
            object_model_code: "RELAPOPO".to_string(),
            instance_code: format!("RELAPOPO {}", start_instance.instance_code),
            operate: None,
            start_object_code: start_instance.object_model_code.clone(),
            start_instance_code: start_instance.instance_code.clone(),
            end_object_code: end_instance.object_model_code.clone(),
            end_instance_code: end_instance.instance_code.clone(),
            attributes: vec![],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Hash)]
pub struct DataCenterAttr {
    #[serde(rename = "attributeModelCode")]
    pub attribute_model_code: String,
    pub value: String,
    // pub value: T,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum AttrValue {
    Invalid,
    AttrString(String),
    AttrFloat(f32),
    AttrInt(i32),
    AttrBool(bool),
    AttrStrArray(Vec<String>),
    AttrIntArray(Vec<i32>),
    AttrFloatArray(Vec<f32>),
    AttrVec3(Vec3),
    AttrVec3Array(Vec<Vec3>),
    AttrMap(HashMap<String, Vec<String>>),
    AttrVecVecStringMap(HashMap<String, Vec<Vec<String>>>),
    AttrMapFloat(HashMap<String, f32>),
    AttrMapFloatArray(HashMap<String, Vec<f32>>),
    AttrItemArray(Vec<ItemValue>),
    AttrStringMap(HashMap<String, String>),
}

impl Default for AttrValue {
    fn default() -> Self {
        AttrString("".to_string())
    }
}

impl Into<String> for AttrValue {
    fn into(self) -> String {
        match self {
            AttrValue::Invalid => AttrValue::default().into(),
            AttrString(a) => a,
            AttrFloat(a) => a.to_string(),
            AttrValue::AttrInt(a) => a.to_string(),
            AttrValue::AttrBool(a) => {
                if a {
                    "Y".to_string()
                } else {
                    "N".to_string()
                }
            }
            AttrStrArray(a) => serde_json::to_string(&a).unwrap_or("[]".to_string()),
            AttrValue::AttrIntArray(a) => serde_json::to_string(&a).unwrap_or("[]".to_string()),
            AttrValue::AttrFloatArray(a) => serde_json::to_string(&a).unwrap_or("[]".to_string()),
            AttrValue::AttrVec3(a) => serde_json::to_string(&a).unwrap_or("[]".to_string()),
            AttrValue::AttrVec3Array(a) => serde_json::to_string(&a).unwrap_or("[]".to_string()),
            AttrValue::AttrMap(a) => serde_json::to_string(&a).unwrap_or("{}".to_string()),
            AttrValue::AttrVecVecStringMap(a) => {
                serde_json::to_string(&a).unwrap_or("{}".to_string())
            }
            AttrValue::AttrMapFloat(a) => serde_json::to_string(&a).unwrap_or("{}".to_string()),
            AttrValue::AttrMapFloatArray(a) => {
                serde_json::to_string(&a).unwrap_or("{}".to_string())
            }
            AttrValue::AttrItemArray(a) => serde_json::to_string(&a).unwrap_or("[]".to_string()),
            AttrValue::AttrStringMap(a) => serde_json::to_string(&a).unwrap_or("{}".to_string()),
        }
    }
}

impl AttrValue {
    pub fn to_string(self) -> String {
        self.into()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum ItemValue {
    String(String),
    Int(i32),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum HoleType {
    // 孔洞类
    STUCJ,
    // 钢制套管类
    STUCG,
    // 纤维水泥电缆导管类
    STUCH,
    // 槽类
    STUCK,
    // 地坑类
    STUCL,
    // 地漏类
    STUCM,
    // 贯穿件
    STUCD,
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ThreeDDatacenterRequest {
    pub title: String,
    pub refnos: Vec<String>,
    pub create_rvm_relations: bool,
    // 是否为初设
    #[serde(default)]
    pub b_first_time_design: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ThreeDDatacenterResponse {
    #[serde(rename = "Success")]
    pub success: bool,
    #[serde(rename = "Result")]
    pub result: String,
    #[serde(rename = "KeyValue")]
    pub key_value: String,
    #[serde(rename = "LoginUrl")]
    pub login_url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TubiData {
    pub pre_refno: RefU64,
    pub lstu_name: String,
    pub length: f32,
}

#[derive(Resource, Serialize, Deserialize, Clone, Debug, Default)]
pub struct SendHoleDataToArango {
    pub _key: String,
    #[serde(rename = "KeyValue")]
    pub key_value: String,
    #[serde(rename = "formdata")]
    pub form_data: SendHoleDataFormData,
}

///虚拟孔洞提资单数据
#[derive(Resource, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TiziVirtualHoleData {
    #[serde(rename = "id", alias = "KeyValue")]
    pub key_value: String,
    #[serde(rename = "formdata")]
    pub form_data: SendHoleDataFormData,
}

impl TiziVirtualHoleData {
    pub fn to_arango_struct(self) -> SendHoleDataToArango {
        SendHoleDataToArango {
            _key: self.key_value.clone(),
            key_value: self.key_value,
            form_data: self.form_data,
        }
    }

    pub fn to_publish_json(&self) -> anyhow::Result<String> {
        let mut obj = serde_json::to_value(self)?;
        if let serde_json::Value::Object(m) = &mut obj {
            let value = m.remove("id").unwrap();
            m.insert("KeyValue".into(), value);
        }
        Ok(serde_json::to_string(&obj)?)
    }
}

impl SendHoleDataToArango {
    pub fn to_ui_struct(self) -> TiziVirtualHoleData {
        TiziVirtualHoleData {
            key_value: self.key_value,
            form_data: self.form_data,
        }
    }
}

//提资列表
#[derive(Resource, Serialize, Deserialize, Clone, Debug, Default)]
pub struct AuditDataVec {
    pub data: Vec<SendHoleDataToArango>,
}

//可提资物资信息
#[derive(Resource, Default, Clone, Debug, Serialize, Deserialize, Component, PartialEq)]
pub struct VirtualHoleData {
    pub key: String,
    pub No: String,
    pub last_use_time: String,
    pub type_: String,
    pub major: String,
    pub description: String,
    pub operator: String,
    pub coord: String,
    pub remark: String,
    pub is_hole: bool,
    pub flag: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SendHoleDataFormData {
    #[serde(rename = "ProjCode")]
    pub project_code: String,
    #[serde(rename = "HumanCode")]
    pub human_code: String,
    #[serde(rename = "Title")]
    pub title: String,
    #[serde(rename = "Major")]
    pub major: String,
    #[serde(rename = "WXType")]
    pub wx_type: String,
    #[serde(rename = "JD_Name")]
    pub jd_name: String,
    #[serde(rename = "SH_Name")]
    pub sh_name: String,
    #[serde(rename = "SD_Name")]
    pub sd_name: String,
    #[serde(rename = "SZ_Name")]
    pub sz_name: String,
    #[serde(rename = "DeviseHum")]
    pub devise_hum: String,
    #[serde(rename = "OverruleHum")]
    pub overrule_hum: String,
    #[serde(rename = "Memo")]
    pub memo: String,
    #[serde(rename = "databody")]
    pub data_body: DataCenterProject,
    #[serde(rename = "modelbody")]
    pub model_body: Vec<HoleDataModelBody>,
    #[serde(rename = "Detail")]
    pub detail: Vec<DataCenterDetail>,
    #[serde(rename = "files")]
    pub files: Vec<DataCenterFile>,
    #[serde(rename = "ModelData")]
    pub model_data: Vec<Vec<(RefU64, String)>>,
    // pub model_data: HoleWallBoardVec,
}

//墙板列表
#[derive(Serialize, Deserialize, Default, Clone, Debug, Resource)]
pub struct HoleWallBoardVec {
    pub data: Vec<(RefU64, String)>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct HoleDataModelBody {
    pub code: String,
    pub status: bool,
    #[serde(rename = "JD")]
    pub jd: Vec<String>,
    #[serde(rename = "SH")]
    pub sh: Vec<String>,
    #[serde(rename = "SD")]
    pub sd: Vec<String>,
    #[serde(rename = "SZ")]
    pub sz: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataCenterFile {
    #[serde(rename = "filename")]
    pub file_name: String,
    #[serde(rename = "filestream")]
    pub file_stream: String,
}

impl DataCenterFile {
    pub fn from_file_bytes(file_bytes: FileBytes) -> Self {
        Self {
            file_name: file_bytes.file_name,
            file_stream: base64::encode(&file_bytes.data),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataCenterDetail {
    #[serde(rename = "Code")]
    pub code: String,
    #[serde(rename = "Type")]
    pub detail_type: String,
    #[serde(rename = "Major")]
    pub major: String,
    #[serde(rename = "ActExplain")]
    pub act_explain: String,
    #[serde(rename = "Posi")]
    pub position: String,
    #[serde(rename = "Memo")]
    pub memo: String,
    #[serde(rename = "Upddate")]
    pub update: String,
    #[serde(rename = "ActHum")]
    pub act_hum: String,
    pub is_hole: bool,
    pub key: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct CableWeight {
    pub types: String,
    pub width: String,
    /// 托盘重量
    pub tray_weight: String,
    /// 电缆线重
    pub cable_weight: String,
}

//接收创建虚拟孔洞流程的结构体
#[derive(Resource, Serialize, Deserialize, Clone, Debug, Default)]
pub struct ForwardHoleData {
    pub title: String,
    //孔洞或埋件的key
    pub hole_keys: Vec<String>,
    pub embed_keys: Vec<String>,
    pub jd_name: String,
    pub sh_name: String,
    pub sd_name: String,
    // #[serde(default)]
    // pub sz_name: String,
    pub human_code: String,
    pub memo: String,
}

#[test]
fn test_attr_json() {
    let _data = AttrStrArray(vec!["hello".to_string(), "world".to_string()]);
    let data = AttrFloat(1.0);
    let json = serde_json::to_string(&data).unwrap();
    dbg!(&json);
}

#[test]
fn test_item_value() {
    let item_1 = ItemValue::String("hello".to_string());
    let item_2 = ItemValue::Int(1);
    let r = vec![item_1, item_2];
    let data = serde_json::to_string(&r).unwrap();
    dbg!(&data);
}

/// 从恩为插件过来的原生孔洞数据
#[derive(Resource, Serialize, Deserialize, Clone, Debug, Default)]
pub struct RawHoleData {
    // node identifier
    #[serde(rename = "id", alias = "_key")]
    pub _key: String,
    // node link
    #[serde(rename = "RelyItem")]
    pub rely_item: String,
    // node main item
    #[serde(rename = "MainItem")]
    pub main_item: String,
    // node speciality
    #[serde(rename = "Speciality")]
    pub speciality: String,
    // node position
    #[serde(rename = "Position")]
    pub position: String,
    // node work
    #[serde(rename = "HoleWork")]
    pub hole_work: String,
    // node work by
    #[serde(rename = "WorkBy")]
    pub work_by: String,
    // node time
    #[serde(rename = "Time")]
    pub time: String,
    // node shape
    #[serde(rename = "Shape")]
    pub shape: String,
    // node orientation
    #[serde(rename = "Ori")]
    pub ori: String,
    // node item reference
    #[serde(rename = "ItemREF")]
    pub item_ref: String,
    #[serde(rename = "RelyItemREF")]
    pub rely_item_ref: String,
    // node main item reference
    #[serde(rename = "MainItemREF")]
    pub main_item_ref: String,
    // node open item
    #[serde(rename = "OpenItem")]
    pub open_item: String,
    // node plug type
    #[serde(rename = "PlugType")]
    pub plug_type: String,
    // node height
    #[serde(rename = "SizeHeight")]
    pub size_height: f32,
    // node width
    #[serde(rename = "SizeWidth")]
    pub size_width: f32,
    // node bank width
    #[serde(rename = "BankWidth")]
    pub bank_width: f32,
    // node bank height
    #[serde(rename = "BankHeight")]
    pub bank_height: f32,
    // node hot distance
    #[serde(rename = "HotDis")]
    pub hot_dis: String,
    // node heat thickness
    #[serde(rename = "HeatThick")]
    pub heat_thick: f32,
    // node reference number
    #[serde(rename = "refNo")]
    pub refno: String,
    // node fitting reference number
    #[serde(rename = "FittRefNo")]
    pub fitt_refno: String,
    // node subsurface material
    #[serde(rename = "SubsMaterial")]
    pub subs_material: String,
    // node subsurface thickness
    #[serde(rename = "SubsThickness")]
    pub subs_thickness: f32,
    // node create
    #[serde(rename = "iCreate")]
    pub i_create: i32,
    // node subsurface type
    #[serde(rename = "SubsType")]
    pub subs_type: String,
    // node extent length 1
    #[serde(rename = "ExtentLength1")]
    pub extent_length1: f32,
    // node extent length 2
    #[serde(rename = "ExtentLength2")]
    pub extent_length2: f32,
    // node second
    #[serde(rename = "Second")]
    pub second: bool,
    // node rehole
    #[serde(rename = "ReHole")]
    pub re_hole: i32,
    // node note
    #[serde(rename = "Note")]
    pub note: String,
    #[serde(rename = "SizeThrowWall")]
    pub size_throw_wall: f32,
    #[serde(rename = "HoleBPID")]
    pub hole_bpid: String,
    #[serde(rename = "HoleBPVER")]
    pub hole_bpver: String,
    #[serde(rename = "RelyItemBPID")]
    pub rely_item_bpid: String,
    #[serde(rename = "RelyItemBPVER")]
    pub rely_item_bpver: String,
    #[serde(rename = "MainPipeline")]
    pub main_pipeline: String,
    #[serde(rename = "iFlowState")]
    pub i_flow_state: String,
    #[serde(rename = "hType")]
    pub h_type: String,
    #[serde(rename = "MainItems")]
    pub main_items: String,
    #[serde(rename = "MainItemRefs")]
    pub main_item_refs: String,
    // 只用于存储和查询的数据，不涉及任何业务
    #[serde(flatten)]
    pub map: HashMap<String, String>,
}

impl RawHoleData {
    //todo 写一个proc macro来生成schema
    pub fn get_scheme() -> String {
        let basic_schema = generate_basic_versioned_schema::<Self>();
        format!(
            r#"{{
        "@type" : "Class",
        "@id"   : "VirtualHole",
        "@key"  : {{ "@type": "Lexical", "@fields": ["_key"] }},
        {}
        }}"#,
            basic_schema
        )
    }

    pub fn gen_versioned_data_json(&self) -> anyhow::Result<String> {
        let mut json_map = serde_json::to_value(self).unwrap();
        if let serde_json::Value::Object(m) = &mut json_map {
            m.insert("@id".into(), format!("VirtualHole/{}", self._key).into());
            m.insert("@type".into(), "VirtualHole".into());
        }
        Ok(serde_json::to_string(&json_map)?)
    }

    pub fn to_publish_json(&self) -> anyhow::Result<String> {
        let mut obj = serde_json::to_value(self)?;
        if let serde_json::Value::Object(m) = &mut obj {
            let value = m.remove("id").unwrap();
            m.insert("_key".into(), value);
        }
        Ok(serde_json::to_string(&obj)?)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum DataCenterRecordOperate {
    Insert,
    Modify,
    Delete,
}

pub const DATACENTER_VERSION: &'static str = "datacenter_version";

/// 发布成功后的元数据，只存放最小交付单元
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DataCenterRecord {
    pub refno: RefnoEnum,
    pub instance_code: String,
    // 删除节点所属的zone，不然删除后找不到相关的节点
    pub belong_zone: RefnoEnum,
    pub status: DataCenterRecordOperate,
    pub version: String,
}

impl DataCenterRecord {
    pub fn get_insert_sql(refnos: HashMap<RefnoEnum, String>) -> String {
        if refnos.is_empty() {
            return "".to_string();
        };
        let data = refnos
            .into_iter()
            .map(|(refno, instance_code)| DataCenterRecord {
                refno: refno.into(),
                instance_code: instance_code,
                status: DataCenterRecordOperate::Insert,
                belong_zone: Default::default(),
                version: "".to_string(),
            })
            .collect::<Vec<DataCenterRecord>>();
        data.into_iter()
            .map(|d| {
                format!(
                    "upsert {} set instance_code = '{}' , status = '{:?}'",
                    d.refno.to_table_key(DATACENTER_VERSION),
                    d.instance_code,
                    d.status
                )
            })
            .collect::<Vec<_>>()
            .join(";")
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DataCenterPlatResponse {
    #[serde(rename = "Success")]
    pub success: bool,
    #[serde(rename = "Result")]
    pub result: String,
    #[serde(rename = "KeyValue")]
    pub key_value: String,
    #[serde(rename = "LoginUrl")]
    pub login_url: String,
    #[serde(rename = "StatusCode")]
    pub status_code: u32,
}
