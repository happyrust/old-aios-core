use crate::attval::AttrVal;
use crate::tool::float_tool::f32_round_3;
use crate::{RefU64, RefU64Vec, SurlValue};
use bevy_ecs::component::Component;
use bevy_reflect::Reflect;
use glam::{Vec3, bool, f32, f64, i32};
use num_traits::{FromPrimitive, Num, One, Signed, ToPrimitive, Zero};
#[cfg(feature = "sea-orm")]
use sea_query::Value as SeaValue;
use serde::Deserializer;
use serde::{Deserialize, Serialize};
use serde_json::json;

///新的属性数据结构
#[derive(
    Serialize,
    // Eq,
    PartialEq,
    // Deserialize,
    Clone,
    Debug,
    Component,
    Default,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
// #[serde(untagged)]
pub enum NamedAttrValue {
    #[default]
    InvalidType,
    IntegerType(i32),
    StringType(String),
    F32Type(f32),
    F32VecType(Vec<f32>),
    Vec3Type(Vec3),
    StringArrayType(Vec<String>),
    BoolArrayType(Vec<bool>),
    IntArrayType(Vec<i32>),
    BoolType(bool),
    ElementType(String),
    WordType(String),
    RefU64Type(RefU64),
    RefU64Array(Vec<RefnoEnum>),
    LongType(i64),
    RefnoEnumType(RefnoEnum),
}

use serde::de::{self, EnumAccess, MapAccess, SeqAccess, Visitor};
use std::fmt;
use std::str::FromStr;
use std::vec::Vec;
use surrealdb::sql::{Array, Thing};

use super::RefnoEnum;

impl<'de> Deserialize<'de> for NamedAttrValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct NamedAttrValueVisitor;

        impl<'de> Visitor<'de> for NamedAttrValueVisitor {
            type Value = NamedAttrValue;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid NamedAttrValue")
            }

            fn visit_i32<E>(self, value: i32) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NamedAttrValue::IntegerType(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if value >= i32::MIN as i64 && value <= i32::MAX as i64 {
                    Ok(NamedAttrValue::IntegerType(value as i32))
                } else {
                    Ok(NamedAttrValue::LongType(value))
                }
            }

            fn visit_f32<E>(self, value: f32) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NamedAttrValue::F32Type(value))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NamedAttrValue::F32Type(value as f32))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NamedAttrValue::StringType(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NamedAttrValue::StringType(value))
            }

            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(NamedAttrValue::BoolType(value))
            }

            fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
            where
                A: EnumAccess<'de>,
            {
                let _ = data;
                Ok(NamedAttrValue::InvalidType)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut vec = Vec::new();
                let mut first_elem_type = None;
                //在这里能不能直接判断类型
                while let Some(elem) = seq.next_element::<serde_json::Value>()? {
                    if first_elem_type.is_none() {
                        first_elem_type = Some(match elem {
                            serde_json::Value::Bool(_) => "bool",
                            serde_json::Value::Number(ref n) if n.is_f64() => "f64",
                            serde_json::Value::Number(ref n) if n.is_u64() || n.is_i64() => "i32",
                            // serde_json::Value::Number(_) => "f64",
                            serde_json::Value::String(_) => "String",
                            serde_json::Value::Array(_) => "Array",
                            serde_json::Value::Object(_) => "Object",
                            _ => "InvalidType",
                        });
                    }
                    vec.push(elem);
                }

                match first_elem_type {
                    Some("f64") => Ok(NamedAttrValue::F32VecType(
                        vec.into_iter()
                            .filter_map(|v| v.as_f64().map(|f| f as f32))
                            .collect(),
                    )),
                    Some("String") => Ok(NamedAttrValue::StringArrayType(
                        vec.into_iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect(),
                    )),
                    Some("bool") => Ok(NamedAttrValue::BoolArrayType(
                        vec.into_iter().filter_map(|v| v.as_bool()).collect(),
                    )),
                    Some("i32") => Ok(NamedAttrValue::IntArrayType(
                        vec.into_iter()
                            .filter_map(|v| v.as_i64().map(|i| i as i32))
                            .collect(),
                    )),
                    // RefU64Array 可能需要特殊处理，这里仅作为示例
                    Some("Object") => Ok(NamedAttrValue::RefU64Array(
                        vec.into_iter()
                            .filter_map(|v| RefnoEnum::deserialize(v).ok())
                            .collect(),
                    )),
                    _ => Err(de::Error::custom("Unsupported array type")),
                }
            }

            fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let value = RefU64::deserialize(de::value::MapAccessDeserializer::new(map))?;
                Ok(NamedAttrValue::RefU64Type(value))
            }
        }

        deserializer.deserialize_any(NamedAttrValueVisitor)
    }
}

