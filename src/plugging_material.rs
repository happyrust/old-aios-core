use crate::types::*;
use bevy_ecs::prelude::Event;
use bevy_ecs::prelude::Resource;
use serde::Deserialize;
use serde::Serialize;
#[derive(Resource, Serialize, Deserialize, Debug, Default, Clone)]
pub struct PluggingData {
    pub own_refno: RefU64,
    pub refno: RefU64,
    pub name: String,
    pub size: String,
    pub room_1: String,
    pub room_2: String,
    pub height: f64,
    pub cable_area: f64,
    pub plugging_area: f64,
    pub plugging_volume: f64,
    pub materials: String,
}

#[derive(Resource, Serialize, Deserialize, Debug, Default, Clone)]
pub struct PluggingVec {
    pub data: Vec<PluggingData>,
}

///备份封堵配置数据
#[derive(Resource, Serialize, Deserialize, Debug, Default, Clone)]
pub struct PluggingMaterialBackVec {
    pub data: Vec<PluggingMaterial>,
}

#[derive(Resource, Serialize, Deserialize, Debug, Clone, Default)]
pub struct PluggingMaterialVec {
    pub data: Vec<PluggingMaterial>,
}

#[derive(Resource, Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct PluggingMaterial {
    pub plugging_type: String,
    pub material_type: String,
    pub hight: String,
    pub thickness: String,
    pub usage: String,
}

///发送事件，向数据库里更新封堵材料配置数据
#[derive(Serialize, Deserialize, Debug, Default, Clone, Event)]
pub struct UpdatePluggingSettingEvent {
    pub add_plugging_setting: Vec<PluggingMaterial>,
    pub delete_plugging_setting: Vec<PluggingMaterial>,
}
