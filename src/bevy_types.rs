use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct UserPermissionHttp {
    pub project_code: String,
    pub user_code: String,
    pub is_system: bool,
    pub site: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct UserPermissionHash {
    pub project_code: String,
    pub user_code: String,
    pub is_system: bool,
    pub site: Vec<u32>,
}
