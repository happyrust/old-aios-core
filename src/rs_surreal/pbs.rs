use crate::consts::MAX_INSERT_LENGTH;
use crate::parsed_data::CateAxisParam;
use crate::pdms_types::{CataHashRefnoKV, EleTreeNode, PdmsElement};
use crate::pe::SPdmsElement;
use crate::ssc_setting::PbsElement;
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
use surrealdb::sql::Thing;

///查询pbs children 数据
pub async fn get_children_pbs_nodes(id: &Thing) -> anyhow::Result<Vec<PbsElement>> {
    let sql = format!(
        "select *, array::len(<-pbs_owner) as children_cnt from (select value in from pbs:⟨{}⟩<-pbs_owner);",
        id.id.to_raw()
    );
    // dbg!(&sql);
    let mut response = SUL_DB.query(sql).await.unwrap();
    // dbg!(&response);
    let nodes: Vec<PbsElement> = response.take(0)?;
    Ok(nodes)
}
