use bevy_ecs::component::Component;
use bevy_reflect::Reflect;
#[cfg(feature = "sea-orm")]
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use serde::{Deserializer, Serializer};
use serde_with::DisplayFromStr;
use serde_with::serde_as;
use std::fmt::{Debug, Display, Formatter, Write};
use std::hash::Hash;
use std::ops::Deref;
use std::str::FromStr;
use std::{default, fmt, hash};

#[derive(Debug, PartialEq, Eq, derive_more::Display)]
pub struct ParseRefU64Error;

#[derive(
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Hash,
    Clone,
    Copy,
    Default,
    Component,
    Eq,
    PartialEq,
    PartialOrd,
    Ord,
    Reflect,
)]
pub struct RefU64(pub u64);

// slotmap::new_key_type! { struct RefU64; }

impl RefU64 {
    // 自定义序列化方法
    pub fn serialize_as_u64<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u64(self.0)
    }

    // 自定义反序列化方法
    pub fn deserialize_from_u64<'de, D>(deserializer: D) -> Result<RefU64, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u64::deserialize(deserializer)?;
        Ok(RefU64(value))
    }
}

impl From<u64> for RefU64 {
    fn from(d: u64) -> Self {
        Self(d)
    }
}

impl Serialize for RefU64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

#[derive(Debug, Serialize)]
// #[serde(untagged)]
enum RefnoVariant {
    RefThing(Thing),
    Str(String),
    Num(u64),
}

impl<'de> Deserialize<'de> for RefnoVariant {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de;
        use serde::de::Visitor;
        struct RefnoVariantVisitor;

        impl<'de> Visitor<'de> for RefnoVariantVisitor {
            type Value = RefnoVariant;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a Thing, a string, or an unsigned integer")
            }

            fn visit_str<E>(self, value: &str) -> Result<RefnoVariant, E>
            where
                E: de::Error,
            {
                Ok(RefnoVariant::Str(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<RefnoVariant, E>
            where
                E: de::Error,
            {
                Ok(RefnoVariant::Str(value))
            }

            fn visit_u64<E>(self, value: u64) -> Result<RefnoVariant, E>
            where
                E: de::Error,
            {
                Ok(RefnoVariant::Num(value))
            }

            fn visit_map<A>(self, map: A) -> Result<RefnoVariant, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                // 尝试将map反序列化为Thing
                let thing = Thing::deserialize(de::value::MapAccessDeserializer::new(map))?;
                Ok(RefnoVariant::RefThing(thing))
            }
        }

        deserializer.deserialize_any(RefnoVariantVisitor)
    }
}

impl<'de> Deserialize<'de> for RefU64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if let Ok(s) = RefnoVariant::deserialize(deserializer) {
            match s {
                RefnoVariant::RefThing(s) => Ok(s.into()),
                RefnoVariant::Str(s) => Self::from_str(s.as_str())
                    .map_err(|_| serde::de::Error::custom("refno parse string error")),
                RefnoVariant::Num(d) => Ok(Self(d)),
            }
        } else {
            return Err(serde::de::Error::custom("refno parse error"));
        }
    }
}

// impl<'de> Deserialize<'de> for RefU64 {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         let string = RefnoVariant::deserialize(deserializer)?;
//         // println!("RefU64: {}", &string);
//         // let value = RefnoVariant::deserialize(deserializer).unwrap();
//         // let thing = RefnoVariant::deserialize(deserializer).unwrap();
//         // dbg!(&thing);
//         Ok(Default::default())
//         // // if let Ok(s) = RefnoVariant::deserialize(deserializer) {
//         //     match s {
//         //         RefnoVariant::Thing(s) => Ok(s.into()),
//         //         RefnoVariant::Str(s) => Self::from_str(s.as_str())
//         //             .map_err(|_| serde::de::Error::custom("refno parse string error")),
//         //         RefnoVariant::Num(d) => Ok(Self(d)),
//         //     }
//         // } else {
//         //     return Err(serde::de::Error::custom("refno parse error"));
//         // }
//     }
// }

