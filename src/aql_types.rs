use crate::types::*;
use serde_derive::{Deserialize, Serialize};

/// 只存储 _from 和 _to 的信息 不加其他边界信息
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct AqlEdge {
    pub _key: String,
    pub _from: String,
    pub _to: String,
}

impl AqlEdge {
    pub fn new(from: RefU64, to: RefU64, from_collection: &str, to_collection: &str) -> Self {
        let hash = from.hash_with_another_refno(to);
        AqlEdge {
            _key: hash.to_string(),
            _from: format!("{}/{}", from_collection, from.to_string()),
            _to: format!("{}/{}", to_collection, to.to_string()),
        }
    }
}
