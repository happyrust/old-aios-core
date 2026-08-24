use crate::RefU64;
use bevy_transform::prelude::Transform;
use parry3d::bounding_volume::Aabb;
use serde_derive::{Deserialize, Serialize};

///Room元素
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct RoomElement {
    #[serde(rename = "id")]
    pub refno: RefU64,
    ///room名称
    pub name: String,
    ///room的aabb
    pub aabb: Option<Aabb>,
    ///room的panels
    pub panels: Vec<RoomPanelElement>,
}

//提前缓存，经常需要使用到的
///房间panel的信息, panel 的owner就是房间节点
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoomPanelElement {
    #[serde(rename = "id")]
    pub refno: RefU64,
    ///对应的aabb
    pub aabb: Aabb,
    //对应的几何体
    // pub inst_geo: EleInstGeo,
    ///对应的方位
    pub transform: Transform,
}
