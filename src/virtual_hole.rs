use crate::data_center::TiziVirtualHoleData;
use crate::geometry::GeoBasicType;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::types::*;
use bevy_ecs::event::Event;
use bevy_ecs::resource::Resource;
use bevy_transform::prelude::*;
use parry3d::bounding_volume::Aabb;
use serde::Deserialize;
use serde::Serialize;
use serde_with::DisplayFromStr;
use serde_with::serde_as;

#[derive(Resource, PartialEq, Default, Debug, Serialize, Deserialize)]
pub struct PersonnelInfo {
    #[serde(rename = "人员工号")]
    pub job_num: String,
    #[serde(rename = "人员名称")]
    pub name: String,
}

#[derive(PartialEq, Resource, Default, Debug, Serialize, Deserialize)]
pub struct PersonnelInfoVec {
    pub data: Vec<PersonnelInfo>,
}

/// 封堵材料返回的数据
#[derive(PartialEq, Resource, Debug, Serialize, Deserialize)]
pub struct PluggingHoleData {
    // 孔洞的参考号
    pub hole_refno: RefU64,
    // 孔洞的 name
    pub hole_name: String,
    // 孔洞的尺寸( 圆形:直径 ，方形:长 宽 )
    pub hole_size: HoleSize,
    // 孔洞两边的房间
    pub hole_rooms: (String, String),
    // 电缆的占用面积
    pub cable_area: f32,
    // 防火封堵材料面积
    pub plugging_area: f32,
    // 封堵体积
    pub plugging_volume: f32,
    // 封堵材料
    pub plugging_material: String,
}

#[derive(PartialEq, Resource, Debug, Serialize, Deserialize)]
pub enum HoleSize {
    Circle(CircleHoleSize),
    Rect(RectHoleSize),
}

/// 圆形孔洞尺寸（不含高度）
#[derive(PartialEq, Resource, Default, Debug, Serialize, Deserialize)]
pub struct CircleHoleSize {
    pub radius: f32,
    pub height: f32,
}

/// 方形孔洞尺寸（不含高度）
#[derive(PartialEq, Resource, Default, Debug, Serialize, Deserialize)]
pub struct RectHoleSize {
    pub length: f32,
    pub width: f32,
    pub height: f32,
}

/// 孔洞封堵方式
#[derive(PartialEq, Default, Debug, Serialize, Deserialize)]
pub struct HoleBlockageMethod {
    pub method: String,
    pub thickness: f32,
}

#[serde_as]
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct HoleInstInfo {
    #[serde_as(as = "DisplayFromStr")]
    pub refno: RefU64,
    pub inst: Vec<HoleEleGeosInfo>,
}

#[derive(
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Default,
    Resource,
)]
#[serde_as]
pub struct HoleEleGeosInfo {
    pub refno: RefU64,
    //todo 这里的数据是重复的，需要复用
    //有哪一些 geo insts 组成
    //也可以通过edge 来组合
    #[serde(default)]
    pub geo_hash: Option<String>,
    //是否可见
    pub visible: bool,
    //所属一般类型，ROOM、STRU、PIPE等, 用枚举处理
    // pub generic_type: PdmsGenericType,
    pub aabb: Option<Aabb>,
    //相对世界坐标系下的变换矩阵 rot, translation, scale
    pub transform: Transform,

    #[serde(default)]
    pub pts: Vec<i32>,

    #[serde(default)]
    pub geo_type: GeoBasicType,

    pub geo_param: PdmsGeoParam,
}

//显示提资记录以及当前该记录处于哪个校核阶段
#[derive(Resource, Serialize, Deserialize, Clone, Debug, Default)]
pub struct ShowReviewerData {
    pub record: TiziVirtualHoleData,
    pub reviewer: String,
}

//将Vec<ShowReviewerData>组织成一种资源插入ECS
#[derive(Resource, Serialize, Deserialize, Clone, Debug, Default)]
pub struct ShowReviewerDataVec {
    pub data: Vec<ShowReviewerData>,
}

//绘制虚拟孔洞埋件需要的数据
#[derive(Clone, Debug, Default, Serialize, Deserialize, Event)]
pub struct HoleGeoData {
    //图数据库的key
    pub key: String,
    //形状
    pub shape: String,
    //编号
    pub item_ref: String,
    //长
    pub size_length: f32,
    //宽
    pub size_width: f32,
    //位置
    pub position: String,
    //朝向
    pub ori: String,
    //对应墙板的refno
    pub rely_item_ref: String,
    //厚度
    pub size_throw_wall: f32,
}
