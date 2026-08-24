use crate::cache::mgr::BytesTrait;
use crate::consts::{ATT_CURD, ATT_STYP, UNSET_STR};
use crate::pdms_types::*;
use crate::prim_geo::ctorus::CTorus;
use crate::prim_geo::cylinder::SCylinder;
use crate::prim_geo::dish::Dish;
use crate::prim_geo::pyramid::Pyramid;
use crate::prim_geo::rtorus::RTorus;
use crate::prim_geo::sbox::SBox;
use crate::prim_geo::snout::LSnout;
use crate::prim_geo::sphere::Sphere;
use crate::ref64vec::RefU64Vec;
use crate::shape::pdms_shape::BrepShapeTrait;
use crate::tool::db_tool::{db1_dehash, db1_hash, db1_hash_i32};
use crate::tool::float_tool::{hash_f32, hash_f64_slice};
use crate::types::attval::AttrVal;
use crate::types::attval::AttrVal::*;
use crate::{BHashMap, RefI32Tuple, RefU64};
use bevy_ecs::component::Component;
use derive_more::{Deref, DerefMut};
use glam::*;
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Debug;

///PDMS的属性数据Map
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
    Component,
)]
pub struct AttrMap {
    pub map: BHashMap<NounHash, AttrVal>,
}

impl Debug for AttrMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.to_string_hashmap();
        s.fmt(f)
    }
}

// impl Into<NamedAttrMap> for AttrMap {
//     fn into(self) -> NamedAttrMap {
//         let mut map = BTreeMap::new();
//         for (k, v) in self.map {
//             if is_uda(k) {   continue; }
//             map.entry(db1_dehash(k)).or_insert(NamedAttrValue::from(&v));
//         }
//         NamedAttrMap { map }
//     }
// }

#[cfg(not(target_arch = "wasm32"))]
impl BytesTrait for AttrMap {}

impl AttrMap {
    ///是否为负实体
    #[inline]
    pub fn is_neg(&self) -> bool {
        TOTAL_NEG_NOUN_NAMES.contains(&self.get_type())
    }

    ///是否为正实体
    #[inline]
    pub fn is_pos(&self) -> bool {
        GENRAL_POS_NOUN_NAMES.contains(&self.get_type())
    }

    #[inline]
    pub fn is_joint_type(&self) -> bool {
        JOINT_TYPES.contains(&self.get_type())
    }