impl FromStr for RefU64 {
    type Err = ParseRefU64Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ts = s.split(['=', ':']).skip(1).next().unwrap_or(s);
        let nums = ts
            .split(['_', '/'])
            .filter_map(|x| x.parse::<u32>().ok())
            .collect::<Vec<_>>();
        if nums.len() == 2 {
            Ok(Self::from_two_nums(nums[0], nums[1]))
        } else if let Ok(d) = ts.parse::<u64>().map_err(|_| ParseRefU64Error) {
            Ok(Self(d))
        } else {
            Err(ParseRefU64Error)
        }
    }
}

impl From<Thing> for RefU64 {
    fn from(thing: Thing) -> Self {
        thing.id.to_raw().as_str().into()
    }
}

#[cfg(feature = "sea-orm")]
impl sea_orm::sea_query::ValueType for RefU64 {
    fn try_from(v: Value) -> Result<Self, sea_orm::sea_query::ValueTypeErr> {
        <String as sea_orm::sea_query::ValueType>::try_from(v)
            .map(|v| Self::from_str(&v).unwrap_or_default())
    }

    fn type_name() -> String {
        stringify!(StringVec).to_owned()
    }

    fn array_type() -> sea_orm::sea_query::ArrayType {
        sea_orm::sea_query::ArrayType::String
    }

    fn column_type() -> sea_orm::sea_query::ColumnType {
        sea_orm::sea_query::ColumnType::Text
    }
}

#[cfg(feature = "sea-orm")]
impl Into<sea_orm::Value> for RefU64 {
    fn into(self) -> sea_orm::Value {
        let string: String = self.to_string();
        sea_orm::Value::String(Some(Box::new(string)))
    }
}

#[cfg(feature = "sea-orm")]
impl sea_orm::TryGetable for RefU64 {
    fn try_get_by<I: sea_orm::ColIdx>(
        res: &sea_orm::QueryResult,
        idx: I,
    ) -> Result<Self, sea_orm::TryGetError> {
        <String as sea_orm::TryGetable>::try_get_by(res, idx)
            .map(|v| Self::from_str(&v).unwrap_or_default())
    }
}

impl From<&str> for RefU64 {
    fn from(s: &str) -> Self {
        Self::from_str(s).unwrap_or_default()
    }
}

impl From<String> for RefU64 {
    fn from(s: String) -> Self {
        Self::from_str(s.as_str()).unwrap_or_default()
    }
}

impl hash::Hash for ArchivedRefU64 {
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl PartialEq for ArchivedRefU64 {
    fn eq(&self, other: &ArchivedRefU64) -> bool {
        self.0 == other.0
    }
}

impl Eq for ArchivedRefU64 {}

impl Deref for RefU64 {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Debug for RefU64 {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_string().as_str())
    }
}

impl Into<u64> for RefU64 {
    fn into(self) -> u64 {
        self.0
    }
}

impl From<&RefI32Tuple> for RefU64 {
    fn from(n: &RefI32Tuple) -> Self {
        let bytes: Vec<u8> = [n.get_0().to_be_bytes(), n.get_1().to_be_bytes()].concat();
        let v = u64::from_be_bytes(bytes[..8].try_into().unwrap());
        Self(v)
    }
}

impl From<RefI32Tuple> for RefU64 {
    fn from(n: RefI32Tuple) -> Self {
        let bytes: Vec<u8> = [n.get_0().to_be_bytes(), n.get_1().to_be_bytes()].concat();
        let v = u64::from_be_bytes(bytes[..8].try_into().unwrap());
        Self(v)
    }
}

impl From<&[u8]> for RefU64 {
    fn from(input: &[u8]) -> Self {
        Self(u64::from_be_bytes(input[..8].try_into().unwrap()))
    }
}

impl Into<Vec<u8>> for RefU64 {
    fn into(self) -> Vec<u8> {
        self.0.to_be_bytes().to_vec().into()
    }
}

