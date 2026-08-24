use crate::consts::MAX_INSERT_LENGTH;
use crate::parsed_data::CateAxisParam;
use crate::pdms_types::{CataHashRefnoKV, EleTreeNode};
use crate::pe::SPdmsElement;
use crate::table::ToTable;
use crate::tool::db_tool::db1_dehash;
use crate::tool::math_tool::*;
use crate::{NamedAttrMap, RefU64};
use crate::{SUL_DB, SurlValue};
use crate::{graph::QUERY_DEEP_CHILDREN_REFNOS, types::*};
use cached::Cached;
use cached::proc_macro::cached;
use dashmap::DashMap;
use indexmap::IndexMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::DisplayFromStr;
use serde_with::serde_as;
use std::collections::{BTreeMap, HashMap};
use surrealdb::Surreal;
use surrealdb::engine::any::Any;

///遍历pe获得最新的sesno
pub async fn query_pe_latest_sesno(db_num: u32) -> anyhow::Result<u32> {
    let mut response = SUL_DB
        .query(format!(
            r#"object::values((select math::max(sesno) from pe where dbnum={db_num} group all)[0])[0];"#,
        ))
        .await?;
    let max_sesno: Option<u32> = response.take(0)?;
    Ok(max_sesno.unwrap_or_default())
}

///查询数据库中所有文件的最新sesno
pub async fn query_latest_sesno(db_num: u32) -> anyhow::Result<u32> {
    // object::values((select math::max(sesno) from pe where dbnum={db_num} group all)[0])[0];
    // "INSERT IGNORE INTO db_file_info (id, db_type, sesno, dbnum, dt) VALUES ('{}', '{}', '{}', '{}', '{}');",
    let mut response = SUL_DB
        .query(format!(
            r#"
            select value sesno from only db_file_info where dbnum={db_num} limit 1;
            "#,
        ))
        .await?;
    let sesno: Option<u32> = response.take(0)?;
    Ok(sesno.unwrap_or_default())
}
