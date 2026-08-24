use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct NegativeEles {
    pub _key: String,
    pub mesh: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NegativeEdges {
    pub _key: String,
    pub _from: String,
    pub _to: String,
}
