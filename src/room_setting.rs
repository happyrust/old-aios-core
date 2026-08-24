use crate::ssc_setting::SiteData;
use bevy_ecs::prelude::Resource;
use serde::{Deserialize, Serialize};

#[derive(Resource, Serialize, Deserialize, Debug, Default, Clone)]
pub struct RoomSiteVec {
    pub data: Vec<SiteData>,
}

#[derive(Resource, Serialize, Deserialize, Debug, Default, Clone)]
pub struct RoomSelectedSiteVec {
    pub data: Vec<SiteData>,
}

#[derive(Resource, Serialize, Deserialize, Debug, Default, Clone)]
pub struct FlagRoomSiteVec {
    pub data: Vec<SiteData>,
}
