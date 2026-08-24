use crate::cache::mgr::BytesTrait;
use crate::consts::{UNSET_STR, WORD_HASH};
use crate::helper::normalize_sql_string;
#[cfg(feature = "sea-orm")]
use crate::orm::{BoolVec, F32Vec, I32Vec, StringVec};
use crate::pe::SPdmsElement;
use crate::prim_geo::cylinder::SCylinder;
use crate::prim_geo::*;
use crate::shape::pdms_shape::BrepShapeTrait;
use crate::tool::db_tool::{db1_dehash, db1_hash};
use crate::tool::float_tool::*;
use crate::tool::math_tool::*;
use crate::types::attmap::AttrMap;
use crate::types::named_attvalue::NamedAttrValue;
use crate::{
    AttrVal, RefI32Tuple, RefU64, SurlStrand, SurlValue, cal_ori_by_extru_axis,
    cal_ori_by_z_axis_ref_x, cal_ori_by_z_axis_ref_y, get_default_pdms_db_info,
};
use crate::{pdms_types::*, query_refno_sesno};
use bevy_ecs::component::Component;
use bevy_reflect::{DynamicStruct, Reflect};
use derive_more::{Deref, DerefMut};
use glam::{Affine3A, DMat3, DQuat, DVec3, Mat3, Mat4, Quat, Vec3};
use indexmap::IndexMap;
#[cfg(feature = "sea-orm")]
use sea_orm::{ConnectionTrait, DatabaseConnection};
#[cfg(feature = "sea-orm")]
use sea_query::{Alias, MysqlQueryBuilder};
use serde_derive::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use surrealdb::sql::{Id, Thing};

///带名称的属性map
#[derive(
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Serialize,
    Deserialize,
    Deref,
    DerefMut,
    Clone,
    Default,
    Debug,
    Component,
    // Eq,
    PartialEq,
)]
pub struct NamedAttrMap {
    #[serde(flatten)]
    pub map: BTreeMap<String, NamedAttrValue>,
}

impl NamedAttrMap {
    pub async fn build_refno_sesno_map(
        &self,
        sesno: u32,
        dbnum: i32,
    ) -> anyhow::Result<HashMap<RefU64, u32>> {
        let mut refno_sesno_map = HashMap::new();
        for (_, value) in &self.map {
            if let NamedAttrValue::RefU64Type(refno) = value {
                if let Ok((sesno, latest_sesno)) = query_refno_sesno(*refno, sesno, dbnum).await {
                    if latest_sesno != sesno {
                        refno_sesno_map.insert(*refno, sesno);
                    }
                }
            } else if let NamedAttrValue::RefU64Array(refnos) = value {
                for &refno_enum in refnos {
                    let refno = refno_enum.refno();
                    if let Ok((sesno, latest_sesno)) = query_refno_sesno(refno, sesno, dbnum).await
                    {
                        if latest_sesno != sesno {
                            refno_sesno_map.insert(refno, sesno);
                        }
                    }
                }
            }
        }
        Ok(refno_sesno_map)
    }
}

impl BytesTrait for NamedAttrMap {}