    ///是否为空
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.map.len() == 0
    }

    ///序列化成bincode
    #[inline]
    pub fn into_bincode_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    ///从bincode反序列化
    #[inline]
    pub fn from_bincode_bytes(bytes: &[u8]) -> Option<Self> {
        bincode::deserialize(bytes).ok()
    }

    #[inline]
    pub fn into_rkyv_bytes(&self) -> Vec<u8> {
        rkyv::to_bytes::<_, 1024>(self).unwrap().to_vec()
    }

    #[inline]
    pub fn into_rkyv_compress_bytes(&self) -> Vec<u8> {
        use flate2::Compression;
        use flate2::write::DeflateEncoder;
        use std::io::Write;
        let mut e = DeflateEncoder::new(Vec::new(), Compression::default());
        let _ = e.write_all(&self.into_rkyv_bytes());
        e.finish().unwrap_or_default()
    }

    #[inline]
    pub fn from_rkyv_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        use rkyv::Deserialize;
        let archived = unsafe { rkyv::archived_root::<Self>(bytes) };
        let r: Self = archived.deserialize(&mut rkyv::Infallible)?;
        Ok(r)
    }

    #[inline]
    pub fn from_rkvy_compress_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        use flate2::write::DeflateDecoder;
        use std::io::Write;
        let writer = Vec::new();
        let mut deflater = DeflateDecoder::new(writer);
        deflater.write_all(bytes)?;
        Self::from_rkyv_bytes(&deflater.finish()?)
    }

    #[inline]
    pub fn into_compress_bytes(&self) -> Vec<u8> {
        use flate2::Compression;
        use flate2::write::DeflateEncoder;
        use std::io::Write;
        let mut e = DeflateEncoder::new(Vec::new(), Compression::default());
        let _ = e.write_all(&self.into_bincode_bytes());
        e.finish().unwrap_or_default()
    }

    #[inline]
    pub fn from_compress_bytes(bytes: &[u8]) -> Option<Self> {
        use flate2::write::DeflateDecoder;
        use std::io::Write;
        let writer = Vec::new();
        let mut deflater = DeflateDecoder::new(writer);
        deflater.write_all(bytes).ok()?;
        bincode::deserialize(&deflater.finish().ok()?).ok()
    }

    //计算使用元件库的design 元件 hash
    pub fn cal_cata_hash(&self) -> Option<u64> {
        //todo 先只处理spref有值的情况，还需要处理 self.get_as_string("CATA")
        let type_name = self.get_type();
        if CATA_HAS_TUBI_GEO_NAMES.contains(&type_name) {
            return Some(*self.get_refno().unwrap_or_default());
        }
        //由于有ODESP这种，会导致复用出现问题，怎么解决这个问题
        //1、主动去判断是否CataRef是这个类型，即有ODESP这种字段，然后从复用的逻辑排除出来
        //2、解析的时候发现表达式有这些字段，主动去给catref加一个标记位，表示是不能复用的构件
        //3、把相关的数据都做一遍统计，owner、attach
        let ref_name = if type_name == "NOZZ" || type_name == "ELCONN" {
            "CATR"
        } else {
            "SPRE"
        };
        if let Some(spref) = self.get_as_string(ref_name) {
            if spref.starts_with('0') {
                return None;
            }
            if CATA_WITHOUT_REUSE_GEO_NAMES.contains(&type_name) {
                return Some(*self.get_refno().unwrap_or_default());
            }
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            std::hash::Hash::hash(&spref, &mut hasher);
            if let Some(des_para) = self.get_f32_vec("DESP") {
                hash_f64_slice(&des_para, &mut hasher);
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

            return Some(val);
        }
        return None;
    }

    // 返回 DESI 、 CATA .. 等模块值
    pub fn get_db_stype(&self) -> Option<&'static str> {
        let val = self.map.get(&ATT_STYP)?;
        match val {
            AttrVal::IntegerType(v) => Some(match *v {
                1 => "DESI",
                2 => "CATA",
                8 => "DICT",
                _ => "UNSET",
            }),
            _ => None,
        }
    }
}

impl AttrMap {
    pub fn split_to_default_groups(&self) -> (AttrMap, AttrMap) {
        let mut default_att = AttrMap::default();
        let mut comp_att = AttrMap::default();

        for (k, v) in self.map.iter() {
            if DEFAULT_NOUNS.contains(k) {
                default_att.map.insert(k.clone(), v.clone());
            } else {
                comp_att.insert(k.clone(), v.clone());
            }
        }
        (default_att, comp_att)
    }
}

impl AttrMap {
    #[inline]
    pub fn get_att_by_name(&self, name: &str) -> Option<&AttrVal> {
        self.map.get(&db1_hash(name))
    }

    #[inline]
    pub fn insert(&mut self, k: NounHash, v: AttrVal) {
        self.map.insert(k, v);
    }

    #[inline]
    pub fn insert_by_att_name(&mut self, k: &str, v: AttrVal) {
        self.map.insert(db1_hash(k), v);
    }

    #[inline]
    pub fn contains_attr_name(&self, name: &str) -> bool {
        self.map.contains_key(&db1_hash(name))
    }

    #[inline]
    pub fn contains_attr_hash(&self, hash: u32) -> bool {
        self.map.contains_key(&hash)
    }