#[cfg(feature = "sea-orm")]
impl Into<Value> for NamedAttrValue {
    fn into(self) -> Value {
        match self {
            NamedAttrValue::IntegerType(val) => Value::Int(val.into()),
            NamedAttrValue::StringType(val)
            | NamedAttrValue::WordType(val)
            | NamedAttrValue::ElementType(val) => Value::String(Some(val.into())),
            NamedAttrValue::F32Type(val) => Value::Float(val.into()),
            NamedAttrValue::BoolType(val) => Value::Bool(val.into()),
            NamedAttrValue::RefU64Type(val) => Value::String(Some(val.to_string().into())),
            NamedAttrValue::F32VecType(val) => {
                let new_val = val.into_iter().map(|x| f32_round_3(x)).collect::<Vec<_>>();
                Value::Json(Some(json!(new_val).into()))
            }
            NamedAttrValue::Vec3Type(val) => Value::Json(Some(json!(val).into())),
            NamedAttrValue::StringArrayType(val) => Value::Json(Some(json!(val).into())),
            NamedAttrValue::BoolArrayType(val) => Value::Json(Some(json!(val).into())),
            NamedAttrValue::IntArrayType(val) => Value::Json(Some(json!(val).into())),

            _ => Value::String(None),
            // NamedAttrValue::InvalidType => {}
        }
    }
}

impl From<(&str, surrealdb::sql::Value)> for NamedAttrValue {
    fn from(tuple: (&str, surrealdb::sql::Value)) -> Self {
        let (tn, value) = tuple;
        match value {
            surrealdb::sql::Value::Number(val) => match tn {
                "REAL" => NamedAttrValue::F32Type(val.as_float() as _),
                _ => NamedAttrValue::IntegerType(val.as_int() as _),
            },
            surrealdb::sql::Value::Bool(val) => NamedAttrValue::BoolType(val),
            surrealdb::sql::Value::Strand(val) => NamedAttrValue::StringType(val.as_string()),
            surrealdb::sql::Value::Array(val) => match tn {
                "REAL" | "DIR" | "POS" => NamedAttrValue::F32VecType(
                    val.into_iter()
                        .map(|x| surrealdb::sql::Number::try_from(x).unwrap().as_float() as f32)
                        .collect(),
                ),
                "INT" => NamedAttrValue::IntArrayType(
                    val.into_iter()
                        .map(|x| surrealdb::sql::Number::try_from(x).unwrap().as_int() as _)
                        .collect(),
                ),
                "BOOL" => NamedAttrValue::BoolArrayType(
                    val.into_iter()
                        .map(|x| bool::try_from(x).unwrap())
                        .collect(),
                ),
                "TEXT" => NamedAttrValue::StringArrayType(
                    val.into_iter()
                        .map(|x| String::try_from(x).unwrap())
                        .collect(),
                ),

                "REF" => NamedAttrValue::RefU64Array(
                    val.into_iter()
                        .map(|x| RefnoEnum::from(x.to_string().as_str()))
                        .collect::<Vec<_>>(),
                ),

                _ => NamedAttrValue::InvalidType,
            },
            surrealdb::sql::Value::Thing(val) => {
                if let surrealdb::sql::Id::Array(_) = &val.id {
                    NamedAttrValue::RefnoEnumType(RefnoEnum::from(val))
                } else {
                    NamedAttrValue::RefU64Type(RefU64::from(val))
                }
            }
            surrealdb::sql::Value::Object(val) => {
                if let Some((key, v)) = val.into_iter().next() {
                    (tn, v).into()
                } else {
                    NamedAttrValue::InvalidType
                }
            }
            _ => NamedAttrValue::InvalidType,
        }
    }
}

