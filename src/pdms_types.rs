use crate::cache::mgr::BytesTrait;
use crate::consts::*;
use crate::geometry::{EleInstGeo, EleInstGeosData};
#[cfg(feature = "sea-orm")]
use crate::orm::*;
use crate::parsed_data::CateAxisParam;
use crate::pe::SPdmsElement;
use crate::shape::pdms_shape::BrepShapeTrait;
use crate::tool::db_tool::{db1_dehash, db1_hash};
use crate::types::attmap::AttrMap;
use crate::types::attval::{AttrVal, AttrValAql};
use crate::types::named_attvalue::NamedAttrValue;
pub use crate::types::*;
use bevy_ecs::prelude::*;
use bevy_math::*;
use bevy_reflect::Reflect;
#[cfg(feature = "render")]
use bevy_render::prelude::*;
use bevy_transform::prelude::*;
use dashmap::DashMap;
use derive_more::{Deref, DerefMut};
use id_tree::NodeId;
use itertools::Itertools;
use parry3d::bounding_volume::Aabb;
#[cfg(feature = "sea-orm")]
use sea_orm::entity::prelude::*;
#[cfg(feature = "sea-orm")]
use sea_query::*;
#[cfg(feature = "sea-orm")]
use sea_query::*;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use serde_repr::{Deserialize_repr, Serialize_repr};
use serde_with::{DisplayFromStr, serde_as};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::{Debug, Display, Pointer};
use std::io::{Read, Write};
use std::str::FromStr;
use std::string::ToString;
use surrealdb::types::{RecordId as Thing, SurrealValue};

///控制pdms显示的深度层级
pub const LEVEL_VISBLE: u32 = 6;

///非负实体基本体的种类
pub const PRIMITIVE_NOUN_NAMES: [&'static str; 8] = [
    "BOX", "CYLI", "SLCY", "CONE", "DISH", "CTOR", "RTOR", "PYRA",
];

///基本体几何体相关的属性
pub const PRIMITIVE_GEO_ATTR_NAMES: [&'static str; 8] = [
    "XLEN", "YLEN", "ZLEN", "XRAD", "YRAD", "ZRAD", "XANG", "YANG",
];

///基本体的种类(包含负实体)
//"SPINE", "GENS",
pub const GNERAL_PRIM_NOUN_NAMES: [&'static str; 22] = [
    "BOX", "CYLI", "SLCY", "CONE", "DISH", "CTOR", "RTOR", "PYRA", "SNOU", "POHE", "NBOX", "NCYL",
    "NSBO", "NCON", "NSNO", "NPYR", "NDIS", "NCTO", "NRTO", "NSCY", "NREV", "POLYHE",
];

///有loop的几何体
pub const GNERAL_LOOP_OWNER_NOUN_NAMES: [&'static str; 9] = [
    "AEXTR", "NXTR", "EXTR", "PANE", "FLOOR", "SCREED", "GWALL", "NREV", "REVO",
];

pub const USE_CATE_NOUN_NAMES: [&'static str; 35] = [
    "FIXING", "GENSEC", "SCREED", "CMPF", "GWALL", "EQUI", "ANCI", "FITT", "SJOI", "SBFI", "CABLE",
    "CNODE", "SCTN", "SCOJ", "PAVE", "SUBE", "SEVE", "SUBJ", "PLOO", "RNODE", "PJOI", "SELJ",
    "STWALL", "WALL", "PALJ", "TUBI", "FLOOR", "CMFI", "PANE", "PFIT", "GPART", "PRTELE", "NOZZ",
    "SPCO", "ELCONN",
];