// impl BytesTrait for RefU64 {
//     fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
//         Ok(self.0.to_be_bytes().to_vec().into())
//     }
//
//     fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
//         Ok(Self(u64::from_be_bytes(bytes[..8].try_into()?)))
//     }
// }

impl Display for RefU64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}_{}", self.get_0(), self.get_1())
    }
}

impl RefU64 {
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.get_0() > 0
    }

    pub fn is_unset(&self) -> bool {
        self.get_0() == 0
    }

    #[inline]
    pub fn get_sled_key(&self) -> [u8; 8] {
        self.0.to_be_bytes()
    }

    #[inline]
    pub fn to_pe_key(&self) -> String {
        self.to_table_key("pe")
    }

    #[inline]
    pub fn to_pe_thing(&self) -> Thing {
        ("pe".to_string(), self.to_string()).into()
    }

    #[inline]
    pub fn to_pe_versioned_key(&self, version: i32) -> String {
        format!("pe_v:{}_{}", &self.to_string(), version)
    }

    #[inline]
    pub fn to_pbs_key(&self) -> String {
        format!("pbs:{}", &self.0.to_string())
    }

    #[inline]
    pub fn to_pbs_thing(&self) -> Thing {
        ("pbs".to_string(), self.to_string()).into()
    }

    pub fn to_type_key(&self, noun: &str) -> String {
        format!("{}:{}", noun, &self.to_string())
    }

    #[inline]
    pub fn to_inst_relate_key(&self) -> String {
        self.to_table_key("inst_relate")
    }

    #[inline]
    pub fn to_inst_relate_history_key(&self, version: i32) -> String {
        format!("inst_relate:{}_{}", &self.to_string(), version)
    }

    #[inline]
    pub fn to_table_key(&self, tbl: &str) -> String {
        format!("{tbl}:{}", &self.to_string())
    }

    #[inline]
    pub fn to_table_history_key(&self, tbl: &str, version: i32) -> String {
        format!("{tbl}_v:{}_{}", &self.to_string(), version)
    }

    #[inline]
    pub fn get_0(&self) -> u32 {
        let bytes = self.0.to_be_bytes();
        u32::from_be_bytes(bytes[0..4].try_into().unwrap())
    }

    #[inline]
    pub fn get_1(&self) -> u32 {
        let bytes = self.0.to_be_bytes();
        u32::from_be_bytes(bytes[4..8].try_into().unwrap())
    }

    #[inline]
    pub fn get_u32_hash(&self) -> u32 {
        use hash32::{FnvHasher, Hasher};
        let mut fnv = FnvHasher::default();
        self.hash(&mut fnv);
        fnv.finish32()
    }

    #[inline]
    pub fn to_slash_string(&self) -> String {
        format!("{}/{}", self.get_0(), self.get_1())
    }

    #[inline]
    pub fn to_e3d_id(&self) -> String {
        format!("={}/{}", self.get_0(), self.get_1())
    }

    #[inline]
    pub fn from_two_nums(n: u32, m: u32) -> Self {
        Self(((n as u64) << 32) + m as u64)
    }

    #[inline]
    pub fn to_array_id(&self) -> String {
        format!("['{}']", self.to_string())
    }

    #[inline]
    pub fn hash_with_another_refno(&self, another_refno: RefU64) -> u64 {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(&self.0, &mut hash);
        std::hash::Hash::hash(&another_refno.0, &mut hash);
        std::hash::Hasher::finish(&hash)
    }

    #[inline]
    pub fn hash_with_str(&self, input: &str) -> u64 {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(&self.0, &mut hash);
        std::hash::Hash::hash(&input, &mut hash);
        std::hash::Hasher::finish(&hash)
    }

    /// 返回图数据库的id形式 例如 pdms_eles/1232_5445
    pub fn to_arangodb_ids(collection_name: &str, refnos: Vec<RefU64>) -> Vec<String> {
        refnos
            .into_iter()
            .map(|refno| format!("{}/{}", collection_name, refno.to_string()))
            .collect()
    }

    #[inline]
    pub fn format_url_name(&self, col: &str) -> String {
        format!("{}/{}", col, self.to_string())
    }

    /// 将参考号字符串类型集合转为 Vec<RefU64>
    pub fn from_refno_strs(refno_strs: &Vec<String>) -> Vec<Self> {
        refno_strs
            .iter()
            .filter_map(|refno| Self::from_str(refno).ok())
            .collect()
    }
    /// 转换为pdms的形式
    pub fn to_pdms_str(&self) -> String {
        format!("{}/{}", self.get_0(), self.get_1())
    }
}

