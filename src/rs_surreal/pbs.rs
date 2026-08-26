use crate::consts::MAX_INSERT_LENGTH;
use crate::parsed_data::CateAxisParam;
use crate::pdms_types::{CataHashRefnoKV, EleTreeNode, PdmsElement};
use crate::pe::SPdmsElement;
use crate::table::ToTable;
use crate::tool::db_tool::db1_dehash;
use crate::tool::math_tool::*;
use crate::{graph::QUERY_DEEP_CHILDREN_REFNOS, types::*};
use crate::{NamedAttrMap, RefU64};
use crate::{SurlValue, SUL_DB};
use cached::proc_macro::cached;
use cached::Cached;
use dashmap::DashMap;
use indexmap::IndexMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use std::collections::{BTreeMap, HashMap};
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use crate::ssc_setting::PbsElement;
use surrealdb::types::{RecordId as Thing, ToSql};

///查询pbs children 数据
pub async fn get_children_pbs_nodes(id: &Thing) -> anyhow::Result<Vec<PbsElement>> {
    let sql = format!(
        "select *, array::len(<-pbs_owner) as children_cnt from (select value in from pbs:⟨{}⟩<-pbs_owner);",
        id.key.to_sql()
    );
    // dbg!(&sql);
    let mut response = SUL_DB
        .query(sql)
        .await.unwrap();
    // dbg!(&response);
    let nodes: Vec<PbsElement> = response.take(0)?;
    Ok(nodes)
}