#[test]
fn test_from_surreal() {
    let sql = vec!["17463_6376".to_string(), "17463_6379".to_string()];
    let value = surrealdb::sql::Value::Array(Array::from(sql));
    if let surrealdb::sql::Value::Array(value) = value {
        let r = value
            .into_iter()
            .map(|x| match x.clone().record() {
                None => RefnoEnum::from(x.to_string().as_str()),
                Some(id) => RefnoEnum::from(id),
            })
            .collect::<Vec<_>>();
        dbg!(&r);
    }
}

impl From<&AttrVal> for NamedAttrValue {
    fn from(v: &AttrVal) -> Self {
        use crate::attval::AttrVal::*;
        match v.clone() {
            InvalidType => Self::InvalidType,
            IntegerType(d) => Self::IntegerType(d),
            StringType(d) => Self::StringType(d),
            DoubleType(d) => {
                if d > f32::MAX as f64 {
                    Self::StringType(d.to_string())
                } else {
                    Self::F32Type(d as f32)
                }
            }
            DoubleArrayType(d) => Self::F32VecType(d.into_iter().map(|x| x as f32).collect()),
            StringArrayType(d) => Self::StringArrayType(d),
            BoolArrayType(d) => Self::BoolArrayType(d),
            IntArrayType(d) => Self::IntArrayType(d),
            BoolType(d) => Self::BoolType(d),
            Vec3Type(d) => Self::F32VecType(d.into_iter().map(|x| x as f32).collect()),
            ElementType(d) => Self::StringType(d),
            WordType(d) => Self::StringType(d),
            RefU64Type(d) => Self::RefU64Type(d),
            StringHashType(d) => Self::IntegerType(d as i32),
            RefU64Array(d) => Self::RefU64Array(d.into_iter().map(|x| x.into()).collect()),
        }
    }
}

impl From<AttrVal> for NamedAttrValue {
    fn from(v: AttrVal) -> Self {
        (&v).into()
    }
}

impl NamedAttrValue {
    #[inline]
    pub fn get_default_val(typ: &str) -> Self {
        match typ {
            "LOG" => Self::BoolType(false),
            "REAL" => Self::F32Type(0.0),
            "ELEMENT" => Self::RefU64Type(Default::default()),
            "TEXT" => Self::StringType("".into()),
            _ => Self::StringType("unset".into()),
        }
    }
}

impl NamedAttrValue {
    #[inline]
    pub fn i32_value(&self) -> i32 {
        return match self {
            Self::IntegerType(v) => *v,
            _ => 0,
        };
    }

    #[inline]
    pub fn i32_array_value(&self) -> Vec<i32> {
        return match self {
            Self::IntArrayType(v) => v.to_vec(),
            _ => vec![],
        };
    }

    // #[inline]
    // pub fn double_value(&self) -> Option<f64> {
    //     return match self {
    //         Self::F32Type(v) => Some(*v),
    //         _ => None,
    //     };
    // }

    #[inline]
    pub fn f32_value(&self) -> Option<f32> {
        return match self {
            Self::F32Type(v) => Some(*v),
            _ => None,
        };
    }

    #[inline]
    pub fn vec3_value(&self) -> Option<Vec3> {
        return match self {
            Self::Vec3Type(v) => Some(*v),
            _ => None,
        };
    }

    // #[inline]
    // pub fn dvec_value(&self) -> Option<Vec<f64>> {
    //     return match self {
    //         Self::DoubleArrayType(v) => Some(v.to_vec()),
    //         _ => None,
    //     };
    // }

    #[inline]
    pub fn element_value(&self) -> Option<String> {
        return match self {
            Self::ElementType(v) => Some(v.clone()),
            _ => None,
        };
    }

    #[inline]
    pub fn string_value(&self) -> String {
        return match self {
            Self::StringType(v) => v.to_string(),
            Self::WordType(v) => v.to_string(),
            _ => "unset".to_string(),
        };
    }

    #[inline]
    pub fn refno_value(&self) -> Option<RefU64> {
        return match self {
            Self::RefU64Type(v) => Some(*v),
            _ => None,
        };
    }

