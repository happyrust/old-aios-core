use crate::RefU64;
use crate::attval::AttrVal::*;
use crate::pdms_types::AiosStrHash;
use crate::ref64vec::RefU64Vec;
use bevy_ecs::component::Component;
use bevy_reflect::Reflect;
use glam::{Vec3, bool, f32, f64, i32};
use serde_derive::{Deserialize, Serialize};

#[derive(
    Default,
    Serialize,
    Deserialize,
    Clone,
    Debug,
    Component,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub enum AttrVal {
    #[default]
    InvalidType,
    IntegerType(i32),
    StringType(String),
    DoubleType(f64),
    DoubleArrayType(Vec<f64>),
    StringArrayType(Vec<String>),
    BoolArrayType(Vec<bool>),
    IntArrayType(Vec<i32>),
    BoolType(bool),
    Vec3Type([f64; 3]),
    ElementType(String),
    WordType(String),

    RefU64Type(RefU64),
    StringHashType(AiosStrHash),
    RefU64Array(RefU64Vec),
}

impl From<AttrValAql> for AttrVal {
    fn from(value: AttrValAql) -> Self {
        match value {
            AttrValAql::InvalidType => InvalidType,
            AttrValAql::IntegerType(i) => IntegerType(i),
            AttrValAql::StringType(d) => StringType(d),
            AttrValAql::DoubleType(d) => DoubleType(d),
            AttrValAql::DoubleArrayType(d) => DoubleArrayType(d),
            AttrValAql::StringArrayType(d) => StringArrayType(d),
            AttrValAql::BoolArrayType(d) => BoolArrayType(d),
            AttrValAql::IntArrayType(d) => IntArrayType(d),
            AttrValAql::BoolType(d) => BoolType(d),
            AttrValAql::Vec3Type(d) => Vec3Type(d),
            AttrValAql::ElementType(d) => ElementType(d),
            AttrValAql::WordType(d) => WordType(d),
            // AttrValAql::RefU64Type(d) => { RefU64Type(d) }
            AttrValAql::StringHashType(d) => StringHashType(d),
            AttrValAql::RefU64Array(d) => RefU64Array(d),
        }
    }
}

impl AttrVal {
    #[inline]
    pub fn i32_value(&self) -> i32 {
        return match self {
            IntegerType(v) => *v,
            _ => 0,
        };
    }

    #[inline]
    pub fn i32_array_value(&self) -> Vec<i32> {
        return match self {
            IntArrayType(v) => v.to_vec(),
            _ => vec![],
        };
    }

    #[inline]
    pub fn double_value(&self) -> Option<f64> {
        return match self {
            DoubleType(v) => Some(*v),
            _ => None,
        };
    }

    #[inline]
    pub fn f32_value(&self) -> Option<f32> {
        return match self {
            DoubleType(v) => Some(*v as f32),
            _ => None,
        };
    }

    #[inline]
    pub fn vec3_value(&self) -> Option<[f64; 3]> {
        return match self {
            Vec3Type(v) => Some(*v),
            _ => None,
        };
    }

    #[inline]
    pub fn dvec_value(&self) -> Option<Vec<f64>> {
        return match self {
            DoubleArrayType(v) => Some(v.to_vec()),
            _ => None,
        };
    }

    #[inline]
    pub fn element_value(&self) -> Option<String> {
        return match self {
            ElementType(v) => Some(v.clone()),
            _ => None,
        };
    }

    #[inline]
    pub fn string_value(&self) -> String {
        return match self {
            StringType(v) => v.to_string(),
            WordType(v) => v.to_string(),
            _ => "unset".to_string(),
        };
    }

    #[inline]
    pub fn refno_value(&self) -> Option<RefU64> {
        return match self {
            RefU64Type(v) => Some(*v),
            _ => None,
        };
    }

    #[inline]
    pub fn string_hash_value(&self) -> Option<AiosStrHash> {
        return match self {
            StringHashType(v) => Some(v.clone()),
            _ => None,
        };
    }

    #[inline]
    pub fn refu64_vec_value(&self) -> Option<RefU64Vec> {
        return match self {
            RefU64Array(v) => Some(v.clone()),
            _ => None,
        };
    }

    #[inline]
    pub fn bool_value(&self) -> Option<bool> {
        return match self {
            BoolType(v) => Some(*v),
            _ => None,
        };
    }