impl From<SurlValue> for NamedAttrMap {
    fn from(s: SurlValue) -> Self {
        let mut map = BTreeMap::default();
        //需要根据类型来判断转换成相应的类型
        if let surrealdb::sql::Value::Object(o) = s {
            if let Some(SurlValue::Strand(name)) = o.get("TYPE") {
                let type_name = name.to_string().clone();
                let db_info = get_default_pdms_db_info();
                {
                    for (k, v) in o.0 {
                        // if k.as_str() == "WELDTY" {
                        //     dbg!(&k);
                        // }
                        //refno的页数号也要获取出来
                        if k == "PGNO" {
                            map.insert(
                                k.clone(),
                                NamedAttrValue::IntegerType(v.try_into().unwrap_or_default()),
                            );
                            continue;
                        } else if k == "SESNO" {
                            map.insert(
                                k.clone(),
                                NamedAttrValue::IntegerType(v.try_into().unwrap_or_default()),
                            );
                            continue;
                        }
                        let default_val = if k == "REFNO" || k == "OWNER" {
                            AttrVal::RefU64Type(Default::default())
                        } else if k == "TYPE" {
                            AttrVal::StringType(Default::default())
                        } else if let Some(val) = db_info
                            .named_attr_info_map
                            .get(&type_name)
                            .map(|m| m.get(&k).map(|x| x.value().clone()))
                            .flatten()
                        {
                            val.default_val
                        } else {
                            //通过 UDA 去查询这个变量的类型
                            continue;
                        };
                        let named_value = match default_val {
                            crate::AttrVal::IntegerType(_) => {
                                NamedAttrValue::IntegerType(v.try_into().unwrap_or_default())
                            }
                            crate::AttrVal::StringType(_) => {
                                NamedAttrValue::StringType(v.try_into().unwrap_or_default())
                            }
                            crate::AttrVal::WordType(_) => {
                                // dbg!((&k, &v));
                                let named_value = match &v {
                                    SurlValue::Strand(s) => NamedAttrValue::WordType(s.to_string()),
                                    SurlValue::Number(i) => NamedAttrValue::WordType(db1_dehash(
                                        v.try_into().unwrap_or_default(),
                                    )),
                                    _ => NamedAttrValue::WordType("".to_string()), // Default case if type is unexpected
                                };
                                named_value
                            }
                            crate::AttrVal::DoubleType(_) => {
                                NamedAttrValue::F32Type(v.try_into().unwrap_or_default())
                            }
                            crate::AttrVal::DoubleArrayType(_) => {
                                let v: Vec<surrealdb::sql::Value> =
                                    v.try_into().unwrap_or_default();
                                NamedAttrValue::F32VecType(
                                    v.into_iter()
                                        .map(|x| f32::try_from(x).unwrap_or_default())
                                        .collect(),
                                )
                            }
                            crate::AttrVal::Vec3Type(_) => {
                                let v: Vec<surrealdb::sql::Value> =
                                    v.try_into().unwrap_or_default();
                                let p = v
                                    .into_iter()
                                    .map(|x| f32::try_from(x).unwrap_or_default())
                                    .collect::<Vec<_>>();
                                if p.len() < 3 {
                                    //如果不够3个，就补0，错误处理？
                                    NamedAttrValue::Vec3Type(Vec3::ZERO)
                                } else {
                                    NamedAttrValue::Vec3Type(Vec3::new(p[0], p[1], p[2]))
                                }
                            }
                            crate::AttrVal::StringArrayType(_) => {
                                let v: Vec<surrealdb::sql::Value> =
                                    v.try_into().unwrap_or_default();
                                NamedAttrValue::StringArrayType(
                                    v.into_iter()
                                        .map(|x| String::try_from(x).unwrap_or_default())
                                        .collect(),
                                )
                            }
                            crate::AttrVal::BoolArrayType(_) => {
                                let v: Vec<surrealdb::sql::Value> =
                                    v.try_into().unwrap_or_default();
                                NamedAttrValue::BoolArrayType(
                                    v.into_iter()
                                        .map(|x| bool::try_from(x).unwrap_or_default())
                                        .collect(),
                                )
                            }
                            crate::AttrVal::IntArrayType(_) => {
                                let v: Vec<surrealdb::sql::Value> =
                                    v.try_into().unwrap_or_default();
                                NamedAttrValue::IntArrayType(
                                    v.into_iter()
                                        .map(|x| i32::try_from(x).unwrap_or_default())
                                        .collect(),
                                )
                            }
                            crate::AttrVal::BoolType(_) => {
                                NamedAttrValue::BoolType(v.try_into().unwrap_or_default())
                            }
                            crate::AttrVal::RefU64Type(_) | crate::AttrVal::ElementType(_) => {
                                if let SurlValue::Thing(record) = v {
                                    if matches!(record.id, Id::Array(_)) {
                                        NamedAttrValue::RefnoEnumType(record.into())
                                    } else {
                                        NamedAttrValue::RefU64Type(record.into())
                                    }
                                } else {
                                    NamedAttrValue::InvalidType
                                }
                            }
                            crate::AttrVal::RefU64Array(_) => {
                                let v: Vec<surrealdb::sql::Value> =
                                    v.try_into().unwrap_or_default();
                                NamedAttrValue::RefU64Array(
                                    v.into_iter()
                                        .map(|x| {
                                            if let SurlValue::Thing(id) = x {
                                                id.into()
                                            } else {
                                                Default::default()
                                            }
                                        })
                                        .collect(),
                                )
                            }
                            _ => NamedAttrValue::InvalidType,
                        };
                        map.insert(k.clone(), named_value);
                    }
                }
            }
        }
        Self { map }
    }
}

impl From<AttrMap> for NamedAttrMap {
    fn from(v: AttrMap) -> Self {
        (&v).into()
    }
}

impl From<&AttrMap> for NamedAttrMap {
    fn from(v: &AttrMap) -> Self {
        Self {
            map: v
                .map
                .iter()
                .map(|(h, v)| (db1_dehash(*h), NamedAttrValue::from(v)))
                .collect(),
        }
    }
}

#[cfg(feature = "sea-orm")]
impl Into<DynamicStruct> for NamedAttrMap {
    fn into(self) -> DynamicStruct {
        let mut ds = DynamicStruct::default();
        for (k, v) in self.map {
            match v.clone() {
                _ => {}
                NamedAttrValue::IntegerType(d) => ds.insert(k.as_str(), d),
                NamedAttrValue::StringType(d) => ds.insert(k.as_str(), d),
                NamedAttrValue::F32Type(d) => ds.insert(k.as_str(), d),
                NamedAttrValue::F32VecType(d) => ds.insert(k.as_str(), F32Vec(d)),
                NamedAttrValue::Vec3Type(d) => ds.insert(k.as_str(), F32Vec(d.to_array().into())),
                NamedAttrValue::StringArrayType(d) => ds.insert(k.as_str(), StringVec(d)),
                NamedAttrValue::BoolArrayType(d) => ds.insert(k.as_str(), BoolVec(d)),
                NamedAttrValue::IntArrayType(d) => ds.insert(k.as_str(), I32Vec(d)),
                NamedAttrValue::BoolType(d) => ds.insert(k.as_str(), d),
                NamedAttrValue::ElementType(d) => ds.insert(k.as_str(), d),
                NamedAttrValue::WordType(d) => ds.insert(k.as_str(), d),
                NamedAttrValue::RefU64Type(d) => ds.insert(k.as_str(), d),
            }
        }

        ds
    }
}

impl NamedAttrMap {
    ///初始化
    pub fn new(type_name: &str) -> Self {
        let mut v = Self::default();
        let db_info = get_default_pdms_db_info();
        let hash = db1_hash(type_name) as i32;
        if let Some(info) = db_info.noun_attr_info_map.get(&hash) {
            for kv in info.value() {
                if kv.offset == 0 {
                    v.insert(kv.name.clone(), (&kv.default_val).into());
                }
            }
        }
        v.insert(
            "TYPE".to_string(),
            NamedAttrValue::StringType(type_name.to_string()),
        );
        v
    }

    pub fn pe(&self, dbnum: i32) -> SPdmsElement {
        let refno = self.get_refno_or_default();
        let owner = self.get_refno_by_att_or_default("OWNER").into();
        let noun = self.get_type();
        let name = self.get_string("NAME").unwrap_or_default();

        let ele = SPdmsElement {
            refno,
            owner,
            name,
            noun,
            dbnum,
            cata_hash: self.cal_cata_hash(),
            sesno: self.sesno(),
            ..Default::default()
        };
        ele
    }

