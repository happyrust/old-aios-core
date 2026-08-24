use crate::pdms_types::PdmsNodeTrait;
use crate::types::*;
use bevy_ecs::prelude::{Component, Resource};
use serde_derive::{Deserialize, Serialize};

/// 元数据管理各个字段在excel中的第几列
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MetadataManagerTreeNodeExcelIndex {
    pub user_code: Option<usize>,
    pub chinese_name: Option<usize>,
    pub english_name: Option<usize>,
    pub english_define: Option<usize>,
    pub chinese_define: Option<usize>,
    pub classify_code: Option<usize>,
    pub classify_name: Option<usize>,
    pub custom_item: Option<usize>,
    pub desc: Option<usize>,
    pub state: Option<usize>,
    pub owned_name: Option<usize>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, Component)]
pub struct MetadataManagerTreeNode {
    pub id: u64,
    pub owner: u64,
    pub user_code: String,
    pub chinese_name: String,
    pub english_name: String,
    pub english_define: String,
    pub chinese_define: String,
    pub classify_code: String,
    pub classify_name: String,
    pub custom_item: String,
    pub desc: String,
    // true 有效 false 无效
    pub state: bool,
    pub owned_name: String,
}

impl PdmsNodeTrait for MetadataManagerTreeNode {
    #[inline]
    fn get_refno(&self) -> RefU64 {
        RefU64(self.id)
    }

    #[inline]
    fn get_name(&self) -> &str {
        &self.chinese_name
    }

    #[inline]
    fn get_noun_hash(&self) -> u32 {
        0
    }

    #[inline]
    fn get_type_name(&self) -> &str {
        ""
    }

    #[inline]
    fn get_children_count(&self) -> usize {
        1
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MetadataManagerTableDataExcelIndex {
    pub code: Option<usize>,
    pub data_type: Option<usize>,
    pub data_constraint: Option<usize>,
    pub b_multi: Option<usize>,
    pub english_name: Option<usize>,
    pub chinese_name: Option<usize>,
    pub english_define: Option<usize>,
    pub chinese_define: Option<usize>,
    pub unit: Option<usize>,
    pub group: Option<usize>,
    pub custom_item: Option<usize>,
    pub desc: Option<usize>,
    pub state: Option<usize>,
    pub owned_name: Option<usize>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MetadataManagerTableData {
    pub id: u64,
    pub code: String,
    pub data_type: String,
    pub data_constraint: String,
    pub b_multi: bool,
    pub english_name: String,
    pub chinese_name: String,
    pub english_define: String,
    pub chinese_define: String,
    pub unit: String,
    pub group: String,
    pub custom_item: String,
    pub desc: String,
    pub state: bool,
    pub owned_name: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FileBytes {
    pub file_name: String,
    pub data: Vec<u8>,
}

#[derive(Resource, Debug, Default, Clone, Serialize, Deserialize)]
pub struct ShowMetadataManagerTableData {
    pub id: u64,
    pub old_code: String,
    pub new_code: String,
    pub data_type: String,
    pub data_constraint: String,
    pub b_multi: String,
    pub english_name: String,
    pub chinese_name: String,
    pub english_define: String,
    pub chinese_define: String,
    pub unit: String,
    pub group: String,
    pub custom_item: String,
    pub desc: String,
    pub state: String,
    pub owned_name: String,
    pub change: bool,
}

impl ShowMetadataManagerTableData {
    pub fn init(table_data: MetadataManagerTableData) -> ShowMetadataManagerTableData {
        let mut data = ShowMetadataManagerTableData::default();
        let group = if table_data.group.is_empty() {
            "[0]".to_string()
        } else {
            table_data.group
        };
        data.id = table_data.id;
        data.old_code = table_data.code.clone();
        data.new_code = table_data.code;
        data.english_name = table_data.english_name;
        data.chinese_name = table_data.chinese_name;
        data.english_define = table_data.english_define;
        data.chinese_define = table_data.chinese_define;
        data.b_multi = if table_data.b_multi {
            "Y".to_string()
        } else {
            "N".to_string()
        };
        data.data_type = table_data.data_type;
        data.data_constraint = table_data.data_constraint;
        data.unit = table_data.unit;
        data.desc = table_data.desc;
        data.group = group;
        data.change = false;
        data.state = if table_data.state {
            "有效".to_string()
        } else {
            "无效".to_string()
        };
        data.owned_name = table_data.owned_name;
        data
    }
}
