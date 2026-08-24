use crate::data_center::RawHoleData;
use bevy_ecs::prelude::Resource;
use glam::Vec3;
use nom::character::streaming::char;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

//显示需创建ATTA的refno及name
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ATTAPos {
    pub pos: Vec<Vec3>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ATTAPosVec {
    pub data: Vec<ATTAPos>,
}

#[derive(Resource, Serialize, Deserialize, Clone, Debug, Default)]
pub struct VirtualHoleGraphNodeQuery {
    // node identifier
    // #[serde(rename = "Code")]
    pub _key: String,
    // node link
    #[serde(rename = "RelyItem")]
    pub rely_item: String,
    // node main item
    #[serde(rename = "MainItem")]
    pub main_item: String,
    // node speciality
    #[serde(rename = "Speciality")]
    pub speciality: String,
    // node position
    #[serde(rename = "Position")]
    pub position: String,
    // node work
    #[serde(rename = "HoleWork")]
    pub hole_work: String,
    // node work by
    #[serde(rename = "WorkBy")]
    pub work_by: String,
    // node time
    #[serde(rename = "Time")]
    pub time: String,
    // node shape
    #[serde(rename = "Shape")]
    pub shape: String,
    // node orientation
    #[serde(rename = "Ori")]
    pub ori: String,
    // node item reference
    #[serde(rename = "ItemREF")]
    pub item_ref: String,
    #[serde(rename = "RelyItemREF")]
    pub rely_item_ref: String,
    // node main item reference
    #[serde(rename = "MainItemREF")]
    pub main_item_ref: String,
    // node open item
    #[serde(rename = "OpenItem")]
    pub open_item: String,
    // node plug type
    #[serde(rename = "PlugType")]
    pub plug_type: String,
    // node height
    #[serde(rename = "SizeHeight")]
    pub size_height: f32,
    // node width
    #[serde(rename = "SizeWidth")]
    pub size_width: f32,
    // node bank width
    #[serde(rename = "BankWidth")]
    pub bank_width: f32,
    // node bank height
    #[serde(rename = "BankHeight")]
    pub bank_height: f32,
    // node hot distance
    #[serde(rename = "HotDis")]
    pub hot_dis: String,
    // node heat thickness
    #[serde(rename = "HeatThick")]
    pub heat_thick: f32,
    // node reference number
    #[serde(rename = "refNo")]
    pub refno: String,
    // node fitting reference number
    #[serde(rename = "FittRefNo")]
    pub fitt_refno: String,
    // node subsurface material
    #[serde(rename = "SubsMaterial")]
    pub subs_material: String,
    // node subsurface thickness
    #[serde(rename = "SubsThickness")]
    pub subs_thickness: f32,
    // node create
    #[serde(rename = "iCreate")]
    pub i_create: i32,
    // node subsurface type
    #[serde(rename = "SubsType")]
    pub subs_type: String,
    // node extent length 1
    #[serde(rename = "ExtentLength1")]
    pub extent_length1: f32,
    // node extent length 2
    #[serde(rename = "ExtentLength2")]
    pub extent_length2: f32,
    // node second
    #[serde(rename = "Second")]
    pub second: bool,
    // node rehole
    #[serde(rename = "ReHole")]
    pub re_hole: i32,
    // node note
    #[serde(rename = "Note")]
    pub note: String,
    #[serde(rename = "SizeThrowWall")]
    pub size_throw_wall: f32,
    #[serde(rename = "HoleBPID")]
    pub hole_bpid: String,
    #[serde(rename = "HoleBPVER")]
    pub hole_bpver: String,
    #[serde(rename = "RelyItemBPID")]
    pub rely_item_bpid: String,
    #[serde(rename = "RelyItemBPVER")]
    pub rely_item_bpver: String,
    #[serde(rename = "MainPipeline")]
    pub main_pipeline: String,
    #[serde(rename = "iFlowState")]
    pub i_flow_state: String,
    #[serde(rename = "hType")]
    pub h_type: String,
    #[serde(rename = "MainItems")]
    pub main_items: String,
    #[serde(rename = "MainItemRefs")]
    pub main_item_refs: String,
    #[serde(rename = "Version")]
    #[serde(default = "default_version_value")]
    pub version: char,
    // 校审状态
    #[serde(default)]
    #[serde(rename = "JSStatus")]
    pub js_status: String,
    // 只用于存储和查询的数据，不涉及任何业务
    #[serde(flatten, default)]
    pub map: HashMap<String, String>,
}

fn default_version_value() -> char {
    ' '
}

// This function gets the key of a virtual embed graph node from the
// intelld, code, and relyitem fields. The key is used to store
// the node in the graph and to search for the node in the graph.

#[derive(Resource, Serialize, Deserialize, Clone, Debug, Default)]
pub struct VirtualEmbedGraphNode {
    // the intelligence of the node
    pub _key: String,
    #[serde(rename = "RelyItem")]
    // 关联零件名称
    pub rely_item: String,
    #[serde(rename = "RelyItemRef")]
    // 关联零件标识
    pub rely_item_ref: String,
    #[serde(rename = "MainItem")]
    // 主零件名称
    pub main_item: String,
    #[serde(rename = "Speciality")]
    // 专业
    pub speciality: String,
    #[serde(rename = "Position")]
    // 位置
    pub position: String,
    #[serde(rename = "Ori")]
    // 方向
    pub ori: String,
    #[serde(rename = "Work")]
    // 操作人
    pub work: String,
    #[serde(rename = "WorkBy")]
    // 工作单位
    pub work_by: String,
    #[serde(rename = "Time")]
    // 时间
    pub time: String,
    #[serde(rename = "StanderType")]
    // 规格型号
    pub stander_type: String,
    #[serde(rename = "OpenItem")]
    // 开口部位
    pub open_item: String,
    #[serde(rename = "SizeLength")]
    // 尺寸-长度
    pub size_length: f32,
    #[serde(rename = "SizeWidth")]
    // 尺寸-宽度
    pub size_width: f32,
    #[serde(rename = "SizeThickness")]
    // 尺寸-厚度
    pub size_thickness: f32,
    #[serde(rename = "MinThickness")]
    // 最小厚度
    pub min_thickness: f32,
    #[serde(rename = "Load")]
    // 荷载
    pub load: String,
    #[serde(rename = "MinDistance")]
    // 最小距离
    pub min_distance: f32,
    #[serde(rename = "SubsMaterial")]
    // 补强材料
    pub subs_material: String,
    #[serde(rename = "FittID")]
    // 配件标识
    pub fitt_id: String,
    #[serde(rename = "REF")]
    // 参照标准
    pub ref_standard: String,
    #[serde(rename = "Shape")]
    // 形状
    pub shape: String,
    #[serde(rename = "Note")]
    // 备注
    pub note: String,
    #[serde(rename = "EmbedBPID")]
    // 嵌入零件标识
    pub embed_bpid: String,
    #[serde(rename = "EmbedBPVER")]
    // 嵌入零件版本
    pub embed_bpver: String,
    #[serde(rename = "RelyItemBPID")]
    // 关联零件标识
    pub rely_item_bpid: String,
    #[serde(rename = "RelyItemBPVER")]
    // 关联零件版本
    pub rely_item_bpver: String,
    #[serde(rename = "Form")]
    // 形式
    pub form: String,
    // 只用于存储和查询的数据，不涉及任何业务
    #[serde(flatten)]
    pub map: HashMap<String, String>,
}

#[derive(Resource, Serialize, Deserialize, Clone, Debug, Default)]
pub struct VirtualEmbedGraphNodeQuery {
    // the intelligence of the node
    pub _key: String,
    #[serde(rename = "RelyItem")]
    // 关联零件名称
    pub rely_item: String,
    #[serde(rename = "RelyItemRef")]
    // 关联零件标识
    pub rely_item_ref: String,
    #[serde(rename = "MainItem")]
    // 主零件名称
    pub main_item: String,
    #[serde(rename = "Speciality")]
    // 专业
    pub speciality: String,
    #[serde(rename = "Position")]
    // 位置
    pub position: String,
    #[serde(rename = "Ori")]
    // 方向
    pub ori: String,
    #[serde(rename = "Work")]
    // 操作人
    pub work: String,
    #[serde(rename = "WorkBy")]
    // 工作单位
    pub work_by: String,
    #[serde(rename = "Time")]
    // 时间
    pub time: String,
    #[serde(rename = "StanderType")]
    // 规格型号
    pub stander_type: String,
    #[serde(rename = "OpenItem")]
    // 开口部位
    pub open_item: String,
    #[serde(rename = "SizeLength")]
    // 尺寸-长度
    pub size_length: f32,
    #[serde(rename = "SizeWidth")]
    // 尺寸-宽度
    pub size_width: f32,
    #[serde(rename = "SizeThickness")]
    // 尺寸-厚度
    pub size_thickness: f32,
    #[serde(rename = "MinThickness")]
    // 最小厚度
    pub min_thickness: f32,
    #[serde(rename = "Load")]
    // 荷载
    pub load: String,
    #[serde(rename = "MinDistance")]
    // 最小距离
    pub min_distance: f32,
    #[serde(rename = "SubsMaterial")]
    // 补强材料
    pub subs_material: String,
    #[serde(rename = "FittID")]
    // 配件标识
    pub fitt_id: String,
    #[serde(rename = "REF")]
    // 参照标准
    pub ref_standard: String,
    #[serde(rename = "Shape")]
    // 形状
    pub shape: String,
    #[serde(rename = "Note")]
    // 备注
    pub note: String,
    #[serde(rename = "EmbedBPID")]
    // 嵌入零件标识
    pub embed_bpid: String,
    #[serde(rename = "EmbedBPVER")]
    // 嵌入零件版本
    pub embed_bpver: String,
    #[serde(rename = "RelyItemBPID")]
    // 关联零件标识
    pub rely_item_bpid: String,
    #[serde(rename = "RelyItemBPVER")]
    // 关联零件版本
    pub rely_item_bpver: String,
    #[serde(rename = "Form")]
    // 形式
    pub form: String,
    #[serde(rename = "Version")]
    #[serde(default = "default_version_value")]
    pub version: char,
    // 校审状态
    #[serde(default)]
    #[serde(rename = "JSStatus")]
    pub js_status: String,
    // 只用于存储和查询的数据，不涉及任何业务
    #[serde(flatten)]
    pub map: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum VirtualHoleGraphNodeJSStatus {
    // 发起流程
    Initiate,
    // 会签
    CounterSign,
    // 校核
    Review,
    // 审核
    Audit,
    // 审定
    Approve,
    // 批准
    FinalApprove,
    Unknown,
}

impl VirtualHoleGraphNodeJSStatus {
    pub fn from_chinese(chinese: &str) -> Self {
        match chinese {
            "发起流程" => Self::Initiate,
            "会签" => Self::CounterSign,
            "校核" => Self::Review,
            "审核" => Self::Audit,
            "审定" => Self::Approve,
            "批准" => Self::FinalApprove,
            _ => Self::Unknown,
        }
    }

    pub fn to_chinese(&self) -> &str {
        match self {
            Self::Initiate => "发起流程",
            Self::CounterSign => "会签",
            Self::Review => "校核",
            Self::Audit => "审核",
            Self::Approve => "审定",
            Self::FinalApprove => "批准",
            Self::Unknown => "未知",
        }
    }
}

//存储虚拟孔洞detail
#[derive(Resource, Serialize, Deserialize, Clone, Debug, Default)]
pub struct VirtualHoleHistoryData {
    pub data: RawHoleData,
}

//存储虚拟埋件detail
#[derive(Resource, Serialize, Deserialize, Clone, Debug, Default)]
pub struct VirtualEmbedHistoryData {
    pub data: VirtualEmbedGraphNodeQuery,
}

//存储校核人虚拟孔洞detail
#[derive(Resource, Serialize, Deserialize, Clone, Debug, Default)]
pub struct ReviewerHoleDetail {
    pub data: RawHoleData,
}

//存储校核人虚拟埋件detail
#[derive(Resource, Serialize, Deserialize, Clone, Debug, Default)]
pub struct ReviewerEmbedDetail {
    pub data: VirtualEmbedGraphNodeQuery,
}
