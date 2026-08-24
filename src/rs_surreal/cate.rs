//元件库相关的查询方法
use crate::consts::MAX_INSERT_LENGTH;
use crate::parsed_data::CateAxisParam;
use crate::pdms_types::{CataHashRefnoKV, EleTreeNode, PdmsElement};
use crate::pe::SPdmsElement;
use crate::ssc_setting::PbsElement;
use crate::table::ToTable;
use crate::tool::db_tool::{db1_dehash, db1_hash};
use crate::tool::math_tool::*;
use crate::{NamedAttrMap, RefU64};
use crate::{SUL_DB, SurlValue};
use crate::{get_default_pdms_db_info, graph::QUERY_DEEP_CHILDREN_REFNOS, types::*};
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

#[cached]
pub fn get_all_types_as_sql_string(att_types: Vec<String>) -> String {
    get_all_types_has(att_types)
        .into_iter()
        .map(|x| format!("'{}'", x))
        .join(",")
}

#[cached]
pub fn get_all_types_has(att_types: Vec<String>) -> Vec<String> {
    let info = get_default_pdms_db_info();
    let mut result = vec![];
    for att_type in &att_types {
        let spre_hash = db1_hash(att_type) as i32;
        let types = info
            .noun_attr_info_map
            .iter()
            .filter(|k| k.value().contains_key(&spre_hash))
            .map(|k| db1_dehash(*k.key() as u32))
            .collect::<Vec<String>>();
        result.extend(types);
    }
    result
}

/// 创建所有的元件库的 relate 关系
pub async fn build_cate_relate(replace_exist: bool) -> anyhow::Result<()> {
    let all_cate_types = get_all_types_as_sql_string(vec!["SPRE".to_string(), "CATR".to_string()]);
    // dbg!(&all_spres);
    let mut sql = if replace_exist {
        "delete cate_relate;".to_string()
    } else {
        "".to_string()
    };
    sql.push_str(&format!(
        r#"
        let $a = select id from only cate_relate limit 1;
        if $a == none {{
            for $table in [{}] {{
                for $e in (select REFNO, SPRE from type::table($table)) {{
                    let $id = type::thing("cate_relate", record::id($e.REFNO));
                    if $e.SPRE != NONE {{
                        relate ($e.REFNO)->$id->($e.SPRE);
                    }}
                }}
            }};

            for $e in (select REFNO, CATR from type::table("SPCO")) {{
                    let $id = type::thing("cate_relate", record::id($e.REFNO));
                    if $e.CATR != NONE {{
                        relate ($e.REFNO)->$id->($e.CATR);
                    }}
            }}
        }}
    "#,
        all_cate_types
    ));
    let mut response = SUL_DB.query(sql).await?;
    Ok(())
}

pub async fn query_ele_refnos_by_spres(spres: &[RefU64]) -> anyhow::Result<Vec<RefU64>> {
    if spres.is_empty() {
        return Ok(vec![]);
    }
    let sql = format!(
        r#"
        array::flatten(select value <-cate_relate.in from  [{}])
        "#,
        spres.into_iter().map(|x| x.to_pe_key()).join(",")
    );
    let mut response = SUL_DB.query(sql).await?;
    let refnos: Vec<RefU64> = response.take(0).unwrap();
    Ok(refnos)
}
