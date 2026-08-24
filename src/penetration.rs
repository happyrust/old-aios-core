use crate::types::*;
use bevy_ecs::resource::Resource;
use bevy_math::prelude::Vec3;
use serde::{Deserialize, Serialize};

//贯穿件结构体
#[derive(Resource, Serialize, Deserialize, Debug, Default, Clone)]
pub struct PenetrationData {
    pub owner_refno: RefU64,
    pub refno: RefU64,
    pub name: String,
    pub position: Vec3,
    pub x_deviation_angle: String,
    pub inner_room_num: String,
    pub outer_room_num: String,
    pub height_difference: f32,
}

//所有的贯穿件组织成资源
#[derive(Resource, Serialize, Deserialize, Debug, Default, Clone)]
pub struct PenetrationVec {
    pub data: Vec<PenetrationData>,
}