///pdms的参考号
#[derive(Serialize, Deserialize, Clone, Debug, Default, Copy, Eq, PartialEq, Hash)]
pub struct RefI32Tuple(pub (i32, i32));

use crate::cache::mgr::BytesTrait;
use anyhow::anyhow;
#[cfg(feature = "sea-orm")]
use sea_orm::sea_query::ValueType;
use std::string::String;
use surrealdb::sql::Thing;

impl Into<String> for RefI32Tuple {
    fn into(self) -> String {
        String::from(format!("{}/{}", self.get_0(), self.get_1()))
    }
}

impl From<&[u8]> for RefI32Tuple {
    fn from(input: &[u8]) -> Self {
        Self::new(
            i32::from_be_bytes(input[0..4].try_into().unwrap()),
            i32::from_be_bytes(input[4..8].try_into().unwrap()),
        )
    }
}

impl From<&str> for RefI32Tuple {
    fn from(s: &str) -> Self {
        let x: Vec<i32> = s
            .split('/')
            .map(|x| x.parse::<i32>().unwrap_or_default())
            .collect();
        Self::new(x[0], x[1])
    }
}

impl From<&RefU64> for RefI32Tuple {
    fn from(n: &RefU64) -> Self {
        let n = n.0.to_be_bytes();
        Self((
            i32::from_be_bytes(n[..4].try_into().unwrap()),
            i32::from_be_bytes(n[4..].try_into().unwrap()),
        ))
    }
}

impl RefI32Tuple {
    #[inline]
    pub fn new(ref_0: i32, ref_1: i32) -> Self {
        Self { 0: (ref_0, ref_1) }
    }

    #[inline]
    pub fn get_0(&self) -> i32 {
        self.0.0
    }

    #[inline]
    pub fn get_1(&self) -> i32 {
        self.0.1
    }
}

/// 参考号和 sesno 的对应关系
#[derive(
    Default,
    Debug,
    Clone,
    Copy,
    Eq,
    PartialEq,
    Hash,
    Serialize,
    Deserialize,
    Ord,
    PartialOrd,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct RefnoSesno {
    pub refno: RefU64,
    pub sesno: u32,
}

impl RefnoSesno {
    pub fn new(refno: RefU64, sesno: u32) -> Self {
        Self { refno, sesno }
    }

    pub fn to_pe_key(&self) -> String {
        if self.sesno == 0 {
            self.refno.to_pe_key()
        } else {
            format!("pe:{}", self.to_string())
        }
    }

    #[inline]
    pub fn to_table_key(&self, tbl: &str) -> String {
        format!("{tbl}:{}", self.to_string())
    }
}

impl From<&str> for RefnoSesno {
    fn from(value: &str) -> Self {
        serde_json::from_str(value).unwrap_or_default()
    }
}

impl ToString for RefnoSesno {
    fn to_string(&self) -> String {
        format!("['{}',{}]", self.refno.to_string(), self.sesno)
    }
}

impl From<(String, u32)> for RefnoSesno {
    fn from(value: (String, u32)) -> Self {
        let refno = RefU64::from_str(&value.0).unwrap_or_default();
        Self::new(refno, value.1)
    }
}

impl Into<RefU64> for RefnoSesno {
    fn into(self) -> RefU64 {
        self.refno
    }
}

impl Into<u32> for RefnoSesno {
    fn into(self) -> u32 {
        self.sesno
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    Eq,
    PartialEq,
    Hash,
    Serialize,
    Ord,
    PartialOrd,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Component,
)]
#[serde(untagged)]
pub enum RefnoEnum {
    Refno(RefU64),
    SesRef(RefnoSesno),
}

