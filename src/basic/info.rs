use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,
)]
pub struct RefnoInfo {
    /// 参考号的ref0
    pub ref_0: u32,
    /// 对应db number
    pub db_no: u32,
}