    #[inline]
    pub fn get_val_as_reflect(&self) -> Box<dyn Reflect> {
        return match self {
            InvalidType => Box::new("unset".to_string()),
            StringType(v) | ElementType(v) | WordType(v) => Box::new(v.to_string()),
            RefU64Type(v) => Box::new(v.to_string()),
            BoolArrayType(v) => Box::new(v.clone()),
            IntArrayType(v) => Box::new(v.clone()),
            IntegerType(v) => Box::new(*v),
            DoubleArrayType(v) => Box::new(v.clone()),
            DoubleType(v) => Box::new(*v),
            BoolType(v) => Box::new(*v),
            StringHashType(v) => Box::new(*v),
            StringArrayType(v) => Box::new(v.iter().map(|x| x.to_string()).collect::<Vec<_>>()),
            Vec3Type(v) => Box::new(Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32)),
            RefU64Array(v) => Box::new(v.iter().map(|x| x.to_string()).collect::<Vec<_>>()),
        };
    }

    #[inline]
    pub fn get_val_as_string(&self) -> String {
        return match self {
            AttrVal::InvalidType => "unset".to_string(),
            IntegerType(v) => v.to_string(),
            StringType(v) => v.to_string(),
            DoubleType(v) => v.to_string(),
            DoubleArrayType(v) => serde_json::to_string(v).unwrap(),
            StringArrayType(v) => serde_json::to_string(v).unwrap(),
            BoolArrayType(v) => serde_json::to_string(v).unwrap(),
            IntArrayType(v) => serde_json::to_string(v).unwrap(),
            BoolType(v) => v.to_string(),
            Vec3Type(v) => serde_json::to_string(v).unwrap(),
            ElementType(v) => v.to_string(),
            WordType(v) => v.to_string(),
            RefU64Type(v) => v.to_string().to_string(),
            StringHashType(v) => v.to_string(),
            RefU64Array(v) => serde_json::to_string(v).unwrap(),
        };
    }

    pub fn get_val_as_string_csv(&self) -> String {
        return match self {
            AttrVal::InvalidType => "unset".to_string(),
            IntegerType(v) => v.to_string().replace(",", ";"),
            StringType(v) => v.to_string().replace(",", ";"),
            DoubleType(v) => v.to_string().replace(",", ";"),
            DoubleArrayType(v) => serde_json::to_string(v).unwrap().replace(",", ";"),
            StringArrayType(v) => serde_json::to_string(v).unwrap().replace(",", ";"),
            BoolArrayType(v) => serde_json::to_string(v).unwrap().replace(",", ";"),
            IntArrayType(v) => serde_json::to_string(v).unwrap().replace(",", ";"),
            BoolType(v) => v.to_string().replace(",", ";"),
            Vec3Type(v) => serde_json::to_string(v).unwrap().replace(",", ";"),
            ElementType(v) => v.to_string().replace(",", ";"),
            WordType(v) => v.to_string().replace(",", ";"),
            RefU64Type(v) => v.to_string().to_string().replace(",", ";"),
            StringHashType(v) => v.to_string().replace(",", ";"),
            RefU64Array(v) => serde_json::to_string(v).unwrap().replace(",", ";"),
        };
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Component)]
#[serde(untagged)]
pub enum AttrValAql {
    InvalidType,
    IntegerType(i32),
    StringType(String),
    DoubleType(f64),
    DoubleArrayType(Vec<f64>),
    StringArrayType(Vec<String>),
    BoolArrayType(Vec<bool>),
    IntArrayType(Vec<i32>),
    BoolType(bool),
    Vec3Type([f64; 3]),
    ElementType(String),
    WordType(String),
    // RefU64Type(RefU64),
    StringHashType(AiosStrHash),
    RefU64Array(RefU64Vec),
}

impl From<AttrVal> for AttrValAql {
    fn from(value: AttrVal) -> Self {
        match value {
            InvalidType => AttrValAql::InvalidType,
            IntegerType(i) => AttrValAql::IntegerType(i),
            StringType(d) => AttrValAql::StringType(d),
            DoubleType(d) => AttrValAql::DoubleType(d),
            DoubleArrayType(d) => AttrValAql::DoubleArrayType(d),
            StringArrayType(d) => AttrValAql::StringArrayType(d),
            BoolArrayType(d) => AttrValAql::BoolArrayType(d),
            IntArrayType(d) => AttrValAql::IntArrayType(d),
            BoolType(d) => AttrValAql::BoolType(d),
            Vec3Type(d) => AttrValAql::Vec3Type(d),
            ElementType(d) => AttrValAql::ElementType(d),
            WordType(d) => AttrValAql::WordType(d),
            RefU64Type(d) => AttrValAql::StringType(d.to_string().into()),
            StringHashType(d) => AttrValAql::StringHashType(d),
            RefU64Array(d) => AttrValAql::RefU64Array(d),
        }
    }
}
