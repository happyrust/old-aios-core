use crate::attmap::AttrMap;
use crate::consts::EXPR_ATT_SET;
use crate::pdms_types::{AttrInfo, DifferenceValue};
use crate::tool::db_tool::db1_hash;
use crate::{NamedAttrMap, NamedAttrValue, pdms_types};
use dashmap::DashMap;
use glam::i32;
use serde_derive::{Deserialize, Serialize};

/// 显式属性的结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplicitAttr {
    /// 属性名称
    pub name: String,
    /// 属性值
    pub value: NamedAttrValue,
    /// 是否为UDA属性
    pub is_uda: bool,
    /// 哈希值
    pub hash_val: i32,
}

/// 完整的属性映射结构体
///
/// 包含两个主要字段:
/// - `attmap`: 常规的命名属性映射
/// - `explicit_attmap`: 显式的命名属性映射
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WholeAttMap {
    /// 常规的命名属性映射
    pub attmap: NamedAttrMap,
    /// 显式的命名属性映射
    pub explicit_attmap: NamedAttrMap,
    /// 显式的UDA属性映射
    pub uda_atts: Vec<ExplicitAttr>,
}

impl WholeAttMap {
    pub fn att_map(&self) -> &NamedAttrMap {
        &self.attmap
    }

    pub fn att_map_mut(&mut self) -> &mut NamedAttrMap {
        &mut self.attmap
    }

    pub fn explicit_attmap(&self) -> &NamedAttrMap {
        &self.explicit_attmap
    }

    pub fn explicit_attmap_mut(&mut self) -> &mut NamedAttrMap {
        &mut self.explicit_attmap
    }

    pub fn uda_atts(&self) -> &Vec<ExplicitAttr> {
        &self.uda_atts
    }

    pub fn uda_atts_mut(&mut self) -> &mut Vec<ExplicitAttr> {
        &mut self.uda_atts
    }

    pub fn refine(mut self, info_map: &DashMap<String, AttrInfo>) -> Self {
        let mut remove_list = vec![];
        for (noun, v) in self.explicit_attmap.map.iter() {
            if let Some(info) = info_map.get(noun) {
                self.attmap.insert(noun.clone(), v.clone());
                if info.offset > 0 {
                    remove_list.push(noun.clone());
                }
            }
        }
        for noun in remove_list {
            self.explicit_attmap.map.remove(&noun);
        }
        self
    }

    #[inline]
    pub fn into_bincode_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
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

    /// 将隐式属性和显示属性放到一个attrmap中
    #[inline]
    pub fn change_implicit_explicit_into_attr(self) -> NamedAttrMap {
        let mut map = self.attmap;
        for (k, v) in self.explicit_attmap.map {
            map.insert(k, v);
        }
        map
    }

    pub fn check_two_attr_difference(
        old_attr: WholeAttMap,
        new_attr: WholeAttMap,
    ) -> Vec<DifferenceValue> {
        vec![]
    }

    /// 将隐式属性和显示属性放到一个attrmap中
    #[inline]
    pub fn merge(&self) -> NamedAttrMap {
        let mut map = self.attmap.clone();
        for (k, v) in &self.explicit_attmap.map {
            if !map.contains_key(k) {
                map.insert(k.clone(), v.clone());
            }
        }
        map
    }
}

fn get_two_attr_map_difference(
    old_map: NamedAttrMap,
    mut new_map: NamedAttrMap,
) -> Vec<DifferenceValue> {
    let mut result = vec![];
    for (k, v) in old_map.map.into_iter() {
        let new_value = new_map.map.remove(&k);
        result.push(DifferenceValue {
            noun: k,
            old_value: Some(v.clone()),
            new_value,
        });
        continue;
    }
    if !new_map.map.is_empty() {
        for (k, v) in new_map.map.into_iter() {
            result.push(DifferenceValue {
                noun: k,
                old_value: None,
                new_value: Some(v),
            })
        }
    }
    result
}
