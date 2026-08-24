use crate::types::*;
use bevy_ecs::prelude::{Component, Event};
use serde_derive::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RefnoStatusInfo {
    pub refno: RefU64,
    pub status: String,
    pub user: String,
    // 设置状态的时间
    pub time: String,
    // 备注
    pub note: String,
}

#[derive(Event, Clone, Debug, Default, Serialize, Deserialize)]
pub struct SetStateEvent {
    pub refnos: Vec<RefU64>,
    pub state_data: RefnoStatusInfo,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StatusInfoUI {
    pub refno: RefU64,
    pub status: String,
    pub user: String,
    // 设置状态的时间
    pub time: String,
    // 备注
    pub note: String,
    pub selected: bool,
}
