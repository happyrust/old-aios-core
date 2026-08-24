use crate::pdms_types::EleTreeNode;
use crate::pe::SPdmsElement;
use crate::tool::db_tool::{db1_dehash, get_uda_index, is_uda};
use crate::types::*;
use crate::{NamedAttrMap, RefU64};
use crate::{SUL_DB, SurlValue};
use cached::proc_macro::cached;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::f32::consts::E;
use std::sync::Mutex;
use surrealdb::sql::Thing;

#[cached]
pub async fn get_uda_refno(hash: i32) -> Option<RefU64> {
    if !is_uda(hash as _) {
        return None;
    }
    let uda_hash_name = db1_dehash(hash as _);
    let name = uda_hash_name[1..].to_string();
    let index = get_uda_index(hash as _);
    // dbg!(name, index);
    if let Ok(mut response) = SUL_DB
        .query(
            r#"
            let $a = select value id from only UDA where UKEY=$key limit 1;
            return if $a {
                return $a;
            } else {
                return (select value id from (select * from UDA where string::contains(UDNA, $name) order by UKEY))[$i];
            };
            "#,
        )
        .bind(("key", hash))
        .bind(("name", name))
        .bind(("i", index.unwrap_or_default()))
        .await
    {
        let result: Option<RefU64> = response.take(1).ok()?;
        return result;
    }
    None
}

#[cached]
pub async fn get_uda_name(hash: i32) -> Option<String> {
    get_uda_prop_as_string(hash, "UDNA".to_string()).await
}

#[cached]
pub async fn get_uda_type(hash: i32) -> Option<String> {
    get_uda_prop_as_string(hash, "UTYP".to_string()).await
}

#[cached]
pub async fn get_uda_prop_as_string(hash: i32, prop: String) -> Option<String> {
    if !is_uda(hash as _) {
        return None;
    }
    let uda_hash_name = db1_dehash(hash as _);
    let name = uda_hash_name[1..].to_string();
    let index = get_uda_index(hash as _).unwrap_or_default();
    // dbg!(name, index);
    let sql = format!(
        r#"
        let $a = select value {0} from only UDA where UKEY={hash} limit 1;
        return if $a {{
            return $a;
        }} else {{
            return (select value {0} from (select * from UDA where string::contains(UDNA, '{name}') order by UKEY))[{index}];
        }};
        "#,
        &prop
    );
    // println!("get_uda_prop_as_string: {}", sql);
    if let Ok(mut response) = SUL_DB
        .query(sql)
        // .bind(("key", hash))
        // .bind(("name", name))
        // .bind(("i", index.unwrap_or_default()))
        .await
    {
        let result: Option<String> = response.take(1).ok()?;
        return result;
    }
    None
}
