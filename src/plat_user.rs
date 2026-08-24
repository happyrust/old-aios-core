use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct PuHuaPlatUser {
    #[serde(rename = "人员Id")]
    pub id: String,
    #[serde(rename = "人员工号")]
    pub work_num: String,
    #[serde(rename = "人员名称")]
    pub name: String,
    #[serde(rename = "部门")]
    pub depart: String,
}