impl Default for RefnoEnum {
    fn default() -> Self {
        RefnoEnum::Refno(Default::default())
    }
}

// impl ToString for RefnoEnum {
//     fn to_string(&self) -> String {
//         match self {
//             RefnoEnum::Refno(refno) => refno.to_string(),
//             RefnoEnum::SesRef(ses_ref) => ses_ref.to_string(),
//         }
//     }
// }

impl From<&str> for RefnoEnum {
    fn from(value: &str) -> Self {
        let value = value.trim();
        if value.starts_with('[') {
            RefnoEnum::SesRef(RefnoSesno::from(value))
        } else if value.contains(",") {
            let v: Vec<&str> = value.split(',').collect();
            if v.len() == 2 {
                let refno = RefU64::from_str(v[0]).unwrap_or_default();
                let sesno: u32 = v[1].parse().unwrap_or_default();
                RefnoEnum::SesRef(RefnoSesno::new(refno, sesno))
            } else {
                RefnoEnum::default()
            }
        } else {
            RefnoEnum::Refno(RefU64::from_str(value).unwrap_or_default())
        }
    }
}

//实现 deserialize
impl<'de> Deserialize<'de> for RefnoEnum {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if let Ok(s) = RefnoVariant::deserialize(deserializer) {
            match s {
                RefnoVariant::RefThing(s) => Ok(s.into()),
                RefnoVariant::Str(s) => RefU64::from_str(s.as_str())
                    .map(|x| x.into())
                    .map_err(|_| serde::de::Error::custom("RefnoEnum parse string error")),
                RefnoVariant::Num(d) => Ok(RefnoEnum::Refno(RefU64(d))),
            }
        } else {
            return Err(serde::de::Error::custom("RefnoEnum parse error"));
        }
    }
}

impl RefnoEnum {
    #[inline]
    pub fn to_pe_key(&self) -> String {
        match self {
            RefnoEnum::Refno(refno) => refno.to_pe_key(),
            RefnoEnum::SesRef(ses_ref) => ses_ref.to_pe_key(),
        }
    }

    #[inline]
    pub fn sesno(&self) -> Option<u32> {
        match self {
            RefnoEnum::Refno(_) => None,
            RefnoEnum::SesRef(ses_ref) => Some(ses_ref.sesno),
        }
    }

    #[inline]
    pub fn is_history(&self) -> bool {
        self.sesno().is_some()
    }

    #[inline]
    pub fn refno(&self) -> RefU64 {
        match self {
            RefnoEnum::Refno(refno) => *refno,
            RefnoEnum::SesRef(ses_ref) => ses_ref.refno,
        }
    }

    #[inline]
    pub fn ref_refno(&self) -> &RefU64 {
        match self {
            RefnoEnum::Refno(refno) => refno,
            RefnoEnum::SesRef(ses_ref) => &ses_ref.refno,
        }
    }

    #[inline]
    pub fn to_table_key(&self, tbl: &str) -> String {
        match self {
            RefnoEnum::Refno(refno) => refno.to_table_key(tbl),
            RefnoEnum::SesRef(ses_ref) => ses_ref.to_table_key(tbl),
        }
    }

    #[inline]
    pub fn is_valid(&self) -> bool {
        self.refno().is_valid()
    }

    #[inline]
    pub fn to_inst_relate_key(&self) -> String {
        match self {
            RefnoEnum::Refno(refno) => refno.to_inst_relate_key(),
            RefnoEnum::SesRef(ses_ref) => format!("inst_relate:{}", ses_ref.to_string()),
        }
    }

    #[inline]
    pub fn latest(&self) -> Self {
        self.refno().into()
    }

    #[inline]
    pub fn is_latest(&self) -> bool {
        match self {
            RefnoEnum::Refno(_) => true,
            RefnoEnum::SesRef(_) => false,
        }
    }