    #[inline]
    pub fn is_neg(&self) -> bool {
        TOTAL_NEG_NOUN_NAMES.contains(&self.get_type_str())
    }

    ///是否是joint类型（需要单独计算方位）
    #[inline]
    pub fn is_joint_type(&self) -> bool {
        JOINT_TYPES.contains(&self.get_type_str())
    }

    #[inline]
    pub fn get_name(&self) -> Option<String> {
        self.get_string("NAME")
    }

    #[inline]
    pub fn get_name_or_default(&self) -> String {
        self.get_string_or_default("NAME")
    }

    #[inline]
    pub fn get_dir(&self) -> Option<DVec3> {
        if let Some(end) = self.get_dpose()
            && let Some(start) = self.get_dposs()
        {
            Some((end - start).normalize())
        } else {
            None
        }
    }

    // #[inline]
    // pub fn set_pgno(&mut self, v: i32) {
    //     self.map
    //         .insert("PGNO".into(), NamedAttrValue::IntegerType(v));
    // }

    #[inline]
    pub fn set_sesno(&mut self, v: i32) {
        self.map
            .insert("SESNO".into(), NamedAttrValue::IntegerType(v));
    }

    #[inline]
    pub fn get_e3d_version(&self) -> i32 {
        self.sesno()
    }

    //PGNO
    #[inline]
    pub fn pgno(&self) -> i32 {
        self.get_i32("PGNO").unwrap_or_default()
    }

    #[inline]
    pub fn sesno(&self) -> i32 {
        self.get_i32("SESNO").unwrap_or_default()
    }

    pub fn split_to_default_groups(&self) -> (NamedAttrMap, NamedAttrMap, NamedAttrMap) {
        let mut default_att = NamedAttrMap::default();
        let mut comp_att = NamedAttrMap::default();
        let mut uda_att = NamedAttrMap::default();

        for (k, v) in self.map.clone() {
            if DEFAULT_NAMED_NOUNS.contains(&k.as_str()) {
                default_att.insert(k, v);
            } else if k.starts_with(":") {
                uda_att.insert(k, v);
            } else {
                comp_att.insert(k, v);
            }
        }
        (default_att, comp_att, uda_att)
    }

    #[inline]
    pub fn get_foreign_refno(&self, key: &str) -> Option<RefnoEnum> {
        if let NamedAttrValue::RefU64Type(d) = self.get_val(key)? {
            return Some(RefnoEnum::Refno(*d));
        } else if let NamedAttrValue::RefnoEnumType(d) = self.get_val(key)? {
            return Some(*d);
        }
        None
    }

    #[inline]
    pub fn is_type(&self, type_name: &str) -> bool {
        self.get_type() == type_name
    }

    #[inline]
    pub fn get_type_cloned(&self) -> Option<String> {
        self.get_str("TYPE").map(|x| x.to_string())
    }

    #[inline]
    pub fn get_u32(&self, key: &str) -> Option<u32> {
        self.get_i32(key).map(|s| s as u32)
    }

    #[inline]
    pub fn get_i32(&self, key: &str) -> Option<i32> {
        let v = self.get_val(key)?;
        match v {
            NamedAttrValue::IntegerType(d) => Some(*d as i32),
            _ => None,
        }
    }

