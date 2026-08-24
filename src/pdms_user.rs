use crate::pdms_types::PdmsElement;
use crate::types::*;
use bevy_ecs::prelude::Resource;
use serde_derive::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

//b_designer
#[derive(Debug, Clone, Default, Serialize, Deserialize, Resource)]
pub struct PdmsUser {
    pub user_name: String,
    pub chinese_name: String,
    pub user_major: String,
    pub b_designer: bool,
}

impl PdmsUser {
    pub fn get_name(&self) -> String {
        self.user_name.to_string()
    }

    pub fn get_major(&self) -> String {
        self.user_major.to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PdmsElementWithUser {
    pub refno: String,
    pub owner: RefU64,
    pub name: String,
    pub noun: String,
    pub version: u32,
    pub children_count: usize,
    pub user: String,
}

impl PdmsElementWithUser {
    pub fn from_pdms_element(pdms_element: PdmsElement, user: &str) -> Self {
        Self {
            refno: pdms_element.refno.to_string(),
            owner: pdms_element.owner,
            name: pdms_element.name,
            noun: pdms_element.noun,
            version: pdms_element.version,
            children_count: pdms_element.children_count,
            user: user.to_string(),
        }
    }
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PdmsElementWithMajor {
    #[serde_as(as = "DisplayFromStr")]
    pub refno: RefU64,
    #[serde_as(as = "DisplayFromStr")]
    pub owner: RefU64,
    pub name: String,
    pub noun: String,
    pub major: String,
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RefnoMajor {
    pub refno: String,
    pub major: String,
    pub major_classify: String,
}
