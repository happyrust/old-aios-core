use crate::types::*;
use bevy_ecs::prelude::Component;
use bevy_ecs::prelude::Event;
use bevy_ecs::prelude::Resource;
use bevy_transform::prelude::Transform;
use serde::{Deserialize, Serialize};
use serde_with::DisplayFromStr;
use serde_with::serde_as;
use std::collections::HashMap;

#[derive(Debug, Default, Clone, Serialize, Deserialize, Resource)]
pub struct ReviewModelData {
    #[serde(rename = "id", alias = "KeyValue")]
    pub key_value: String,
    #[serde(rename = "ProjCode")]
    pub proj_code: String,
    #[serde(rename = "UserCode")]
    pub user_code: String,
    /// 选择节点所属的site
    #[serde(rename = "SiteCode")]
    pub site_code: String,
    /// 选择节点所属site的name
    #[serde(rename = "SiteName")]
    pub site_name: String,
    /// 当前处理人的角色  设计人员/校审人员
    #[serde(rename = "UserRole")]
    pub user_role: String,
    /// 选择节点的信息
    #[serde(rename = "ModelData")]
    pub model_data: ModelData,
    /// 截图数据
    #[serde(rename = "FlowPicData")]
    pub flow_pic_data: ThreeDimensionalReviewComment,
    /// 标题
    #[serde(rename = "Title")]
    pub title: String,
    /// 创建提资单人员(设计人员)
    #[serde(rename = "Initiator")]
    pub initiator: String,
    /// 是否完成编校审
    #[serde(rename = "Finished")]
    pub finished: bool,
}

impl ReviewModelData {
    pub fn to_arango_struct(self) -> ThreeDimensionalModelDataToArango {
        ThreeDimensionalModelDataToArango {
            key_value: self.key_value,
            proj_code: self.proj_code,
            user_code: self.user_code,
            site_code: self.site_code,
            site_name: self.site_name,
            user_role: self.user_role,
            model_data: self.model_data,
            flow_pic_data: self.flow_pic_data,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ThreeDimensionalModelDataToArango {
    #[serde(rename = "_key")]
    pub key_value: String,
    #[serde(rename = "ProjCode")]
    pub proj_code: String,
    #[serde(rename = "UserCode")]
    pub user_code: String,
    #[serde(rename = "SiteCode")]
    pub site_code: String,
    #[serde(rename = "SiteName")]
    pub site_name: String,
    #[serde(rename = "UserRole")]
    pub user_role: String,
    #[serde(rename = "ModelData")]
    pub model_data: ModelData,
    #[serde(rename = "FlowPicData")]
    pub flow_pic_data: ThreeDimensionalReviewComment,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ThreeDimensionalReviewComment {
    pub validation: Vec<ThreeDimensionalReviewData>,
    pub review: Vec<ThreeDimensionalReviewData>,
    pub approval: Vec<ThreeDimensionalReviewData>,
    pub endorsement: Vec<ThreeDimensionalReviewData>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ThreeDimensionalReviewData {
    pub comment: Vec<String>,
    pub reply: Vec<String>,
    pub associatedElement: Vec<RefU64>,
    pub cloudLine: String,
    #[serde(rename = "viewpoint")]
    pub camera_transform: Transform,
    pub image: Vec<u8>,
    pub status: bool,
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelDataIndex {
    pub refno: RefU64,
    pub name: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, Resource)]
pub struct ModelData {
    /// 选择校审内容的节点
    pub index: Vec<ModelDataIndex>,
    /// 选择校审内容的节点往上直到site的对应的参考号, map 例子: "SITE":"9304/2","ZONE":"***",Vec顺序与上面index一致
    pub data: Vec<HashMap<String, String>>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum VagueSearchCondition {
    #[default]
    And,
    Or,
    Not,
}

impl Into<String> for VagueSearchCondition {
    fn into(self) -> String {
        match self {
            VagueSearchCondition::And => "并且".to_string(),
            VagueSearchCondition::Or => "或者".to_string(),
            VagueSearchCondition::Not => "不含".to_string(),
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct VagueSearchRequest {
    // pub name: String,
    pub filter_refnos: Vec<RefU64>,
    // key : 过滤的类型 name , type 等  value: 0 : 过滤条件 and or not  1 : 过滤的值
    pub filter_condition: Vec<(String, (VagueSearchCondition, String))>,
}

/// 模糊查询导出为csv文件的数据
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct VagueSearchExportCsvData {
    pub key_word: String,
    pub result: String,
    pub belong_level: String,
    pub att_type: String,
}

impl VagueSearchExportCsvData {
    /// 将结构体数据转为Vec<String>，方便导出csv
    pub fn into_vec_string(self) -> Vec<String> {
        vec![self.key_word, self.result, self.belong_level, self.att_type]
    }
}

/// 模糊查询 从图数据库中查询到的需要导出的数据
#[serde_as]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct VagueSearchExportAqlData {
    #[serde_as(as = "DisplayFromStr")]
    pub refno: RefU64,
    pub name: String,
    pub level: Vec<String>,
    pub att_type: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct VagueSearchExportRequest {
    pub condition: String,
    pub refnos: Vec<RefU64>,
}

///显示范围内所有模型
#[derive(Clone, Debug, Default, Serialize, Deserialize, Event)]
pub struct ShowModelInRangeEvent {
    pub refno: RefU64,
    pub range: f32,
}

///编校审草图结构体
#[derive(Debug, Resource, Default, Clone, Serialize, Deserialize)]
pub struct ReviewBluePrint {
    pub file_name: String,
    pub data: Vec<u8>,
    pub user: String,
    pub time: String,
}

///得到编校审上传图纸Event
#[derive(Clone, Debug, Default, Event, Serialize, Deserialize)]
pub struct GetBluePrintEvent {
    pub filter_type: String,
    pub filter_value: String,
}