    #[inline]
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        if let NamedAttrValue::BoolType(d) = self.get_val(key)? {
            return Some(*d);
        }
        None
    }
    #[inline]
    pub fn get_bool_or_default(&self, key: &str) -> bool {
        self.get_bool(key).unwrap_or_default()
    }

    #[inline]
    pub fn get_refu64(&self, key: &str) -> Option<RefU64> {
        let v = self.get_val(key)?;
        match v {
            NamedAttrValue::RefU64Type(d) => Some(*d),
            _ => None,
        }
    }

    #[inline]
    pub fn get_refu64_vec(&self, key: &str) -> Option<Vec<RefU64>> {
        let v = self.get_val(key)?;
        match v {
            NamedAttrValue::RefU64Array(d) => Some(d.into_iter().map(|x| x.refno()).collect()),
            _ => None,
        }
    }

    #[inline]
    pub fn get_ddesp(&self) -> Option<Vec<f32>> {
        if let Some(NamedAttrValue::F32VecType(d)) = self.get_val("DESP")
            && let Some(NamedAttrValue::IntArrayType(u)) = self.get_val("UNIPAR")
        {
            return Some(
                d.iter()
                    .zip(u)
                    .map(|(x, f)| if *f == WORD_HASH as i32 { 0.0 } else { *x })
                    .collect::<Vec<f32>>(),
            );
        }
        None
    }

    ///获取desp的文字描述
    #[inline]
    pub fn get_wdesp(&self) -> Option<Vec<String>> {
        if let Some(NamedAttrValue::F32VecType(d)) = self.get_val("DESP")
            && let Some(NamedAttrValue::IntArrayType(u)) = self.get_val("UNIPAR")
        {
            return Some(
                d.iter()
                    .zip(u)
                    .map(|(x, f)| {
                        if *f == WORD_HASH as i32 {
                            db1_dehash(*x as u32)
                        } else {
                            "".to_string()
                        }
                    })
                    .collect::<Vec<String>>(),
            );
        }
        None
    }

    // #[inline]
    // pub fn get_refno_as_string(&self) -> Option<String> {
    //     self.get_as_smol_str("REFNO")
    // }

    pub fn get_obstruction(&self) -> Option<u32> {
        self.get_u32("OBST")
    }

    pub fn get_level(&self) -> Option<[u32; 2]> {
        let v = self.get_i32_vec("LEVE")?;
        if v.len() >= 2 {
            return Some([v[0] as u32, v[1] as u32]);
        }
        None
    }

    ///判断构件是否可见
    pub fn is_visible_by_level(&self, level: Option<u32>) -> Option<bool> {
        let levels = self.get_level()?;
        let l = level.unwrap_or(LEVEL_VISBLE);
        Some(levels[0] <= l && l <= levels[1])
    }

    #[inline]
    pub fn history_id(&self) -> String {
        format!(
            "{}:{}_{}",
            self.get_type(),
            self.get_refno_or_default().refno(),
            self.sesno()
        )
    }

    #[inline]
    pub fn get_refno_or_default(&self) -> RefnoEnum {
        self.get_refno().unwrap_or_default()
    }

    #[inline]
    pub fn latest_refno(&self) -> RefU64 {
        self.get_refno_or_default().refno()
    }

    #[inline]
    pub fn ses_refno(&self) -> RefnoSesno {
        RefnoSesno::new(self.latest_refno(), self.sesno() as _)
    }

    #[inline]
    pub fn get_refno(&self) -> Option<RefnoEnum> {
        if let Some(NamedAttrValue::RefU64Type(d)) = self.get_val("REFNO") {
            return Some(RefnoEnum::Refno(*d));
        } else if let Some(NamedAttrValue::RefnoEnumType(d)) = self.get_val("REFNO") {
            return Some(*d);
        }
        None
    }

    #[inline]
    pub fn get_refno_lossy(&self) -> Option<RefU64> {
        if let Some(s) = self.get_as_string("REFNO") {
            return RefU64::from_str(s.as_str()).ok();
        } else if let Some(s) = self.get_as_string("refno") {
            // dbg!(&s);
            return RefU64::from_str(s.as_str()).ok();
        }
        None
    }

    #[inline]
    pub fn get_owner_as_string(&self) -> String {
        self.get_as_string("OWNER").unwrap_or(UNSET_STR.into())
    }

    pub fn get_type(&self) -> String {
        if let Some(NamedAttrValue::StringType(v)) = self.map.get("TYPE") {
            v.to_string()
        } else {
            "unset".to_string()
        }
    }

    pub fn get_type_str(&self) -> &str {
        if let Some(NamedAttrValue::StringType(v)) = self.map.get("TYPE") {
            v.as_str()
        } else {
            "unset"
        }
    }

    pub fn get_type_hash(&self) -> u32 {
        db1_hash(self.get_type_str())
    }

    #[inline]
    pub fn get_string_or_default(&self, att_name: &str) -> String {
        self.get_string(att_name).unwrap_or_default()
    }

    #[inline]
    pub fn get_string(&self, att_name: &str) -> Option<String> {
        let att = self.map.get(att_name)?;
        match att {
            NamedAttrValue::StringType(s) => Some(s.clone()),
            _ => None,
        }
    }

    #[inline]
    pub fn get_owner(&self) -> RefnoEnum {
        self.get_refno_by_att_or_default("OWNER")
    }

    #[inline]
    pub fn get_refno_by_att_or_default(&self, att_name: &str) -> RefnoEnum {
        self.get_refno_by_att(att_name).unwrap_or_default()
    }

    #[inline]
    pub fn get_refno_by_att(&self, att_name: &str) -> Option<RefnoEnum> {
        let att = self.map.get(att_name)?;
        match att {
            NamedAttrValue::RefU64Type(s) => Some(RefnoEnum::Refno(*s)),
            NamedAttrValue::RefnoEnumType(s) => Some(*s),
            _ => None,
        }
    }

    ///生成版本数据json
    pub fn gen_versioned_json_map(&self) -> IndexMap<String, serde_json::Value> {
        let mut map = IndexMap::new();
        let type_name = self.get_type();
        let refno = self.get_refno_by_att("REFNO").unwrap_or_default();
        map.insert("id".into(), refno.to_string().into());
        map.insert("TYPE".into(), type_name.into());
        map.insert("REFNO".into(), refno.to_string().into());

        for (key, val) in self.map.clone().into_iter() {
            //refno 单独处理
            if key.starts_with(":") || key.as_str() == "REFNO" {
                continue;
            }
            let new_key = key.replace(":", "_");
            map.insert(new_key, val.into());
        }
        map
    }

    pub fn gen_sur_json(&self) -> Option<String> {
        self.gen_sur_json_exclude(&[], None)
    }

    pub fn gen_sur_json_with_id(&self, id: String) -> Option<String> {
        self.gen_sur_json_exclude(&[], Some(id))
    }

    /// 根据会话号生成sur json字符串
    ///
    /// # 参数
    /// * `sesno` - 会话号
    /// * `sesno_map` - RefU64到会话号的映射表
    ///
    /// # 返回值
    /// * `Option<String>` - 生成的sur json字符串,如果生成失败则返回None
    pub fn gen_sur_json_with_sesno(
        &self,
        sesno: i32,
        sesno_map: &HashMap<RefU64, u32>,
    ) -> Option<String> {
        let mut map: IndexMap<String, serde_json::Value> = IndexMap::new();
        let mut record_map: IndexMap<String, RefnoEnum> = IndexMap::new();
        let mut records_map: IndexMap<String, Vec<RefnoEnum>> = IndexMap::new();
        let type_name = self.get_type();
        let refno = self.get_refno_or_default();
        // map.insert("id".into(), id.unwrap_or(refno.to_string()).into());
        let id_str = format!("['{}',{}]", refno.refno(), sesno);
        map.insert("TYPE".into(), type_name.into());

        for (key, val) in self.map.clone().into_iter() {
            //refno 单独处理
            if key.starts_with("UDA:") || key.as_str() == "REFNO" {
                continue;
            }
            if matches!(val, NamedAttrValue::RefU64Type(_))
                || matches!(val, NamedAttrValue::ElementType(_))
            {
                let refno = val.refno_value().unwrap_or_default();
                if let Some(&sesno) = sesno_map.get(&refno)
                    && sesno != 0
                {
                    record_map.insert(key, RefnoSesno::new(refno, sesno).into());
                } else {
                    record_map.insert(key, refno.into());
                }
            } else if let NamedAttrValue::RefU64Array(refnos) = val {
                for refno_enum in refnos {
                    let refno = refno_enum.refno();
                    if let Some(&sesno) = sesno_map.get(&refno)
                        && sesno != 0
                    {
                        records_map
                            .entry(key.clone())
                            .or_default()
                            .push(RefnoSesno::new(refno, sesno).into());
                    } else {
                        records_map
                            .entry(key.clone())
                            .or_default()
                            .push(refno.into());
                    }
                }
            } else {
                map.insert(key, val.into());
            }
        }

        //加上pe，去掉双引号
        let Ok(mut sjson) = serde_json::to_string_pretty(&map) else {
            dbg!(&self);
            return None;
        };

        sjson.remove(sjson.len() - 1);
        //后续是否需要指定 sesno，更新数据里的 引用数据
        sjson.push_str(&format!(
            r#", "REFNO": pe:['{}',{}], "id": {}, "#,
            refno.refno(),
            sesno,
            id_str
        ));
        for (key, val) in record_map.into_iter() {
            if val.refno().is_unset() {
                continue;
            }
            sjson.push_str(&format!(r#""{}": {},"#, key, val.to_pe_key()));
        }
        for (key, val) in records_map.into_iter() {
            if val.is_empty() {
                continue;
            }
            let s = format!(
                r#""{}": [{}],"#,
                key,
                val.iter()
                    .map(|r| r.to_pe_key())
                    .collect::<Vec<_>>()
                    .join(",")
            );
            // dbg!(&s);
            sjson.push_str(&s);
        }
        sjson.remove(sjson.len() - 1);
        sjson.push_str("}");

        Some(normalize_sql_string(&sjson))
    }

    /// 生成排除指定字段的JSON字符串
    ///
    /// # 参数
    /// * `excludes` - 需要排除的字段名数组
    /// * `id` - 可选的ID字符串,如果不提供则使用refno作为ID
    ///
    /// # 返回值
    /// * `Option<String>` - 成功则返回JSON字符串,失败返回None
    pub fn gen_sur_json_exclude(&self, excludes: &[&str], id: Option<String>) -> Option<String> {
        let mut map: IndexMap<String, serde_json::Value> = IndexMap::new();
        let mut record_map: IndexMap<String, RefU64> = IndexMap::new();
        let mut records_map: IndexMap<String, Vec<RefU64>> = IndexMap::new();
        let type_name = self.get_type();
        let refno = self.get_refno_or_default();
        map.insert("id".into(), id.unwrap_or(refno.to_string()).into());
        map.insert("TYPE".into(), type_name.into());

        for (key, val) in self.map.clone().into_iter() {
            //refno 单独处理
            if key.starts_with("UDA:") || key.as_str() == "REFNO" {
                continue;
            }
            if matches!(val, NamedAttrValue::RefU64Type(_))
                || matches!(val, NamedAttrValue::ElementType(_))
            {
                record_map.insert(key, val.refno_value().unwrap_or_default());
            } else if let NamedAttrValue::RefU64Array(refnos) = val {
                records_map.insert(key, refnos.into_iter().map(|x| x.refno()).collect());
            } else {
                map.insert(key, val.into());
            }
        }

        for key in excludes {
            map.remove(*key);
        }

        //加上pe，去掉双引号
        let Ok(mut sjson) = serde_json::to_string_pretty(&map) else {
            dbg!(&self);
            return None;
        };

        sjson.remove(sjson.len() - 1);
        //后续是否需要指定 sesno，更新数据里的 引用数据
        sjson.push_str(&format!(r#", "REFNO": {}, "#, refno.to_pe_key(),));
        for (key, val) in record_map.into_iter() {
            if val.is_unset() && excludes.contains(&key.as_str()) {
                continue;
            }
            sjson.push_str(&format!(r#""{}": {},"#, key, val.to_pe_key()));
        }
        for (key, val) in records_map.into_iter() {
            if val.is_empty() && excludes.contains(&key.as_str()) {
                continue;
            }
            let s = format!(
                r#""{}": [{}],"#,
                key,
                val.iter()
                    .map(|r| r.to_pe_key())
                    .collect::<Vec<_>>()
                    .join(",")
            );
            // dbg!(&s);
            sjson.push_str(&s);
        }
        sjson.remove(sjson.len() - 1);
        sjson.push_str("}");

        Some(normalize_sql_string(&sjson))
    }

    pub fn gen_sur_json_uda(&self, excludes: &[&str]) -> Option<String> {
        let mut uda_json_vec = vec![];
        for (key, val) in self.map.clone().into_iter() {
            //refno 单独处理
            if key.as_str() == "REFNO" {
                continue;
            }
            if key.starts_with("UDA:") {
                let json = if matches!(val, NamedAttrValue::RefU64Type(_))
                    || matches!(val, NamedAttrValue::ElementType(_))
                {
                    val.refno_value().unwrap_or_default().to_pe_key()
                } else {
                    serde_json::to_string(&val).unwrap()
                };
                uda_json_vec.push(format!("{{ 'u': {}, 'v': {} }}", key.as_str(), json));
            }
        }
        if uda_json_vec.is_empty() {
            return None;
        }

        let type_name = self.get_type();
        let refno = self.get_refno_by_att("REFNO").unwrap_or_default();
        let mut map: IndexMap<String, serde_json::Value> = IndexMap::new();
        map.insert("id".into(), refno.to_string().into());
        map.insert("TYPE".into(), type_name.clone().into());

        for key in excludes {
            map.remove(*key);
        }

        //加上pe，去掉双引号
        let Ok(mut sjson) = serde_json::to_string_pretty(&map) else {
            dbg!(&self);
            return None;
        };

        sjson.remove(sjson.len() - 1);
        sjson.push_str(&format!(
            r#", "refno": {}:{} "#,
            type_name,
            refno.to_string()
        ));
        if !uda_json_vec.is_empty() {
            sjson.push_str(&format!(r#", "udas": [{}]"#, uda_json_vec.join(",")));
        }
        sjson.push_str("}");
        Some(sjson)
    }

    #[inline]
    pub fn get_matrix(&self) -> Option<Affine3A> {
        let mut affine = Affine3A::IDENTITY;
        let pos = self.get_f32_vec("POS")?;
        affine.translation = glam::f32::Vec3A::new(pos[0], pos[1], pos[2]);
        let ang = self.get_f32_vec("ORI")?;
        affine.matrix3 = glam::f32::Mat3A::from_rotation_z(ang[2].to_radians())
            * glam::f32::Mat3A::from_rotation_y(ang[1].to_radians())
            * glam::f32::Mat3A::from_rotation_x(ang[0].to_radians());
        Some(affine)
    }

    #[inline]
    pub fn get_mat4(&self) -> Option<Mat4> {
        Some(Mat4::from(self.get_matrix()?))
    }

    pub fn get_f32_vec(&self, key: &str) -> Option<Vec<f32>> {
        let val = self.get_val(key)?;
        return match val {
            NamedAttrValue::F32VecType(data) => Some(data.clone()),
            NamedAttrValue::Vec3Type(data) => Some(vec![data.x, data.y, data.z]),
            _ => None,
        };
    }

    pub fn get_vec3(&self, key: &str) -> Option<Vec3> {
        if let NamedAttrValue::Vec3Type(d) = self.get_val(key)? {
            return Some(Vec3::new(d[0], d[1], d[2]));
        }
        None
    }

    pub fn get_dvec3(&self, key: &str) -> Option<DVec3> {
        self.get_vec3(key).map(|v| DVec3::from(v))
    }

    pub fn get_i32_vec(&self, key: &str) -> Option<Vec<i32>> {
        if let NamedAttrValue::IntArrayType(d) = self.get_val(key)? {
            return Some(d.clone());
        }
        None
    }

    pub fn get_refno_vec(&self, key: &str) -> Option<Vec<RefnoEnum>> {
        if let NamedAttrValue::RefU64Array(d) = self.get_val(key)? {
            return Some(d.clone());
        }
        None
    }

    ///生成具有几何属性的element的shape
    pub fn create_brep_shape(&self, limit_size: Option<f32>) -> Option<Box<dyn BrepShapeTrait>> {
        let type_noun = self.get_type_str();
        let mut r: Option<Box<dyn BrepShapeTrait>> = match type_noun {
            "BOX" | "NBOX" => Some(Box::new(SBox::from(self))),
            "CYLI" | "SLCY" | "NCYL" => Some(Box::new(SCylinder::from(self))),
            "SPHE" => Some(Box::new(Sphere::from(self))),
            "CONE" | "NCON" | "SNOU" | "NSNO" => Some(Box::new(LSnout::from(self))),
            "DISH" | "NDIS" => Some(Box::new(Dish::from(self))),
            "CTOR" | "NCTO" => Some(Box::new(CTorus::from(self))),
            "RTOR" | "NRTO" => Some(Box::new(RTorus::from(self))),
            "PYRA" | "NPYR" => Some(Box::new(Pyramid::from(self))),
            _ => None,
        };
        if r.is_some() && limit_size.is_some() {
            r.as_mut().unwrap().apply_limit_by_size(limit_size.unwrap());
        }
        r
    }

    #[inline]
    pub fn get_val(&self, key: &str) -> Option<&NamedAttrValue> {
        self.map.get(key).into()
    }

    #[inline]
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.get_f32(key).map(|x| x as f64)
    }

    #[inline]
    pub fn get_f32(&self, key: &str) -> Option<f32> {
        self.get_val(key)?.f32_value()
    }

    #[inline]
    pub fn get_f32_or_default(&self, key: &str) -> f32 {
        self.get_f32(key).unwrap_or_default()
    }

    /// 获取位置信息
    ///
    /// # 返回值
    /// * `Option<Vec3>` - 如果存在POS属性，返回对应的Vec3位置向量；如果不存在POS但存在POSS，则返回POSS位置向量；否则返回None
    #[inline]
    pub fn get_position(&self) -> Option<Vec3> {
        if let Some(pos) = self.get_f32_vec("POS") {
            return Some(Vec3::new(pos[0], pos[1], pos[2]));
        } else {
            //如果没有POS，就以POSS来尝试
            self.get_poss()
        }
    }

    #[inline]
    pub fn get_posse_dist(&self) -> Option<f32> {
        Some(self.get_pose()?.distance(self.get_poss()?))
    }

    #[inline]
    pub fn get_poss(&self) -> Option<Vec3> {
        let pos = self.get_f32_vec("POSS")?;
        if pos.len() == 3 {
            return Some(Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32));
        }
        None
    }

    #[inline]
    pub fn get_dposs(&self) -> Option<DVec3> {
        self.get_poss().map(|v| DVec3::from(v))
    }

    #[inline]
    pub fn get_pose(&self) -> Option<Vec3> {
        let pos = self.get_f32_vec("POSE")?;
        if pos.len() == 3 {
            return Some(Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32));
        }
        None
    }

    #[inline]
    pub fn get_dpose(&self) -> Option<DVec3> {
        self.get_pose().map(|v| DVec3::from(v))
    }

    #[inline]
    pub fn get_str(&self, key: &str) -> Option<&str> {
        let v = self.get_val(key)?;
        match v {
            NamedAttrValue::StringType(s)
            | NamedAttrValue::WordType(s)
            | NamedAttrValue::ElementType(s) => Some(s.as_str()),
            _ => None,
        }
    }

    #[inline]
    pub fn get_str_or_default(&self, key: &str) -> &str {
        self.get_str(key).unwrap_or("unset")
    }

    #[inline]
    pub fn get_as_strings(&self, keys: &[&str]) -> Vec<String> {
        let mut result = vec![];
        for key in keys {
            result.push(self.get_as_string(*key).unwrap_or(UNSET_STR.to_string()));
        }
        result
    }

    #[inline]
    pub fn get_as_string(&self, key: &str) -> Option<String> {
        use NamedAttrValue::*;
        let v = self.get_val(key)?;
        let s = match v {
            StringType(s) | WordType(s) | ElementType(s) => s.to_string(),
            IntegerType(d) => d.to_string().into(),
            F32Type(d) => d.to_string().into(),
            BoolType(d) => d.to_string().into(),
            F32VecType(d) => d
                .iter()
                .map(|i| format!(" {:.3}", i))
                .collect::<String>()
                .into(),
            StringArrayType(d) => d
                .iter()
                .map(|i| format!(" {}", i))
                .collect::<String>()
                .into(),
            IntArrayType(d) => d
                .iter()
                .map(|i| format!(" {}", i))
                .collect::<String>()
                .into(),
            BoolArrayType(d) => d
                .iter()
                .map(|i| format!(" {}", i))
                .collect::<String>()
                .into(),
            Vec3Type(d) => d
                .to_array()
                .iter()
                .map(|i| format!(" {:.3}", i))
                .collect::<String>()
                .into(),
            RefU64Type(d) => RefI32Tuple::from(d).into(),
            _ => UNSET_STR.into(),
        };
        Some(s)
    }

    #[inline]
    pub fn get_rotation(&self) -> Option<DQuat> {
        let type_name = self.get_type_str();
        let mut quat = DQuat::IDENTITY;
        if self.contains_key("ZDIR") {
            let axis_dir = self.get_dvec3("ZDIR").unwrap_or_default().normalize();
            if axis_dir.is_normalized() {
                quat = cal_quat_by_zdir_with_xref(axis_dir);
                // dbg!(dquat_to_pdms_ori_xyz_str(&quat, true));
            }
        } else if self.contains_key("OPDI") {
            //PJOI 的方向
            let axis_dir = self.get_dvec3("OPDI").unwrap_or_default().normalize();
            if axis_dir.is_normalized() {
                quat = cal_quat_by_zdir_with_xref(axis_dir);
            }
        } else {
            match type_name {
                "CMPF" | "PFIT" => {
                    let sjus = self.get_str("SJUS").unwrap_or("unset");
                    //unset 和 UBOT 一样的效果
                    //DTOP, DCEN, DBOT
                    if sjus.starts_with("D") {
                        quat = DQuat::from_mat3(&DMat3::from_cols(
                            DVec3::X,
                            DVec3::NEG_Y,
                            DVec3::NEG_Z,
                        ));
                    }
                }
                _ => {
                    let angs = self.get_dvec3("ORI")?;
                    quat = angles_to_ori(angs)?;
                }
            }
        }
        return Some(quat);
    }
}

impl NamedAttrMap {
    #[cfg(feature = "sea-orm")]
    #[inline]
    pub fn get_columns(&self) -> Vec<Alias> {
        self.map.keys().map(|x| Alias::new(x.clone())).collect()
    }

    #[cfg(feature = "sea-orm")]
    #[inline]
    pub fn get_values(&self) -> Vec<sea_query::Value> {
        self.map.values().map(|x| x.clone().into()).collect()
    }

    //填充其他显示类型数据为默认数据, 包括uda的默认属性
    pub fn fill_explicit_default_values(&mut self) {
        let db_info = get_default_pdms_db_info();
        let noun_hash = self.get_type_hash() as i32;
        if let Some(m) = db_info.noun_attr_info_map.get(&noun_hash) {
            for info in m.value() {
                if info.offset == 0 && !self.map.contains_key(&info.name) {
                    self.map
                        .insert(info.name.clone(), (&info.default_val).into());
                }
            }
        }
    }

    pub fn contains_attr_hash(&self, hash: u32) -> bool {
        self.map.contains_key(&db1_dehash(hash))
    }

    ///执行保存
    #[cfg(feature = "sea-orm")]
    pub async fn exec_insert(&self, db: &DatabaseConnection, replace: bool) -> anyhow::Result<()> {
        let sql = self.gen_insert_sql(replace)?;
        db.execute_unprepared(&sql).await?;
        Ok(())
    }

    ///生成保存的sql
    #[cfg(feature = "sea-orm")]
    pub fn gen_insert_sql(&self, replace: bool) -> anyhow::Result<String> {
        let type_name = self.get_type();

        let mut query = sea_query::Query::insert()
            .into_table(Alias::new(type_name))
            .columns(self.get_columns())
            .to_owned();
        if replace {
            query.replace();
        }
        query
            .values(self.get_values().into_iter().map(|x| x.into()))?
            .to_owned();
        Ok(query.to_string(MysqlQueryBuilder))
    }

    ///生成插入的语句
    #[cfg(feature = "sea-orm")]
    pub async fn exec_insert_many<I>(
        atts: I,
        db: &DatabaseConnection,
        replace: bool,
    ) -> anyhow::Result<()>
    where
        I: IntoIterator<Item = Self>,
    {
        let sqls = Self::gen_insert_many_sql(atts, replace)?;
        for sql in sqls {
            db.execute_unprepared(&sql).await?;
        }
        Ok(())
    }

    ///生成插入的语句
    #[cfg(feature = "sea-orm")]
    pub fn gen_insert_many_sql<I>(atts: I, replace: bool) -> anyhow::Result<Vec<String>>
    where
        I: IntoIterator<Item = Self>,
    {
        ///按照类型重新划分组
        let mut grouped_map: BTreeMap<String, Vec<Self>> = BTreeMap::new();

        for mut a in atts {
            let type_name = a.get_type();
            a.fill_explicit_default_values();
            grouped_map.entry(type_name).or_insert(Vec::new()).push(a);
        }
        if grouped_map.is_empty() {
            return Err(anyhow::anyhow!("Empty atts can't gen insert sql."));
        }
        let mut final_sqls = Vec::new();
        //这里需要按type归类，按不同的type分类
        for (type_name, new_atts) in grouped_map {
            let colums = new_atts[0].get_columns();
            let mut query = sea_query::Query::insert()
                .into_table(Alias::new(type_name))
                .columns(colums)
                .to_owned();
            if replace {
                query.replace();
            }
            new_atts.into_iter().map(|x| x.get_values()).for_each(|x| {
                query.values(x.into_iter().map(|x| x.into())).unwrap();
            });
            let sql = query.to_string(MysqlQueryBuilder);
            final_sqls.push(sql);
        }
        Ok(final_sqls)
    }

    ///执行增量更新的提交
    #[cfg(feature = "sea-orm")]
    pub async fn exec_commit_atts_change(
        db: &DatabaseConnection,
        message: Option<&str>,
    ) -> anyhow::Result<()> {
        // db.execute_unprepared(r#"call dolt_add('.')"#).await.unwrap();
        let msg = message.unwrap_or("提交增量更新数据");
        db.execute_unprepared(&format!(r#"call dolt_commit('-m', '{}')"#, msg))
            .await?;

        Ok(())
    }

    /// 获取string属性数组，忽略为空的值
    pub fn get_attr_strings_without_default(&self, keys: &[&str]) -> Vec<String> {
        let mut results = vec![];
        for &attr_name in keys {
            if let Some(result) = self.get_val(attr_name) {
                match result {
                    NamedAttrValue::StringType(v) => {
                        if v != "" {
                            results.push(v.trim_matches('\0').to_owned().clone().into());
                        }
                    }
                    _ => {}
                }
            }
        }
        results
    }

    pub fn get_attr_strings(&self, keys: &[&str]) -> Vec<String> {
        let mut results = vec![];
        for &attr_name in keys {
            if let Some(result) = self.get_str(attr_name) {
                results.push(result.trim_matches('\0').to_owned().clone().into());
            } else {
                results.push("".to_string());
            }
        }
        results
    }

    //后面还要根据参考号确定使用哪个类型、还有db numer
    //生成查询语句
    #[cfg(feature = "sea-orm")]
    pub fn gen_query_sql(refnos: &Vec<RefU64>) -> anyhow::Result<Vec<String>> {
        //首先要查询到类型信息
        let types = sea_query::Query::select().to_owned();

        //按照类型不同, 分别去查询
        let query = sea_query::Query::select()
            // .cond_where()
            // .columns()
            // .column(Char::Character)
            // .column((Font::Table, Font::Name))
            // .from(Char::Table)
            // .left_join(Font::Table, Expr::col((Char::Table, Char::FontId)).equals((Font::Table, Font::Id)))
            // .and_where(Expr::col(Char::SizeW).is_in([3, 4]))
            // .and_where(Expr::col(Char::Character).like("A%"))
            .to_owned();
        Ok(vec![])
    }
}

impl NamedAttrMap {
    //计算使用元件库的design 元件 hash
    pub fn cal_cata_hash(&self) -> String {
        //todo 先只处理spref有值的情况，还需要处理 self.get_as_string("CATA")
        let type_name = self.get_type_str();
        //由于有ODESP这种，会导致复用出现问题，怎么解决这个问题
        //1、主动去判断是否CataRef是这个类型，即有ODESP这种字段，然后从复用的逻辑排除出来
        //2、解析的时候发现表达式有这些字段，主动去给catref加一个标记位，表示是不能复用的构件
        //3、把相关的数据都做一遍统计，owner、attach

        //todo 这里能否使用数据库查询得到的数据，而不是从内存中获取
        let ref_name = if type_name == "NOZZ" || type_name == "ELCONN" {
            "CATR"
        } else {
            "SPRE"
        };
        if let Some(spref) = self.get_as_string(ref_name)
            && !CATA_WITHOUT_REUSE_GEO_NAMES.contains(&type_name)
        {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            std::hash::Hash::hash(&spref, &mut hasher);
            if let Some(des_para) = self.get_f32_vec("DESP") {
                hash_f32_slice(&des_para, &mut hasher);
            }
            let ref_strs = ["ANGL", "HEIG", "RADI"];
            let key_strs = self.get_as_strings(&ref_strs);
            for (ref_str, key_str) in ref_strs.iter().zip(key_strs) {
                std::hash::Hash::hash(*ref_str, &mut hasher);
                std::hash::Hash::hash(&key_str, &mut hasher);
            }

            //如果是土建模型 "DRNS", "DRNE"
            if let Some(drns) = self.get_as_string("DRNS")
                && let Some(drne) = self.get_as_string("DRNE")
            {
                std::hash::Hash::hash(&drns, &mut hasher);
                std::hash::Hash::hash(&drne, &mut hasher);
                let poss = self.get_vec3("POSS").unwrap_or_default();
                let pose = self.get_vec3("POSE").unwrap_or_default();
                let v = (pose - poss).length();
                hash_f32(v, &mut hasher);
            }
            //JUSL is adjus in wire calculation, so here we should make hash unique by jusl
            let jusl = self.get_str_or_default("JUSL");
            std::hash::Hash::hash(jusl, &mut hasher);

            let val = std::hash::Hasher::finish(&hasher);

            return val.to_string();
        }
        return self.get_refno().unwrap_or_default().to_string();
    }
}
