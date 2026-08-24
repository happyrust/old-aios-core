use bevy_ecs::resource::Resource;
use serde::Deserialize;
use serde::Serialize;

/// 电气平台传入得信息，需要知道该name得设备 在传入得version到最新得版本是否发生变化
#[derive(PartialEq, Resource, Default, Debug, Serialize, Deserialize)]
pub struct VersionControlDataCenterRequest {
    pub name: String,
    pub version: String,
}

#[derive(PartialEq, Resource, Default, Debug, Serialize, Deserialize)]
pub struct VersionControlDataCenterResponse {
    #[serde(rename = "mod")]
    pub modify: Vec<String>,
    #[serde(rename = "del")]
    pub delete: Vec<String>,
    #[serde(rename = "err")]
    pub error: Vec<String>,
}
