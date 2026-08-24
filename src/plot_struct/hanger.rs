use crate::types::*;
use serde::{Deserialize, Serialize};

/// 支吊架出图所需的所有数据
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HangerData {
    // 支吊架名称
    pub _key: String,
    // 支吊架下面的所有参考号
    pub refnos: Vec<RefU64>,
    // 支吊架对应的 atta的名称 和 管道 bran 参考号
    pub bran_refno: Vec<(String, RefU64)>,
    // 管道的数据以及物项编码
    pub pipe_datas: Vec<HangerPipeData>,
    // 支吊架中图签 pcla 需要的数据
    pub pcla_datas: Vec<HangerPclaData>,
    // 支吊架中图签 sctn 需要的数据
    pub sctn_datas: Vec<HangerSctnData>,
    // 支吊架中图签 pfit 需要的数据
    pub pfit_datas: Vec<HangerPfitData>,
    // 支吊架中图签 pane 需要的数据
    pub pave_datas: Vec<HangerPaneData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HangerPipeData {
    pub mark: String,
    // atta name 的 最后一位
    pub number: String,
    // pipe的编号
    pub elevation: i32,
    // atta 世界坐标的 z坐标
    // 物项编码
    pub item_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HangerPclaData {
    pub spre_name: String,
    pub count: u32,
    pub unit_weight: u32,
    pub total_weight: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HangerSctnData {
    /// 截面尺寸
    pub across_section: String,
    // spref name 的 "/" 分割的最后一个
    /// 长度
    pub length: i32,
    /// 数量
    pub count: u32,
    pub unit_weight: u32,
    pub total_weight: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HangerPfitData {
    pub spre_name: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HangerPaneData {
    pub func_name: String,
    pub count: u32,
    pub unit_weight: f32,
    pub total_weight: f32,
}