    pub fn to_string_hashmap(&self) -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        for (k, v) in &self.map {
            map.insert(db1_dehash(*k), format!("{:?}", v));
        }
        map
    }

    #[inline]
    pub fn get_name_hash(&self) -> AiosStrHash {
        return if let Some(StringHashType(name_hash)) = self.get_val("NAME") {
            *name_hash
        } else {
            0
        };
    }

    #[inline]
    pub fn get_name_string(&self) -> String {
        return if let Some(StringType(name)) = self.get_val("NAME") {
            name.clone()
        } else {
            Default::default()
        };
    }

    #[inline]
    pub fn get_name(&self) -> Option<String> {
        return if let Some(StringType(name)) = self.get_val("NAME") {
            Some(name.clone())
        } else {
            None
        };
    }

    #[inline]
    pub fn get_main_db_in_mdb(&self) -> Option<RefU64> {
        if let Some(v) = self.map.get(&ATT_CURD) {
            match v {
                AttrVal::IntArrayType(v) => {
                    let refno = RefU64::from_two_nums(v[0] as u32, v[1] as u32);
                    return Some(refno);
                }
                _ => {}
            }
        }
        None
    }

    #[inline]
    pub fn get_foreign_refno(&self, key: &str) -> Option<RefU64> {
        if let RefU64Type(d) = self.get_val(key)? {
            return Some(*d);
        }
        None
    }

    #[inline]
    pub fn get_refno_as_string(&self) -> Option<String> {
        self.get_as_smol_str("REFNO")
    }

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
    pub fn get_refno(&self) -> Option<RefU64> {
        if let RefU64Type(d) = self.get_val("REFNO")? {
            return Some(*d);
        }
        None
    }

    #[inline]
    pub fn get_owner(&self) -> RefU64 {
        if let Some(RefU64Type(d)) = self.get_val("OWNER") {
            return *d;
        }
        RefU64::default()
    }

    #[inline]
    pub fn get_owner_as_string(&self) -> String {
        self.get_as_string("OWNER").unwrap_or(UNSET_STR.into())
    }

    #[inline]
    pub fn get_type(&self) -> &str {
        self.get_str("TYPE").unwrap_or("unset")
    }

    #[inline]
    pub fn get_noun(&self) -> i32 {
        db1_hash_i32(self.get_type())
    }

    #[inline]
    pub fn get_typex(&self) -> &str {
        self.get_str("TYPEX").unwrap_or("unset")
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
            IntegerType(d) => Some(*d as i32),
            _ => None,
        }
    }

    #[inline]
    pub fn get_refu64(&self, key: &str) -> Option<RefU64> {
        let v = self.get_val(key)?;
        match v {
            RefU64Type(d) => Some(*d),
            _ => None,
        }
    }

    #[inline]
    pub fn get_refu64_vec(&self, key: &str) -> Option<RefU64Vec> {
        let v = self.get_val(key)?;
        match v {
            RefU64Array(d) => Some(d.clone()),
            _ => None,
        }
    }

    #[inline]
    pub fn get_str(&self, key: &str) -> Option<&str> {
        let v = self.get_val(key)?;
        match v {
            StringType(s) | WordType(s) | ElementType(s) => Some(s.as_str()),
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
        let v = self.get_val(key)?;
        let s = match v {
            StringType(s) | WordType(s) | ElementType(s) => s.to_string(),
            IntegerType(d) => d.to_string().into(),
            DoubleType(d) => d.to_string().into(),
            BoolType(d) => d.to_string().into(),
            DoubleArrayType(d) => d
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
                .iter()
                .map(|i| format!(" {:.3}", i))
                .collect::<String>()
                .into(),

            RefU64Type(d) => RefI32Tuple::from(d).into(),
            StringHashType(d) => format!("{d}").into(),

            _ => UNSET_STR.into(),
        };
        Some(s)
    }

    #[inline]
    pub fn get_as_smol_str(&self, key: &str) -> Option<String> {
        let v = self.get_val(key)?;
        let s = match v {
            StringType(s) | WordType(s) | ElementType(s) => s.clone(),
            IntegerType(d) => d.to_string().into(),
            DoubleType(d) => d.to_string().into(),
            BoolType(d) => d.to_string().into(),
            DoubleArrayType(d) => d
                .iter()
                .map(|i| format!(" {}", i))
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
                .iter()
                .map(|i| format!(" {}", i))
                .collect::<String>()
                .into(),

            RefU64Type(d) => RefI32Tuple::from(d).into(),
            StringHashType(d) => format!("{d}").into(),

            _ => UNSET_STR.into(),
        };
        Some(s)
    }

    #[inline]
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        if let AttrVal::BoolType(d) = self.get_val(key)? {
            return Some(*d);
        }
        None
    }

    #[inline]
    pub fn get_val(&self, key: &str) -> Option<&AttrVal> {
        self.map.get(&db1_hash(key).into())
    }

    #[inline]
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.get_val(key)?.double_value()
    }

    #[inline]
    pub fn get_f32(&self, key: &str) -> Option<f32> {
        self.get_f64(key).map(|x| x as f32)
    }

    #[inline]
    pub fn get_f32_or_default(&self, key: &str) -> f32 {
        self.get_f64(key).map(|x| x as f32).unwrap_or_default()
    }

    #[inline]
    pub fn get_position(&self) -> Option<Vec3> {
        if let Some(pos) = self.get_f32_vec("POS") {
            return Some(Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32));
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
    pub fn get_pose(&self) -> Option<Vec3> {
        let pos = self.get_f32_vec("POSE")?;
        if pos.len() == 3 {
            return Some(Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32));
        }
        None
    }

    pub fn get_matrix(&self) -> Option<Affine3A> {
        let mut affine = Affine3A::IDENTITY;
        let pos = self.get_f32_vec("POS")?;
        affine.translation = glam::f32::Vec3A::new(pos[0] as f32, pos[1] as f32, pos[2] as f32);
        let ang = self.get_f32_vec("ORI")?;
        affine.matrix3 = glam::f32::Mat3A::from_rotation_z(ang[2].to_radians() as f32)
            * glam::f32::Mat3A::from_rotation_y(ang[1].to_radians() as f32)
            * glam::f32::Mat3A::from_rotation_x(ang[0].to_radians() as f32);
        Some(affine)
    }

    #[inline]
    pub fn get_mat4(&self) -> Option<Mat4> {
        Some(Mat4::from(self.get_matrix()?))
    }

    pub fn get_f32_vec(&self, key: &str) -> Option<Vec<f64>> {
        let val = self.get_val(key)?;
        return match val {
            AttrVal::DoubleArrayType(data) => Some(data.clone()),
            AttrVal::Vec3Type(data) => Some(data.to_vec()),
            _ => None,
        };
    }

    pub fn get_vec3(&self, key: &str) -> Option<Vec3> {
        if let AttrVal::Vec3Type(d) = self.get_val(key)? {
            return Some(Vec3::new(d[0] as f32, d[1] as f32, d[2] as f32));
        }
        None
    }

    pub fn get_dvec3(&self, key: &str) -> Option<DVec3> {
        if let AttrVal::Vec3Type(d) = self.get_val(key)? {
            return Some(DVec3::new(d[0], d[1], d[2]));
        }
        None
    }

    pub fn get_i32_vec(&self, key: &str) -> Option<Vec<i32>> {
        if let AttrVal::IntArrayType(d) = self.get_val(key)? {
            return Some(d.clone());
        }
        None
    }

    ///生成具有几何属性的element的shape
    pub fn create_brep_shape(&self, limit_size: Option<f32>) -> Option<Box<dyn BrepShapeTrait>> {
        let type_noun = self.get_type();
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

    /// 获取string属性数组，忽略为空的值
    pub fn get_attr_strings_without_default(&self, keys: &[&str]) -> Vec<String> {
        let mut results = vec![];
        for &attr_name in keys {
            if let Some(result) = self.get_val(attr_name) {
                match result {
                    AttrVal::StringType(v) => {
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
}
