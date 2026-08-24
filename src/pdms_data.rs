use crate::PdmsDatabaseInfo;
use crate::cache::mgr::BytesTrait;
use crate::pdms_types::*;
use crate::tool::db_tool::db1_dehash;
use crate::types::attmap::AttrMap;
use crate::types::*;
use dashmap::mapref::one::Ref;
use dashmap::{DashMap, DashSet};
use glam::{DVec3, Vec3};
use itertools::Itertools;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::fmt::{Debug, Formatter};
use std::ops::Deref;

///元件库信息
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ScomInfo {
    pub attr_map: NamedAttrMap,
    pub gtype: String,
    pub dtse_params: Vec<DatasetParamStr>,
    ///几何体信息
    pub gm_params: Vec<GmParam>,
    ///和design发生运算的负实体信息
    pub ngm_params: Vec<GmParam>,
    pub axis_params: Vec<AxisParam>,
    pub params: String,
    pub axis_param_numbers: Vec<i32>,
    pub plin_map: HashMap<String, PlinParam>,
}

impl BytesTrait for ScomInfo {}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PlinParam {
    pub vxy: [String; 2],
    pub dxy: [String; 2],
    pub plax: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PlinParamData {
    pub pt: DVec3,
    pub plax: DVec3,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DatasetParamStr {
    pub refno: String,
    pub name: String,
    pub self_type: String,
    pub lock: bool,
    pub owner: String,
    pub description: String,
    pub dkey: String,
    pub ptype: String,
    pub pproperty: String,
    pub dproperty: String,
    pub purpose: String,
    pub number: i32,
    pub dtitle: String,
    pub punits: String,
    pub ruse: String,
    pub lhide: bool,
}

//还是要用枚举，来列举各个情况
//GMSE GMSS
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GmParam {
    pub refno: RefnoEnum,
    /// SCYL  LSNO  SCTO  SDSH  SBOX
    pub gm_type: String, //SCYL  LSNO  SCTO  SDSH  SBOX  SANN  SPRO

    pub prad: String,
    pub pang: String,
    pub pwid: String,
    /// 顺序 pdiameter pbdiameter ptdiameter, 先bottom, 后top
    pub diameters: Vec<String>,
    /// 顺序 pdistance pbdistance ptdistance, 先bottom, 后top
    pub distances: Vec<String>,
    pub shears: Vec<String>,
    pub phei: String,
    pub offset: String,
    /// 顺序 x y z
    pub lengths: Vec<String>,
    pub xyz: Vec<String>,

    /// profile  SPVE   SANN(PX, PY)
    pub verts: Vec<[String; 3]>,
    /// SANN: dx dy dradius dwidth
    pub dxy: Vec<[String; 2]>,
    pub drad: String,
    pub dwid: String,
    /// 顺序 paxis pa_axis pb_axis pc_axis
    pub paxises: Vec<String>,
    pub centre_line_flag: bool,
    pub visible_flag: bool,
    pub frads: Vec<String>,

    pub plax: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AxisParam {
    pub refno: RefnoEnum,
    pub type_name: String,
    pub number: i32,
    pub x: String,
    pub y: String,
    pub z: String,
    pub distance: String,
    pub direction: String,
    // pub dir_flag: bool,
    pub ref_direction: String,
    pub pconnect: String,
    pub pbore: String,
    pub pwidth: String,
    pub pheight: String,
    pub pnt_index_str: Option<String>,
}

/// 增量更新的数据操作
#[derive(Debug, Clone, Serialize, Deserialize, strum_macros::Display, strum_macros::EnumString)]
pub enum DataOperation {
    Modified = 0,
    Added = 1,
    Deleted = 2,
    Invalid = 3,
}

impl From<i32> for DataOperation {
    fn from(v: i32) -> Self {
        match v {
            0 => Self::Modified,
            1 => Self::Added,
            2 => Self::Deleted,
            _ => Self::Invalid,
        }
    }
}

impl Into<i32> for DataOperation {
    fn into(self) -> i32 {
        match self {
            DataOperation::Modified => 0,
            DataOperation::Added => 1,
            DataOperation::Deleted => 2,
            DataOperation::Invalid => 3,
        }
    }
}

impl DataOperation {
    pub fn into_str(self) -> String {
        match self {
            DataOperation::Modified => "修改".to_string(),
            DataOperation::Added => "新增".to_string(),
            DataOperation::Deleted => "删除".to_string(),
            DataOperation::Invalid => "未定义".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct IncrementData {
    pub refno: RefnoEnum,
    pub attr_data_map: AttrMap,
    pub state: DataOperation,
    pub version: u32,
}

impl Debug for IncrementData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IncrementData")
            .field("refno", &self.refno.to_string())
            .field("map", &self.attr_data_map.to_string_hashmap())
            .field("state", &self.state)
            .field("version", &self.version)
            .finish()
    }
}

lazy_static! {
    pub static ref ATTR_INFO_MAP: AttInfoMap = {
        let db_info: PdmsDatabaseInfo = serde_json::from_str(include_str!("../all_attr_info.json")).unwrap();
        //调用方法
        let mut att_info_map = AttInfoMap{
            map: db_info.noun_attr_info_map,
            type_att_names_map: Default::default(),
            type_implicit_att_names_map: Default::default(),
            type_explicit_att_names_map: Default::default(),
            att_name_type_map: Default::default(),
            has_cat_ref_types_set: Default::default(),
        };
        att_info_map.init_type_att_names_map();
        att_info_map
    };
}

#[derive(Default, Debug, Clone)]
pub struct AttInfoMap {
    pub map: DashMap<i32, DashMap<i32, AttrInfo>>,
    pub type_att_names_map: DashMap<String, BTreeSet<String>>,
    pub type_implicit_att_names_map: DashMap<String, BTreeSet<String>>,
    pub type_explicit_att_names_map: DashMap<String, BTreeSet<String>>,
    pub att_name_type_map: DashMap<String, DbAttributeType>,
    pub has_cat_ref_types_set: DashSet<String>,
}

impl Deref for AttInfoMap {
    type Target = DashMap<i32, DashMap<i32, AttrInfo>>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl AttInfoMap {
    #[inline]
    pub fn init_type_att_names_map(&mut self) {
        for k in &self.map {
            let type_name = db1_dehash(*k.key() as u32);
            for v in k.value() {
                self.type_att_names_map
                    .entry(type_name.clone())
                    .or_insert(BTreeSet::new())
                    .insert(v.name.to_string());
                if v.offset > 0 {
                    self.type_implicit_att_names_map
                        .entry(type_name.clone())
                        .or_insert(BTreeSet::new())
                        .insert(v.name.to_string());
                } else {
                    if ["ID", "REFNO", "TYPE", "OWNER"].contains(&v.name.as_str()) {
                        continue;
                    }
                    self.type_explicit_att_names_map
                        .entry(type_name.clone())
                        .or_insert(BTreeSet::new())
                        .insert(v.name.to_string());
                }
                self.att_name_type_map
                    .insert(v.name.to_string(), v.att_type);
                if v.name.as_str() == "CATR" || v.name.as_str() == "SPRE" {
                    self.has_cat_ref_types_set.insert(type_name.clone());
                }
            }
        }
    }

    /// 有元件库的类型
    #[inline]
    pub fn get_has_cat_ref_types_set(&self) -> &DashSet<String> {
        &self.has_cat_ref_types_set
    }

    /// 获取有catref的类型
    #[inline]
    pub fn get_has_cat_ref_type_names(&self) -> Vec<String> {
        self.get_has_cat_ref_types_set()
            .iter()
            .map(|x| x.clone())
            .collect::<Vec<_>>()
    }

    #[inline]
    pub fn get_type_implicit_att_names(&self, type_name: &str) -> Vec<String> {
        self.type_implicit_att_names_map
            .get(type_name)
            .map(|v| v.value().iter().cloned().collect_vec())
            .unwrap_or_default()
    }

    #[inline]
    pub fn get_type_explicit_att_names(&self, type_name: &str) -> Vec<String> {
        self.type_explicit_att_names_map
            .get(type_name)
            .map(|v| v.value().iter().cloned().collect_vec())
            .unwrap_or_default()
    }

    #[inline]
    pub fn get_names_map(&self) -> &DashMap<String, BTreeSet<String>> {
        &self.type_att_names_map
    }

    #[inline]
    pub fn get_names_of_type(&self, type_name: &str) -> Option<Ref<String, BTreeSet<String>>> {
        self.type_att_names_map.get(type_name)
    }

    #[inline]
    pub fn get_names_vec_of_type(&self, type_name: &str) -> Vec<String> {
        self.type_att_names_map
            .get(type_name)
            .map(|x| x.value().iter().map(|x| x.clone()).sorted().collect_vec())
            .unwrap_or_default()
    }

    #[inline]
    pub fn exist_att_by_name(&self, type_name: &str, att_name: &str) -> bool {
        self.type_att_names_map
            .get(type_name)
            .map(|x| x.contains(att_name))
            .unwrap_or(false)
    }

    /// 至少有一个 name 存在
    #[inline]
    pub fn exist_least_one_att_by_names(&self, type_name: &str, att_names: &Vec<&str>) -> bool {
        self.type_att_names_map
            .get(type_name)
            .map(|x| att_names.iter().any(|v| x.value().contains(*v)))
            .unwrap_or(false)
    }

    #[inline]
    pub fn get_val_type_of_att(&self, att_name: &str) -> Option<Ref<String, DbAttributeType>> {
        self.att_name_type_map.get(att_name)
    }
}
