use crate::types::*;
use chrono::{DateTime, Local, Utc};
use sea_orm::{DatabaseBackend, QueryTrait, Schema, entity::prelude::*};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug, Default, DeriveEntityModel)]
#[sea_orm(table_name = "dolt_log")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub commit_hash: String,
    pub committer: String,
    pub email: String,
    pub date: DateTime<Local>,
    pub message: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