    #[inline]
    pub fn to_e3d_id(&self) -> String {
        match self {
            RefnoEnum::Refno(refno) => refno.to_e3d_id(),
            RefnoEnum::SesRef(ses_ref) => ses_ref.refno.to_e3d_id(),
        }
    }

    #[inline]
    pub fn to_pdms_str(&self) -> String {
        format!("{}/{}", self.refno().get_0(), self.refno().get_1())
    }

    #[inline]
    pub fn hash_with_another_refno(&self, another_refno: RefnoEnum) -> u64 {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(&self, &mut hash);
        std::hash::Hash::hash(&another_refno, &mut hash);
        std::hash::Hasher::finish(&hash)
    }

    #[inline]
    pub fn to_array_id(&self) -> String {
        match self {
            RefnoEnum::Refno(refno) => refno.to_array_id(),
            RefnoEnum::SesRef(ses_ref) => ses_ref.to_string(),
        }
    }

    #[inline]
    pub fn to_array_zero_id(&self) -> String {
        format!("['{}', 0]", self.refno().to_string())
    }

    #[inline]
    pub fn to_normal_str(&self) -> String {
        if self.sesno().is_some() {
            format!("{}_{}", self.refno().to_string(), self.sesno().unwrap())
        } else {
            self.refno().to_string()
        }
    }
}

impl From<RefU64> for RefnoEnum {
    fn from(value: RefU64) -> Self {
        Self::Refno(value)
    }
}

impl From<RefnoSesno> for RefnoEnum {
    fn from(value: RefnoSesno) -> Self {
        Self::SesRef(value)
    }
}

impl From<(RefU64, u32)> for RefnoEnum {
    fn from(v: (RefU64, u32)) -> Self {
        Self::SesRef(RefnoSesno::new(v.0, v.1))
    }
}

impl From<(String, u32)> for RefnoEnum {
    fn from(value: (String, u32)) -> Self {
        let refno = RefU64::from_str(&value.0).unwrap_or_default();
        Self::SesRef(RefnoSesno::new(refno, value.1))
    }
}

impl From<(&str, u32)> for RefnoEnum {
    fn from(value: (&str, u32)) -> Self {
        let refno = RefU64::from_str(value.0).unwrap_or_default();
        Self::SesRef(RefnoSesno::new(refno, value.1))
    }
}

impl From<Thing> for RefnoEnum {
    fn from(value: Thing) -> Self {
        //检查是否是 array
        if let surrealdb::sql::Id::Array(array) = &value.id {
            let refno = array.get(0).cloned().unwrap_or_default().to_string();
            let sesno: u32 = array
                .get(1)
                .cloned()
                .unwrap_or_default()
                .try_into()
                .unwrap_or_default();
            Self::SesRef(RefnoSesno::new(refno.into(), sesno))
        } else {
            Self::Refno(RefU64::from_str(&value.id.to_raw()).unwrap_or_default())
        }
    }
}

impl Into<RefU64> for RefnoEnum {
    fn into(self) -> RefU64 {
        match self {
            RefnoEnum::Refno(refno) => refno,
            RefnoEnum::SesRef(ses_ref) => ses_ref.refno,
        }
    }
}

impl Display for RefnoEnum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RefnoEnum::Refno(refno) => write!(f, "{}_{}", refno.get_0(), refno.get_1()),
            RefnoEnum::SesRef(ses_ref) => write!(
                f,
                "['{}_{}', {}]",
                ses_ref.refno.get_0(),
                ses_ref.refno.get_1(),
                ses_ref.sesno
            ),
        }
    }
}

#[macro_export]
macro_rules! pe_key {
    ($s:expr) => {
        crate::RefnoEnum::from($s)
    };
}

#[macro_export]
macro_rules! to_table_key {
    ($refno:expr, $table:expr) => {
        $refno.to_table_key($table)
    };
}

#[macro_export]
macro_rules! to_table_keys {
    ($refnos:expr, $table:expr) => {
        $refnos
            .into_iter()
            .map(|x| x.latest().to_table_key($table))
            .collect::<Vec<_>>()
    };
}