///使用元件库的几何体相关的属性, todo 继续完善
pub const CATA_GEO_ATTR_NAMES: [&'static str; 4] = ["SPRE", "CATR", "ZDIS", "DESP"];

///方位的相关属性, todo 继续完善
pub const TRANSFORM_ATTR_NAMES: [&'static str; 4] = ["POS", "ORI", "POSS", "POSE"];

///管道的类型
pub const PIPING_NOUN_NAMES: [&'static str; 26] = [
    "WELD", "ELBO", "VALV", "FLAN", "GASK", "ATTA", "OLET", "FBLI", "REDU", "TEE", "BEND", "INST",
    "TRNS", "DAMP", "STRT", "TAPE", "THRE", "UNIO", "BRCO", "OFST", "CAP", "PCOM", "FTUB", "STIF",
    "SILE", "COUP",
];

///负实体基本体的种类
pub const GENRAL_NEG_NOUN_NAMES: [&'static str; 13] = [
    "NBOX", "NCYL", "NLCY", "NSBO", "NCON", "NSNO", "NPYR", "NDIS", "NXTR", "NCTO", "NRTO", "NREV",
    "NSCY",
];

///元件库的负实体类型
pub const CATE_NEG_NOUN_NAMES: [&'static str; 13] = [
    "NSBO", "NSCO", "NLSN", "NSSP", "NLCY", "NSCY", "NSCT", "NSRT", "NSDS", "NSSL", "NLPY", "NSEX",
    "NSRE",
];

pub const TOTAL_NEG_NOUN_NAMES: [&'static str; 26] = [
    "NBOX", "NCYL", "NLCY", "NSBO", "NCON", "NSNO", "NPYR", "NDIS", "NXTR", "NCTO", "NRTO", "NREV",
    "NSCY", "NSBO", "NSCO", "NLSN", "NSSP", "NLCY", "NSCY", "NSCT", "NSRT", "NSDS", "NSSL", "NLPY",
    "NSEX", "NSRE",
];

pub const TOTAL_VERT_NOUN_NAMES: [&'static str; 2] = ["VERT", "PAVE"];

pub const TOTAL_LOOP_NOUN_NAMES: [&'static str; 2] = ["LOOP", "PLOO"];

pub const JOINT_TYPES: [&'static str; 2] = ["SJOI", "PJOI"];

pub const GENRAL_POS_NOUN_NAMES: [&'static str; 25] = [
    "BOX", "CYLI", "SLCY", "CONE", "DISH", "CTOR", "RTOR", "PYRA", "SNOU", "FLOOR", "PANEL",
    "SBOX", "SCYL", "LCYL", "SSPH", "LCYL", "SCON", "LSNO", "LPYR", "SDSH", "SCTO", "SEXT", "SREV",
    "SRTO", "SSLC",
];

pub const TOTAL_GEO_NOUN_NAMES: [&'static str; 40] = [
    "BOX", "CYLI", "SLCY", "CONE", "DISH", "CTOR", "RTOR", "PYRA", "SNOU", "PLOO", "LOOP", "POHE",
    "SBOX", "SCYL", "SSPH", "LCYL", "SCON", "LSNO", "LPYR", "SDSH", "SCTO", "SEXT", "SREV", "SRTO",
    "SSLC", "SPRO", "SREC", "NBOX", "NCYL", "NLCY", "NSBO", "NCON", "NSNO", "NPYR", "NDIS", "NXTR",
    "NCTO", "NRTO", "NREV", "NSCY",
];

pub const TOTAL_CATA_GEO_NOUN_NAMES: [&'static str; 31] = [
    "SBOX", "SCYL", "SSPH", "LCYL", "SCON", "LSNO", "LPYR", "SDSH", "SCTO", "SEXT", "SREV", "SRTO",
    "SSLC", "SPRO", "SANN", "BOXI", "TUBE", "SREC", "NSBO", "NSCO", "NLSN", "NSSP", "NLCY", "NSCY",
    "NSCT", "NSRT", "NSDS", "NSSL", "NLPY", "NSEX", "NSRE",
];

///可能会与ngmr发生作用的类型
pub const TOTAL_CONTAIN_NGMR_GEO_NAEMS: [&'static str; 6] =
    ["WALL", "STWALL", "GWALL", "SCTN", "PANEL", "FLOOR"];

///POHE
pub const POHE_GEO_NAMES: [&'static str; 1] = ["POHE"];

///元件库的种类
pub const CATA_GEO_NAMES: [&'static str; 26] = [
    "BRAN", "HANG", "ELCONN", "CMPF", "WALL", "STWALL", "GWALL", "FIXING", "SJOI", "PJOI", "PFIT",
    "GENSEC", "RNODE", "PRTELE", "GPART", "SCREED", "NOZZ", "PALJ", "CABLE", "BATT", "CMFI",
    "SCOJ", "SEVE", "SBFI", "SCTN", "FITT",
];

///有tubi的类型
pub const CATA_HAS_TUBI_GEO_NAMES: [&'static str; 2] = ["BRAN", "HANG"];

///可以重用的类型
pub const CATA_SINGLE_REUSE_GEO_NAMES: [&'static str; 0] = [];

pub const CATA_WITHOUT_REUSE_GEO_NAMES: [&'static str; 24] = [
    "ELCONN", "CMPF", "WALL", "GWALL", "SJOI", "FITT", "PFIT", "FIXING", "PJOI", "GENSEC", "RNODE",
    "PRTELE", "GPART", "SCREED", "PALJ", "CABLE", "BATT", "CMFI", "SCOJ", "SEVE", "SBFI", "STWALL",
    "SCTN", "NOZZ",
];

pub const VISBILE_GEO_NOUNS: [&'static str; 39] = [
    "BOX", "CYLI", "SLCY", "CONE", "DISH", "CTOR", "RTOR", "PYRA", "SNOU", "POHE", "POLYHE",
    "EXTR", "REVO", "FLOOR", "PANE", "ELCONN", "CMPF", "WALL", "GWALL", "SJOI", "FITT", "PFIT",
    "FIXING", "PJOI", "GENSEC", "RNODE", "PRTELE", "GPART", "SCREED", "PALJ", "CABLE", "BATT",
    "CMFI", "SCOJ", "SEVE", "SBFI", "STWALL", "SCTN", "NOZZ",
];

#[derive(Serialize, Deserialize, Clone, Debug, Default, Copy, Eq, PartialEq, Hash)]
pub enum SjusType {
    #[default]
    UNSET,
    UTOP,
    UBOT,
    UCEN,
}

#[derive(Serialize, Deserialize, Debug)]
struct Jsgf {
    #[serde(with = "string")]
    u: u64,
    #[serde(with = "string")]
    i: i64,
}

pub mod string {
    use std::fmt::Display;
    use std::str::FromStr;

    use serde::{Deserialize, Deserializer, Serializer, de};

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Display,
        S: Serializer,
    {
        serializer.collect_str(value)
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: FromStr,
        T::Err: Display,
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifferenceValue {
    pub noun: String,
    pub old_value: Option<NamedAttrValue>,
    // 新增 old_value 为 none
    pub new_value: Option<NamedAttrValue>, // 删除 new_value 为 none
}

pub const DEFAULT_NOUNS: [NounHash; 4] = [TYPE_HASH, NAME_HASH, REFNO_HASH, OWNER_HASH];
pub const DEFAULT_NAMED_NOUNS: [&'static str; 4] = ["TYPE", "NAME", "REFNO", "OWNER"];

#[repr(C)]
#[derive(
    Component,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Serialize,
    Deserialize,
    Default,
    Clone,
    strum_macros::Display,
    strum_macros::EnumString,
    Debug,
    Copy,
    Eq,
    PartialEq,
    Hash,
)]
pub enum PdmsGenericType {
    #[default]
    UNKOWN = 0,
    CE,
    PIPE,
    STRU,
    EQUI,
    ROOM,
    SCTN,
    WALL,
    STWALL,
    CWALL,
    GWALL,
    GENSEC,
    HANG,
    HANDRA,
    PANE,
    CFLOOR,
    FLOOR,
    EXTR,
    CWBRAN,
    REVO,
    CTWALL,
    AREADEF,
    DEMOPA,
    INSURQ,
    STRLNG,
    HVAC,
}

fn de_from_str<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<u64>().map_err(de::Error::custom)
}

fn de_refno_from_str<'de, D>(deserializer: D) -> Result<RefU64, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    RefU64::from_str(&s).map_err(de::Error::custom)
}

fn de_hashset_from_str<'de, D>(deserializer: D) -> Result<HashSet<RefU64>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer).unwrap_or_default();
    Ok(serde_json::from_str::<HashSet<String>>(s.as_str())
        .unwrap_or_default()
        .into_iter()
        .map(|x| RefU64::from_str(x.as_str()).unwrap_or_default())
        .collect())
}

pub fn ser_hashset_as_str<S>(refnos: &HashSet<RefU64>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let set = refnos
        .into_iter()
        .map(|x| x.to_string())
        .collect::<HashSet<_>>();
    s.serialize_str(serde_json::to_string(&set).unwrap_or_default().as_str())
    // s.ser(&set)
}

pub fn ser_u64_as_str<S>(id: &u64, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str((*id).to_string().as_str())
}

pub fn ser_refno_as_str<S>(refno: &RefU64, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(refno.to_string().as_str())
}

pub trait PdmsNodeTrait: Default {
    #[inline]
    fn get_refno(&self) -> RefU64 {
        RefU64::default()
    }

    #[inline]
    fn get_id(&self) -> Option<&Thing> {
        None
    }

    #[inline]
    fn get_name(&self) -> &str {
        ""
    }

    #[inline]
    fn get_mod_cnt(&self) -> u32 {
        0
    }

    #[inline]
    fn get_status(&self) -> &str {
        ""
    }

    #[inline]
    fn get_noun_hash(&self) -> u32 {
        0
    }

    #[inline]
    fn get_type_name(&self) -> &str {
        ""
    }

    #[inline]
    fn get_children_count(&self) -> usize {
        0
    }

    #[inline]
    fn get_order(&self) -> usize {
        0
    }
}

pub const STATE_CODE_INIT: &'static str = "D00";

#[derive(Serialize, Deserialize, Clone, Debug, Default, surrealdb::types::SurrealValue)]
pub struct EleTreeNode {
    pub refno: RefnoEnum,
    pub noun: String,
    #[serde(default)]
    pub name: String,
    pub owner: RefnoEnum,
    #[serde(default)]
    pub order: u16,
    pub children_count: u16,
    #[serde(default)]
    #[surreal(wrap)]
    pub op: EleOperation,
    //修改次数
    pub mod_cnt: Option<u32>,
    #[serde(default)]
    pub children_updated: Option<bool>,
    pub status_code: Option<String>,
}

impl EleTreeNode {
    pub fn new(
        refno: RefnoEnum,
        noun: String,
        name: String,
        owner: RefnoEnum,
        order: u16,
        children_count: u16,
        op: EleOperation,
    ) -> Self {
        Self {
            refno,
            noun,
            name,
            owner,
            order,
            children_count,
            op,
            mod_cnt: None,
            children_updated: None,
            status_code: None,
        }
    }

    pub fn into_handle_struct(self) -> PdmsElementHandle {
        PdmsElementHandle {
            refno: self.refno.to_pdms_str(),
            owner: self.owner.to_pdms_str(),
            name: self.name,
            noun: self.noun,
            version: 0,
            children_count: self.children_count as _,
        }
    }

    #[inline]
    pub fn latest_refno(&self) -> RefU64 {
        self.refno.refno()
    }

    #[inline]
    pub fn latest_owner(&self) -> RefU64 {
        self.owner.refno()
    }
}

impl From<PdmsElement> for EleTreeNode {
    fn from(value: PdmsElement) -> Self {
        EleTreeNode {
            refno: value.refno.into(),
            noun: value.noun,
            name: value.name,
            owner: value.owner.into(),
            order: 0,
            children_count: value.children_count as _,
            op: EleOperation::Modified,
            mod_cnt: None,
            children_updated: None,
            status_code: None,
        }
    }
}

impl PdmsNodeTrait for EleTreeNode {
    #[inline]
    fn get_refno(&self) -> RefU64 {
        self.refno.refno()
    }

    #[inline]
    fn get_name(&self) -> &str {
        self.name.as_str()
    }

    #[inline]
    fn get_mod_cnt(&self) -> u32 {
        self.mod_cnt.unwrap_or_default()
    }

    #[inline]
    fn get_status(&self) -> &str {
        self.status_code.as_deref().unwrap_or_default()
    }

    #[inline]
    fn get_noun_hash(&self) -> u32 {
        db1_hash(&self.noun.to_uppercase())
    }

    #[inline]
    fn get_type_name(&self) -> &str {
        self.noun.as_str()
    }

    #[inline]
    fn get_children_count(&self) -> usize {
        self.children_count as _
    }

    #[inline]
    fn get_order(&self) -> usize {
        self.order as _
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct EleNode {
    pub refno: RefU64,
    pub owner: RefU64,
    pub name_hash: AiosStrHash,
    // pub name: AiosStr,
    pub noun: u32,
    pub version: u32,
    // pub children_count: usize,
    pub children_count: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ChildrenNode {
    pub refno: RefU64,
    pub name: String,
    pub noun: String,
}

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug, Default, Component)]
pub struct CataHashRefnoKV {
    #[serde(default)]
    pub cata_hash: String,
    #[serde(default)]
    pub group_refnos: Vec<RefnoEnum>,
    pub exist_inst: bool,
    pub ptset: Option<BTreeMap<i32, CateAxisParam>>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug, Default, Eq, PartialEq, Component)]
pub struct PdmsElement {
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "_key")]
    pub refno: RefU64,
    #[serde_as(as = "DisplayFromStr")]
    pub owner: RefU64,
    pub name: String,
    pub noun: String,
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub children_count: usize,
}

impl PdmsNodeTrait for PdmsElement {
    #[inline]
    fn get_refno(&self) -> RefU64 {
        self.refno
    }

    #[inline]
    fn get_name(&self) -> &str {
        self.name.as_str()
    }

    #[inline]
    fn get_noun_hash(&self) -> u32 {
        db1_hash(&self.noun.to_uppercase())
    }

    #[inline]
    fn get_type_name(&self) -> &str {
        self.noun.as_str()
    }

    #[inline]
    fn get_children_count(&self) -> usize {
        self.children_count
    }
}

impl PdmsElement {
    pub fn get_enso_headers() -> Vec<String> {
        vec![
            "refno".to_string(),
            "owner".to_string(),
            "name".to_string(),
            "noun".to_string(),
            "version".to_string(),
            "children_count".to_string(),
        ]
    }

    pub fn into_enso_value_json(self) -> Vec<NamedAttrValue> {
        vec![
            NamedAttrValue::StringType(self.refno.to_string()),
            NamedAttrValue::StringType(self.owner.to_string()),
            NamedAttrValue::StringType(self.name),
            NamedAttrValue::StringType(self.noun),
            NamedAttrValue::IntegerType(self.version as i32),
            NamedAttrValue::IntegerType(self.children_count as i32),
        ]
    }
    /// 转为对外接口的结构体
    pub fn into_handle_struct(self) -> PdmsElementHandle {
        PdmsElementHandle {
            refno: self.refno.to_pdms_str(),
            owner: self.owner.to_pdms_str(),
            name: self.name,
            noun: self.noun,
            version: self.version,
            children_count: self.children_count,
        }
    }
}

impl From<EleTreeNode> for PdmsElement {
    fn from(value: EleTreeNode) -> Self {
        Self {
            refno: value.refno.refno(),
            owner: value.owner.refno(),
            name: value.name,
            noun: value.noun,
            version: 0,
            children_count: value.children_count as usize,
        }
    }
}

impl From<SPdmsElement> for PdmsElement {
    fn from(value: SPdmsElement) -> Self {
        Self {
            refno: value.refno.refno(),
            owner: value.owner.refno(),
            name: value.name,
            noun: value.noun,
            version: 0,
            children_count: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Deref, DerefMut)]
pub struct PdmsElementVec(pub Vec<PdmsElement>);

impl BytesTrait for PdmsElementVec {}

impl EleNode {
    pub fn set_default_name(name_hash: AiosStrHash) -> EleNode {
        EleNode {
            name_hash,
            ..Default::default()
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PdmsNodeId(pub NodeId);

/// 每个dbno对应的version
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DbnoVersion {
    pub dbno: u32,
    pub version: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PdmsElementHandle {
    pub refno: String,
    pub owner: String,
    pub name: String,
    pub noun: String,
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub children_count: usize,
}

#[test]
fn test_dashmap() {
    let dashmap_1 = DashMap::new();
    dashmap_1.insert("1", "hello");
    let dashmap_2 = DashMap::new();
    dashmap_2.insert("2", "world");
    let dashmap_3 = DashMap::new();
    dashmap_1.iter().for_each(|m| {
        dashmap_3.insert(m.key().clone(), m.value().clone());
    });
    dashmap_2.iter().for_each(|m| {
        dashmap_3.insert(m.key().clone(), m.value().clone());
    });
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DbAttributeType {
    #[default]
    Unknown,
    INTEGER = 1,
    DOUBLE,
    BOOL,
    STRING,
    ELEMENT,
    WORD,
    DIRECTION,
    POSITION,
    ORIENTATION,
    DATETIME,

    //todo remove these
    DOUBLEVEC,
    INTVEC,
    FLOATVEC,
    TYPEX,
    Vec3Type,
    RefU64Vec,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AttrInfo {
    pub name: String,
    pub hash: i32,
    pub offset: u32,
    pub default_val: AttrVal,
    pub att_type: DbAttributeType,
    /// core.dll `DB_Attribute::ityp`（`0x58d2630`）——与 `att_type` 不同的一维分类，
    /// 51/52 表示「值本身是一个属性键 / noun 键」，core 在读路径上对这两类走
    /// `DB_Uda::oldToNew` 做旧键归一化（ADR-032 边界 B）。
    ///
    /// `None` = **尚未采集**，不是「取值为 0」：现有 `all_attr_info.json` 里一个都没有，
    /// 采下来之前任何按 51/52 分派的逻辑都必须把 `None` 当作「不知道」而非「不是」。
    #[serde(default)]
    pub ityp: Option<i32>,
}

impl AttrInfo {
    fn gen_value_schema(&self) -> Option<&str> {
        match self.default_val {
            AttrVal::IntegerType(_) => {
                if self.offset > 0 {
                    Some(
                        r#"
                         "xsd:integer"
                    "#,
                    )
                } else {
                    Some(
                        r#"{
                        "@class": "xsd:integer",
                        "@type": "Optional"
                    }"#,
                    )
                }
            }
            AttrVal::BoolType(_) => {
                if self.offset > 0 {
                    Some(
                        r#"
                        "xsd:boolean"
                    "#,
                    )
                } else {
                    Some(
                        r#"{
                        "@class": "xsd:boolean",
                        "@type": "Optional"
                    }"#,
                    )
                }
            }
            AttrVal::DoubleType(_) => {
                if self.offset > 0 {
                    Some(
                        r#"
                        "xsd:decimal"
                    "#,
                    )
                } else {
                    Some(
                        r#"{
                        "@class": "xsd:decimal",
                        "@type": "Optional"
                    }"#,
                    )
                }
            }
            //element 暂时为string，需要转换成type class
            AttrVal::StringType(_)
            | AttrVal::RefU64Type(_)
            | AttrVal::WordType(_)
            | AttrVal::ElementType(_) => {
                if self.offset > 0 {
                    Some(
                        r#"
                        "xsd:string"
                    "#,
                    )
                } else {
                    Some(
                        r#"{
                        "@class": "xsd:string",
                        "@type": "Optional"
                    }"#,
                    )
                }
            }
            AttrVal::DoubleArrayType(_) | AttrVal::Vec3Type(_) => Some(
                r#"{
                        "@class": "xsd:decimal",
                        "@type": "Array"
                    }"#,
            ),
            AttrVal::IntArrayType(_) => Some(
                r#"
                     {
                        "@class": "xsd:integer",
                        "@type": "Array"
                     }
                "#,
            ),
            AttrVal::BoolArrayType(_) => Some(
                r#"
                     {
                        "@class": "xsd:boolean",
                        "@type": "Array"
                     }
                "#,
            ),

            // DbAttributeType::ELEMENT => {
            //     Some(r#"{
            //             "@class": "xsd:string",
            //             "@type": "Optional"
            //         }"#)
            // }
            _ => None,
        }
    }

    pub fn gen_schema(&self) -> Option<String> {
        if let Some(s) = self.gen_value_schema() {
            let name = db1_dehash(self.hash as _);
            Some(format!(r#""{}": {}"#, &name, s))
        } else {
            None
        }
    }

    ///需要考虑
    pub fn gen_schema_old(&self) -> Option<String> {
        match self.att_type {
            DbAttributeType::INTEGER => Some(format!(r#""{}": "xsd:integer""#, &self.name)),
            DbAttributeType::DOUBLE => Some(format!(r#""{}": "xsd:decimal""#, &self.name)),
            DbAttributeType::BOOL => Some(format!(r#""{}": "xsd:bool""#, &self.name)),
            DbAttributeType::STRING | DbAttributeType::TYPEX | DbAttributeType::WORD => {
                Some(format!(r#""{}": "xsd:string""#, &self.name))
            }
            DbAttributeType::DIRECTION
            | DbAttributeType::POSITION
            | DbAttributeType::ORIENTATION
            | DbAttributeType::DOUBLEVEC
            | DbAttributeType::FLOATVEC
            | DbAttributeType::Vec3Type => Some(format!(
                r#""{}":
                     {{
                        "@class": "xsd:decimal",
                        "@type": "Array"
                    }}
                "#,
                &self.name
            )),
            DbAttributeType::INTVEC => Some(format!(
                r#""{}":
                     {{
                        "@class": "xsd:integer",
                        "@type": "Array"
                    }}
                "#,
                &self.name
            )),
            DbAttributeType::ELEMENT => Some(format!(r#""{}": "xsd:string""#, &self.name)),
            // DbAttributeType::RefU64Vec => {}
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PDMSDBInfo {
    pub name: String,
    pub db_no: i32,
    pub db_type: String,
    pub version: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PdmsRefno {
    pub ref_no: String,
    pub db: String,
    pub type_name: String,
}

pub type AiosStrHash = u32;

#[derive(
    Debug,
    Clone,
    Default,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct AiosStr(pub String);

impl AiosStr {
    #[inline]
    pub fn get_u32_hash(&self) -> u32 {
        use hash32::{FnvHasher, Hasher};
        use std::hash::Hash;
        let mut fnv = FnvHasher::default();
        self.hash(&mut fnv);
        fnv.finish32()
    }
    pub fn take(self) -> String {
        self.0
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Deref for AiosStr {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefnoNodeId {
    pub refno: u64,
    //  参考号对应的小版本
    pub version: u32,
    // 参考号在树中对应的nodeId
    pub node_id: NodeId,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Component)]
pub struct ProjectDbno {
    pub mdb: u32,
    pub main_db: u32,
    // 每个模块（DESI,CATA .. ）对应得dbno
    pub dbs: HashMap<String, Vec<u32>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxXt {
    pub map: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct YkGd {
    pub map: HashMap<String, String>,
}

/// 每种 type 对应的所有 uda name 和 default value
#[derive(Debug, Serialize, Deserialize)]
pub struct Uda {
    pub reference_type: String,
    pub data: Vec<(String, String)>,
}

/// 数据状态对应的数据结构
#[derive(Default, Clone, Debug, Serialize, Deserialize, Component)]
pub struct DataState {
    pub refno: RefU64,
    pub att_type: String,
    pub name: String,
    pub state: String,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize, Component)]
pub struct DataStateVec {
    pub data_states: Vec<DataState>,
}

/// 数据状态需要显示的pdms属性
#[derive(Default, Clone, Debug, Serialize, Deserialize, Component)]
pub struct DataScope {
    pub refno: RefU64,
    pub att_type: String,
    pub name: String,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize, Component)]
pub struct DataScopeVec {
    pub data_scopes: Vec<DataScope>,
}

unsafe impl Send for DataScopeVec {}

unsafe impl Sync for DataScopeVec {}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IncrementDataSql {
    pub id: String,
    pub refno: RefU64,
    pub operate: EleOperation,
    pub version: u32,
    pub user: String,
    pub old_data: AttrMap,
    pub new_data: AttrMap,
    pub time: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum UdaMajorType {
    /// 工艺
    T,
    /// 通风
    V,
    /// 电气
    E,
    /// 仪控
    I,
    /// 核岛水工
    W,
    /// BOP-暖通
    N,
    /// BOP-水工
    Z,
    /// 通信
    K,
    /// 设备
    S,
    /// 照明
    L,
    /// 辐射安全
    F,
    /// 反应堆热工水力
    H,
    /// 辐射监测
    R,
    /// 建筑
    A,
    /// 结构
    J,
    /// NPIC管道
    P,
    /// NPIC设备
    B,
    /// NPIC电气
    C,
    /// NPIC仪表
    Y,
    /// 多专业
    X,

    NULL,
}

impl UdaMajorType {
    pub fn from_str(input: &str) -> Self {
        match input.to_uppercase().as_str() {
            "T" => Self::T,
            "V" => Self::V,
            "E" => Self::E,
            "I" => Self::I,
            "W" => Self::W,
            "N" => Self::N,
            "Z" => Self::Z,
            "K" => Self::K,
            "S" => Self::S,
            _ => Self::NULL,
        }
    }

    pub fn to_major_str(&self) -> String {
        match self {
            UdaMajorType::T => "T".to_string(),
            UdaMajorType::V => "V".to_string(),
            UdaMajorType::E => "E".to_string(),
            UdaMajorType::I => "I".to_string(),
            UdaMajorType::W => "W".to_string(),
            UdaMajorType::N => "N".to_string(),
            UdaMajorType::Z => "Z".to_string(),
            UdaMajorType::K => "K".to_string(),
            UdaMajorType::S => "S".to_string(),
            UdaMajorType::L => "L".to_string(),
            UdaMajorType::F => "F".to_string(),
            UdaMajorType::H => "H".to_string(),
            UdaMajorType::R => "R".to_string(),
            UdaMajorType::A => "A".to_string(),
            UdaMajorType::J => "J".to_string(),
            UdaMajorType::P => "P".to_string(),
            UdaMajorType::B => "B".to_string(),
            UdaMajorType::C => "C".to_string(),
            UdaMajorType::Y => "Y".to_string(),
            UdaMajorType::X => "X".to_string(),
            UdaMajorType::NULL => "NULL".to_string(),
        }
    }

    pub fn to_chinese_name(&self) -> String {
        match self {
            UdaMajorType::T => "工艺".to_string(),
            UdaMajorType::V => "通风".to_string(),
            UdaMajorType::E => "电气".to_string(),
            UdaMajorType::I => "仪控".to_string(),
            UdaMajorType::W => "给排水".to_string(),
            UdaMajorType::N => "BOP暖".to_string(),
            UdaMajorType::Z => "BOP水".to_string(),
            UdaMajorType::K => "通信".to_string(),
            UdaMajorType::S => "设备".to_string(),
            UdaMajorType::L => "照明".to_string(),
            UdaMajorType::F => "辐射安全".to_string(),
            UdaMajorType::H => "反应堆热工水力".to_string(),
            UdaMajorType::R => "辐射监测".to_string(),
            UdaMajorType::A => "建筑".to_string(),
            UdaMajorType::J => "结构".to_string(),
            UdaMajorType::P => "NPIC管道".to_string(),
            UdaMajorType::B => "NPIC设备".to_string(),
            UdaMajorType::C => "NPIC电气".to_string(),
            UdaMajorType::Y => "NPIC仪表".to_string(),
            UdaMajorType::X => "多专业".to_string(),
            UdaMajorType::NULL => "未知".to_string(),
        }
    }

    pub fn from_chinese_description(input: &str) -> Self {
        match input {
            "管道" | "工艺" => Self::T,
            "电气" => Self::E,
            "设备" => Self::S,
            "通风" => Self::V,
            "仪控" => Self::I,
            "照明" => Self::L,
            "通信" => Self::K,
            "给排水" => Self::W,
            "暖通" => Self::N,
            "辐射安全" => Self::F,
            "反应堆热工水力" => Self::H,
            "辐射监测" => Self::R,
            "建筑" => Self::A,
            "结构" => Self::J,
            "BOP水" => Self::Z,
            "BOP暖" => Self::N,
            "NPIC管道" => Self::P,
            "NPIC设备" => Self::B,
            "NPIC电气" => Self::C,
            "NPIC仪表" => Self::Y,
            "多专业" => Self::X,
            _ => Self::NULL,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PdmsAttrArangodb {
    pub _key: String,
    #[serde(flatten)]
    pub map: HashMap<String, AttrValAql>,
}

/// 参考号属于哪个房间
#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PdmsNodeBelongRoomName {
    #[serde_as(as = "DisplayFromStr")]
    pub refno: RefU64,
    pub room_name: String,
}

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PdmsNameBelongRoomName {
    #[serde_as(as = "DisplayFromStr")]
    pub refno: RefU64,
    #[serde_as(as = "DisplayFromStr")]
    pub name: String,
    pub room_name: String,
}

/// 房间下的所有节点
#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RoomNodes {
    #[serde_as(as = "DisplayFromStr")]
    pub room_name: String,
    pub nodes: Vec<String>,
}

#[derive(PartialEq, Debug, Default, Clone, Copy, Serialize_repr, Deserialize_repr)]
#[repr(i32)]
pub enum EleOperation {
    #[default]
    Add = 0,
    Modified = 1,
    GeometryModified = 2,
    Deleted = 3,
    Duplicate = 4,
    None = 5,
}

impl EleOperation {
    pub fn into_num(&self) -> i32 {
        match &self {
            EleOperation::Add => 0,
            EleOperation::Modified => 1,
            EleOperation::GeometryModified => 2,
            EleOperation::Deleted => 3,
            EleOperation::Duplicate => 4,
            EleOperation::None => 5,
        }
    }
}

impl From<i32> for EleOperation {
    fn from(v: i32) -> Self {
        match v {
            0 => Self::Add,
            1 => Self::Modified,
            2 => Self::Deleted,
            3 => Self::Duplicate,
            _ => Self::None,
        }
    }
}

impl ToString for EleOperation {
    fn to_string(&self) -> String {
        match &self {
            Self::None => "未知".to_string(),
            EleOperation::Add => "增加".to_string(),
            EleOperation::Modified | EleOperation::GeometryModified => "修改".to_string(),
            EleOperation::Deleted => "删除".to_string(),
            EleOperation::Duplicate => "复制".to_string(),
        }
    }
}