    #[inline]
    pub fn bool_value(&self) -> Option<bool> {
        return match self {
            Self::BoolType(v) => Some(*v),
            _ => None,
        };
    }

    #[inline]
    pub fn get_val_as_string(&self) -> String {
        return match self {
            Self::IntegerType(v) => v.to_string(),
            Self::StringType(v) => v.to_string(),
            Self::F32Type(v) => v.to_string(),
            Self::F32VecType(v) => serde_json::to_string(v).unwrap(),
            Self::StringArrayType(v) => serde_json::to_string(v).unwrap(),
            Self::BoolArrayType(v) => serde_json::to_string(v).unwrap(),
            Self::IntArrayType(v) => serde_json::to_string(v).unwrap(),
            Self::BoolType(v) => v.to_string(),
            Self::Vec3Type(v) => serde_json::to_string(v).unwrap(),
            Self::ElementType(v) => v.to_string(),
            Self::WordType(v) => v.to_string(),
            Self::RefU64Type(v) => v.to_string().to_string(),
            _ => "unset".to_string(),
        };
    }
}

impl NamedAttrValue {
    pub fn get_val_as_reflect(&self) -> Box<dyn Reflect> {
        return match self {
            NamedAttrValue::StringType(v)
            | NamedAttrValue::ElementType(v)
            | NamedAttrValue::WordType(v) => Box::new(v.to_string()),
            NamedAttrValue::BoolArrayType(v) => Box::new(v.clone()),
            NamedAttrValue::IntArrayType(v) => Box::new(v.clone()),
            NamedAttrValue::IntegerType(v) => Box::new(*v),
            NamedAttrValue::BoolType(v) => Box::new(*v),
            NamedAttrValue::StringArrayType(v) => {
                Box::new(v.iter().map(|x| x.to_string()).collect::<Vec<_>>())
            }
            NamedAttrValue::F32Type(v) => Box::new(*v),
            NamedAttrValue::F32VecType(v) => Box::new(v.clone()),
            NamedAttrValue::Vec3Type(v) => Box::new(vec![v.x, v.y, v.z]),
            NamedAttrValue::RefU64Type(r) => Box::new(r.to_slash_string()),
            _ => Box::new("unset".to_string()),
        };
    }
}

impl Into<serde_json::Value> for NamedAttrValue {
    fn into(self) -> serde_json::Value {
        match self {
            NamedAttrValue::IntegerType(d) => serde_json::Value::Number(d.into()),
            NamedAttrValue::F32Type(d) => serde_json::Value::Number(
                serde_json::Number::from_f64(d as _)
                    .unwrap_or(serde_json::Number::from_f64(0.0).unwrap()),
            ),
            NamedAttrValue::BoolType(b) => serde_json::Value::Bool(b),
            NamedAttrValue::StringType(s) | NamedAttrValue::WordType(s) => {
                if s.contains('\0') || s.contains("u0000") {
                    return serde_json::Value::String("".into());
                }
                serde_json::Value::String(s)
            }
            NamedAttrValue::ElementType(s) => {
                if s.contains('\0') || s.contains("u0000") {
                    return serde_json::Value::String("".into());
                }
                serde_json::Value::String(format!("pe:{}", s))
            }
            NamedAttrValue::RefU64Type(d) => {
                serde_json::Value::String(format!("pe:{}", d.to_string()))
            }
            NamedAttrValue::F32VecType(d) => {
                serde_json::Value::Array(d.into_iter().map(|x| x.into()).collect())
            }
            NamedAttrValue::Vec3Type(d) => {
                serde_json::Value::Array(d.to_array().into_iter().map(|x| x.into()).collect())
            }
            NamedAttrValue::StringArrayType(d) => {
                serde_json::Value::Array(d.into_iter().map(|x| x.into()).collect())
            }
            NamedAttrValue::BoolArrayType(d) => {
                serde_json::Value::Array(d.into_iter().map(|x| x.into()).collect())
            }
            NamedAttrValue::IntArrayType(d) => {
                serde_json::Value::Array(d.into_iter().map(|x| x.into()).collect())
            }
            _ => serde_json::Value::Null,
        }
    }
}
