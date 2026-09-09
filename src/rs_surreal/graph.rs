use crate::aios_db_mgr::aios_mgr::AiosDBMgr;
use crate::error::{HandleError, init_deserialize_error, init_query_error};
use crate::noun_graph::*;
use crate::pdms_types::{EleTreeNode, PdmsElement};
use crate::pe::SPdmsElement;
use crate::query_ancestor_refnos;
use crate::ssc_setting::PbsElement;
use crate::three_dimensional_review::ModelDataIndex;
use crate::types::*;
use crate::{NamedAttrMap, RefU64, query_types, rs_surreal};
use crate::{SUL_DB, SurlValue};
use anyhow::anyhow;
use cached::proc_macro::cached;
use indexmap::IndexMap;
use itertools::Itertools;
use log::LevelFilter;
use parry3d::simba::scalar::SupersetOf;
use serde::{Deserialize, Serialize};
use simplelog::{ColorChoice, CombinedLogger, Config, TermLogger, TerminalMode, WriteLogger};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::str::FromStr;
use surrealdb::method::Stats;
use surrealdb::sql::Thing;

#[inline]
pub async fn query_filter_all_bran_hangs(refno: RefnoEnum) -> anyhow::Result<Vec<RefnoEnum>> {
    query_filter_deep_children(refno, &["BRAN", "HANG"]).await
}

/// ADR-053 direct 读路由。契约 1：深度优先、每层按记录里的成员原序，不重排。
pub async fn query_deep_children_refnos(refno: RefnoEnum) -> anyhow::Result<Vec<RefnoEnum>> {
    if let Some(ctx) = super::direct::active_direct_reads() {
        return ctx.provider().query_deep_children_refnos(refno).await;
    }
    if super::staging::active_staging_reads().is_some() {
        query_deep_children_refnos_uncached(refno).await
    } else {
        query_deep_children_refnos_cached(refno).await
    }
}

#[cached(name = "QUERY_DEEP_CHILDREN_REFNOS", result = true)]
pub async fn query_deep_children_refnos_cached(refno: RefnoEnum) -> anyhow::Result<Vec<RefnoEnum>> {
    query_deep_children_refnos_uncached(refno).await
}

async fn query_deep_children_refnos_uncached(refno: RefnoEnum) -> anyhow::Result<Vec<RefnoEnum>> {
    let pe_key = refno.to_pe_key();
    let sql = if refno.is_latest() {
        format!(
            r#"
             return array::flatten( object::values( (select
                  [id] as p0, <-pe_owner[? !in.deleted]<-(? as p1)<-pe_owner<-(? as p2)<-pe_owner<-(? as p3)<-pe_owner<-(? as p4)<-pe_owner<-(? as p5)<-pe_owner<-(? as p6)<-pe_owner<-(? as p7)<-pe_owner<-(? as p8)<-pe_owner<-(? as p9)<-pe_owner<-(? as p10)<-pe_owner<-(? as p11)
                   from only {pe_key} where record::exists(id))?:{{}} ) )[? !deleted];
            "#
        )
    } else {
        format!(
            r#"
                let $dt=<datetime>fn::ses_date({pe_key}); 
                let $r = array::flatten( object::values( (select
                    [id] as p0, <-pe_owner<-(? as p1)<-pe_owner<-(? as p2)<-pe_owner<-(? as p3)<-pe_owner<-(? as p4)<-pe_owner<-(? as p5)<-pe_owner<-(? as p6)<-pe_owner<-(? as p7)<-pe_owner<-(? as p8)<-pe_owner<-(? as p9)<-pe_owner<-(? as p10)<-pe_owner<-(? as p11)
                    from only fn::newest_pe({pe_key}) where record::exists(id))?:{{}} ) )[? (!deleted or <datetime>fn::ses_date(id)>$dt)];
                select value fn::find_pe_by_datetime($self.id, $dt) from $r;
            "#
        )
    };
    let idx = if refno.is_latest() { 0 } else { 2 };
    // println!("query_deep_children_refnos sql is {}", &sql);
    return match super::staging::data_db().query(&sql).await {
        Ok(mut response) => match response.take::<Vec<RefnoEnum>>(idx) {
            Ok(data) => Ok(data),
            Err(e) => {
                init_deserialize_error(
                    "Vec<RefnoEnum>",
                    &e,
                    &sql,
                    &std::panic::Location::caller().to_string(),
                );
                Err(anyhow!(e.to_string()))
            }
        },
        Err(e) => {
            init_query_error(&sql, &e, &std::panic::Location::caller().to_string());
            Err(anyhow!(e.to_string()))
        }
    };
}

#[cached(result = true)]
pub async fn query_deep_children_refnos_pbs(refno: Thing) -> anyhow::Result<Vec<Thing>> {
    let pe_key = refno.to_string();
    let sql = format!(
        r#"
             return array::flatten( object::values( select
                  [id] as p0, <-pbs_owner[? !in.deleted]<-(? as p1)<-pbs_owner<-(? as p2)<-pbs_owner<-(? as p3)<-pbs_owner<-(? as p4)<-pbs_owner<-(? as p5)<-pbs_owner<-(? as p6)<-pbs_owner<-(? as p7)<-pbs_owner<-(? as p8)<-pbs_owner<-(? as p9)<-pbs_owner<-(? as p10)<-pbs_owner<-(? as p11)
                   from only {pe_key} ) )[? !deleted];
            "#
    );
    return match super::staging::data_db().query(&sql).await {
        Ok(mut response) => match response.take::<Vec<Thing>>(0) {
            Ok(data) => Ok(data),
            Err(e) => {
                init_deserialize_error(
                    "Vec<RefU64>",
                    &e,
                    &sql,
                    &std::panic::Location::caller().to_string(),
                );
                Err(anyhow!(e.to_string()))
            }
        },
        Err(e) => {
            init_query_error(&sql, &e, &std::panic::Location::caller().to_string());
            Err(anyhow!(e.to_string()))
        }
    };
}

/// ADR-053 direct 读路由。契约 1：过滤不改变原序。
pub async fn query_filter_deep_children(
    refno: RefnoEnum,
    nouns: &[&str],
) -> anyhow::Result<Vec<RefnoEnum>> {
    if let Some(ctx) = super::direct::active_direct_reads() {
        return ctx
            .provider()
            .query_filter_deep_children(refno, nouns)
            .await;
    }
    let refnos = query_deep_children_refnos(refno).await?;
    let pe_keys = refnos.into_iter().map(|x| x.to_pe_key()).join(",");
    let nouns_str = rs_surreal::convert_to_sql_str_array(nouns);
    let sql = if nouns.is_empty() {
        format!(r#"select value id from [{pe_keys}]"#)
    } else {
        format!(r#"select value id from [{pe_keys}] where noun in [{nouns_str}]"#)
    };
    // println!("query_filter_deep_children sql is {}", &sql);
    match super::staging::data_db().query(&sql).with_stats().await {
        Ok(mut response) => {
            if let Some((stats, Ok(result))) = response.take::<Vec<RefnoEnum>>(0) {
                return Ok(result);
            }
        }
        Err(e) => {
            init_query_error(&sql, &e, &std::panic::Location::caller().to_string());
            return Err(anyhow!(e.to_string()));
        }
    }
    Ok(vec![])
}

/// ADR-053 direct 读路由。**复合读**：`query_filter_deep_children` 之后逐个取属性，
/// 两半都已在 provider 上（`query_filter_deep_children` / `get_named_attmap`），
/// 所以这里就地组合，不给 provider 再加一个方法 —— 否则同一套「先取谁再取谁」
/// 在两侧各存一份，正是 ADR-053 R1 说的语义漂移。
pub async fn query_filter_deep_children_atts(
    refno: RefnoEnum,
    nouns: &[&str],
) -> anyhow::Result<Vec<NamedAttrMap>> {
    if let Some(ctx) = super::direct::active_direct_reads() {
        let provider = ctx.provider();
        let refnos = provider.query_filter_deep_children(refno, nouns).await?;
        let mut atts = Vec::with_capacity(refnos.len());
        for child in refnos {
            atts.push(provider.get_named_attmap(child).await?);
        }
        return Ok(atts);
    }
    let refnos = query_deep_children_refnos(refno).await?;
    // dbg!(refnos.len());
    let mut atts = vec![];
    //需要使用chunk
    for chunk in refnos.chunks(200) {
        let pe_keys = chunk.iter().map(|x| x.to_pe_key()).join(",");
        let nouns_str = rs_surreal::convert_to_sql_str_array(nouns);
        let sql = format!(r#"select value refno.* from [{pe_keys}] where noun in [{nouns_str}]"#);
        // println!("query_filter_deep_children_atts sql is {}", &sql);
        match super::staging::data_db().query(&sql).with_stats().await {
            Ok(mut response) => {
                if let Some((stats, Ok(value))) = response.take::<surrealdb::Value>(0) {
                    let result: Vec<surrealdb::sql::Value> = value.into_inner().try_into().unwrap();
                    // dbg!(result.len());
                    atts.extend(result.into_iter().map(|x| x.into()));
                }
            }
            Err(e) => {
                init_query_error(&sql, &e, &std::panic::Location::caller().to_string());
                return Err(anyhow!(e.to_string()));
            }
        }
    }
    Ok(atts)
}

pub async fn query_ele_filter_deep_children_pbs(
    refno: Thing,
    nouns: &[&str],
) -> anyhow::Result<Vec<PbsElement>> {
    let refnos = query_deep_children_refnos_pbs(refno).await?;
    let pe_keys = refnos.into_iter().join(",");
    let nouns_str = rs_surreal::convert_to_sql_str_array(nouns);
    let sql = format!(r#"select * from [{pe_keys}] where noun in [{nouns_str}]"#);
    // println!("sql is {}", &sql);
    match super::staging::data_db().query(&sql).with_stats().await {
        Ok(mut response) => {
            if let Some((stats, Ok(result))) = response.take::<Vec<PbsElement>>(0) {
                return Ok(result);
            }
        }
        Err(e) => {
            init_query_error(&sql, &e, &std::panic::Location::caller().to_string());
            return Err(anyhow!(e.to_string()));
        }
    }
    Ok(vec![])
}

///深度查询
pub async fn query_ele_filter_deep_children(
    refno: RefnoEnum,
    nouns: &[&str],
) -> anyhow::Result<Vec<SPdmsElement>> {
    let refnos = query_deep_children_refnos(refno).await?;
    let pe_keys = refnos.into_iter().map(|x| x.to_pe_key()).join(",");
    let nouns_str = rs_surreal::convert_to_sql_str_array(nouns);
    let sql = format!(r#"select * from [{pe_keys}] where noun in [{nouns_str}]"#);
    // println!("sql is {}", &sql);
    let mut response = super::staging::data_db()
        .query(&sql)
        .with_stats()
        .await
        .unwrap();
    if let Some((stats, Ok(result))) = response.take::<Vec<SPdmsElement>>(0) {
        return Ok(result);
    }
    Ok(vec![])
}

/// Represents the SQL query used to retrieve values from a database.
/// The query is constructed dynamically based on the provided parameters.
/// It selects the `refno` values from a flattened array of objects,
/// where the `noun` values match the specified list of nouns.
pub async fn query_filter_deep_children_by_path(
    refno: RefnoEnum,
    nouns: &[&str],
) -> anyhow::Result<Vec<RefnoEnum>> {
    let end_noun = super::get_type_name(refno).await?;
    let nouns_str = rs_surreal::convert_to_sql_str_array(nouns);
    if let Some(relate_sql) = gen_noun_incoming_relate_sql(&end_noun, nouns) {
        let pe_key = refno.to_pe_key();
        let sql = format!(
            "select value refno from array::flatten(object::values(select {relate_sql} from only {pe_key})) where noun in [{nouns_str}]",
        );
        // println!("sql is {}", &sql);
        let mut response = super::staging::data_db().query(&sql).with_stats().await?;
        if let Some((stats, Ok(result))) = response.take::<Vec<RefnoEnum>>(0) {
            return Ok(result);
        }
    }
    Ok(vec![])
}

//过滤spre 和 catr 不能同时为空的类型,
///
/// ADR-053 direct 读路由。**混合读**：`SPRE`/`CATR` 非空是源模型，而 `filter` 那半
/// （还没有 `inst_relate` / `tubi_relate`）是产物。源模型半边问 provider，产物半边
/// 留在这里查库 —— 理由同 `query_group_by_cata_hash`，见规格 §2.3。
pub async fn query_deep_children_refnos_filter_spre(
    refno: RefnoEnum,
    filter: bool,
) -> anyhow::Result<Vec<RefnoEnum>> {
    if let Some(ctx) = super::direct::active_direct_reads() {
        let candidates = ctx
            .provider()
            .query_deep_children_refnos_filter_spre(refno)
            .await?;
        return retain_ungenerated(candidates, filter).await;
    }
    let pe_key = refno.to_pe_key();
    let mut sql = format!(
        r#"
            let $a = array::flatten( object::values( select
                  [id] as p0, <-pe_owner<-(? as p1)<-pe_owner<-(? as p2)<-pe_owner<-(? as p3)<-pe_owner<-(? as p4)<-pe_owner<-(? as p5)<-pe_owner<-(? as p6)<-pe_owner<-(? as p7)<-pe_owner<-(? as p8)<-pe_owner<-(? as p9)<-pe_owner<-(? as p10)<-pe_owner<-(? as p11)
                   from only {pe_key} ) );

            select value id from $a.refno where SPRE.id !=none || CATR.id != none
        "#,
    );
    if filter {
        sql.push_str(" and array::len(->inst_relate) = 0 and array::len(->tubi_relate) = 0");
    }
    let mut response = super::staging::data_db().query(&sql).await?;
    let result: Vec<RefnoEnum> = response.take(1)?;
    Ok(result)
}

async fn query_versioned_deep_children_filter_inst(
    refno: RefnoEnum,
    nouns: &[&str],
    filter: bool,
) -> anyhow::Result<Vec<RefnoEnum>> {
    let nouns_str = rs_surreal::convert_to_sql_str_array(nouns);
    let pe_key = refno.to_pe_key();
    let mut sql = format!(
        r#"
            let $a = array::flatten( object::values( select
                  [id] as p0, <-pe_owner<-(? as p1)<-pe_owner<-(? as p2)<-pe_owner<-(? as p3)
                  <-pe_owner<-(? as p4)<-pe_owner<-(? as p5)<-pe_owner<-(? as p6)<-pe_owner<-(? as p7)
                  <-pe_owner<-(? as p8)<-pe_owner<-(? as p9)<-pe_owner<-(? as p10)<-pe_owner<-(? as p11)
                   from only {pe_key} ) );

            select value refno from $a"#,
    );
    let mut add_where = false;
    if !nouns.is_empty() {
        if !sql.ends_with("where") {
            sql.push_str(" where ");
            add_where = true;
        }
        sql.push_str(format!(" noun in [{nouns_str}]").as_str());
    }
    if filter {
        if add_where {
            sql.push_str(" and ");
        } else {
            sql.push_str(" where ");
        }
        sql.push_str("array::len(->inst_relate) = 0 and array::len(->tubi_relate) = 0");
    }
    // println!("query_deep_children_filter_inst sql is: {}", &sql);
    let mut response = super::staging::data_db().query(&sql).await?;
    // dbg!(&response);
    let result: Vec<RefnoEnum> = response.take(1)?;
    Ok(result)
}

// #[cached(result = true)]
async fn query_deep_children_filter_inst(
    refno: RefU64,
    nouns: &[&str],
    filter: bool,
) -> anyhow::Result<Vec<RefU64>> {
    let nouns_str = rs_surreal::convert_to_sql_str_array(nouns);
    let pe_key = refno.to_pe_key();
    let mut sql = format!(
        r#"
            let $a = array::flatten( object::values( select
                  [id] as p0, <-pe_owner<-(? as p1)<-pe_owner<-(? as p2)<-pe_owner<-(? as p3)
                  <-pe_owner<-(? as p4)<-pe_owner<-(? as p5)<-pe_owner<-(? as p6)<-pe_owner<-(? as p7)
                  <-pe_owner<-(? as p8)<-pe_owner<-(? as p9)<-pe_owner<-(? as p10)<-pe_owner<-(? as p11)
                   from only {pe_key} ) );

            select value refno from $a where noun in [{nouns_str}]"#,
    );
    if filter {
        sql.push_str(" and array::len(->inst_relate) = 0 and array::len(->tubi_relate) = 0");
    }
    // println!("query_deep_children_filter_inst sql is: {}", &sql);
    let mut response = super::staging::data_db().query(&sql).await?;
    // dbg!(&response);
    let result: Vec<RefU64> = response.take(1)?;
    Ok(result)
}

/// 产物半边：从一批候选里留下**还没有生成过**的（`inst_relate` / `tubi_relate` 都为空）。
///
/// direct 上下文里也走 Surreal，而且是**有意的**：这问的是产物，文件里没有。
/// `filter == false` 时调用方不要这层过滤，原样返回。
async fn retain_ungenerated(
    candidates: Vec<RefnoEnum>,
    filter: bool,
) -> anyhow::Result<Vec<RefnoEnum>> {
    if !filter || candidates.is_empty() {
        return Ok(candidates);
    }
    let mut kept = Vec::with_capacity(candidates.len());
    for chunk in candidates.chunks(200) {
        let pe_keys = chunk.iter().map(|x| x.to_pe_key()).join(",");
        let sql = format!(
            "select value id from [{pe_keys}] \
             where array::len(->inst_relate) = 0 and array::len(->tubi_relate) = 0"
        );
        let mut response = super::staging::data_db().query(&sql).await?;
        let ungenerated: Vec<RefnoEnum> = response.take(0)?;
        kept.extend(ungenerated);
    }
    Ok(kept)
}

pub async fn query_multi_filter_deep_children(
    refnos: &[RefnoEnum],
    nouns: &[&str],
) -> anyhow::Result<HashSet<RefnoEnum>> {
    let mut result = HashSet::new();
    for &refno in refnos {
        let mut children = query_filter_deep_children(refno, nouns).await?;
        result.extend(children.drain(..));
    }
    Ok(result)
}

/// ADR-053 direct 读路由。**混合读，本规格里最容易做错的一条**：SQL 同时问
/// 「深层 children 里哪些是这些 noun」（源模型）与「其中哪些还没有 `inst_relate` /
/// `tubi_relate`」（产物）。产物只在 Surreal 里，文件侧读不到。
///
/// 整体交给 provider 会让「已生成过」判定永远为假 —— 重复生成或漏生成，
/// 而两模式产物 hash 仍一致，**双跑对拍照样绿**（规格 §2.3）。
pub async fn query_multi_deep_versioned_children_filter_inst(
    refnos: &[RefnoEnum],
    nouns: &[&str],
    filter: bool,
) -> anyhow::Result<BTreeSet<RefnoEnum>> {
    if refnos.is_empty() {
        return Ok(Default::default());
    }
    if let Some(ctx) = super::direct::active_direct_reads() {
        let candidates = ctx
            .provider()
            .deep_versioned_children_by_noun(refnos, nouns)
            .await?;
        let kept = retain_ungenerated(candidates.into_iter().collect(), filter).await?;
        return Ok(kept.into_iter().collect());
    }
    let mut result = BTreeSet::new();
    let mut skip_set = BTreeSet::new();
    let refno_nouns = query_types(&refnos.iter().map(|x| x.refno()).collect::<Vec<_>>()).await?;
    for (refno, refno_noun) in refnos.iter().zip(refno_nouns) {
        if !nouns.is_empty() {
            if let Some(r_noun) = &refno_noun {
                if skip_set.contains(r_noun) {
                    continue;
                }
                // //检查是否有和nouns有path往来
                let exist_path = nouns
                    .iter()
                    .any(|child| r_noun == child || !find_noun_path(child, r_noun).is_empty());
                // dbg!(exist_path);
                if !exist_path {
                    skip_set.insert(r_noun.to_owned());
                    continue;
                }
            } else {
                continue;
            }
        }
        //需要先过滤一遍，是否和nouns 的类型有path
        let mut children = query_versioned_deep_children_filter_inst(*refno, nouns, filter).await?;
        result.extend(children.drain(..));
    }
    Ok(result)
}

// #[cached(result = true)]
pub async fn query_multi_deep_children_filter_inst(
    refnos: &[RefU64],
    nouns: &[&str],
    filter: bool,
) -> anyhow::Result<HashSet<RefU64>> {
    if refnos.is_empty() {
        return Ok(Default::default());
    }
    let mut result = HashSet::new();
    let mut skip_set = HashSet::new();
    let refno_nouns = query_types(refnos).await?;
    for (refno, refno_noun) in refnos.iter().zip(refno_nouns) {
        // for refno in refnos {
        if let Some(r_noun) = &refno_noun {
            if skip_set.contains(r_noun) {
                continue;
            }
            // //检查是否有和nouns有path往来
            let exist_path = nouns
                .iter()
                .any(|child| r_noun == child || !find_noun_path(child, r_noun).is_empty());
            // dbg!(exist_path);
            if !exist_path {
                skip_set.insert(r_noun.to_owned());
                continue;
            }
        } else {
            continue;
        }
        //需要先过滤一遍，是否和nouns 的类型有path
        let mut children = query_deep_children_filter_inst(*refno, nouns, filter).await?;
        result.extend(children.drain(..));
    }
    Ok(result)
}

pub async fn query_multi_deep_children_filter_spre(
    refnos: Vec<RefnoEnum>,
    filter: bool,
) -> anyhow::Result<HashSet<RefnoEnum>> {
    let mut result = HashSet::new();
    for refno in refnos {
        let mut children = query_deep_children_refnos_filter_spre(refno, filter).await?;
        result.extend(children.drain(..));
    }
    Ok(result)
}

/// 查询指定refno的祖先节点中符合指定类型的节点
///
/// # 参数
/// * `refno` - 要查询的refno
/// * `nouns` - 要过滤的祖先节点类型列表
///
/// # 返回值
/// * `Vec<RefnoEnum>` - 符合指定类型的祖先节点refno列表
///
/// # 错误
/// * 如果查询失败会返回错误
pub async fn query_filter_ancestors(
    refno: RefnoEnum,
    nouns: &[&str],
) -> anyhow::Result<Vec<RefnoEnum>> {
    let start_noun = super::get_type_name(refno).await?;
    // dbg!(&start_noun);
    let nouns_str = nouns
        .iter()
        .map(|s| format!("'{s}'"))
        .collect::<Vec<_>>()
        .join(",");
    let ancestors = query_ancestor_refnos(refno).await?;
    let sql = format!(
        "select value refno from [{}] where refno.TYPE in [{nouns_str}] or refno.TYPEX in [{nouns_str}]",
        ancestors.iter().map(|x| x.to_pe_key()).join(","),
    );
    let mut response = super::staging::data_db().query(&sql).await?;
    let reuslt: Vec<RefnoEnum> = response.take(0)?;

    Ok(reuslt)
}

/// 查找选中节点以下的uda type
pub async fn get_uda_type_refnos_from_select_refnos(
    select_refnos: Vec<RefnoEnum>,
    uda_type: &str,
    base_type: &str,
) -> anyhow::Result<Vec<PdmsElement>> {
    let mut result = vec![];
    let uda_type = if uda_type.starts_with(":") {
        uda_type[1..].to_string()
    } else {
        uda_type.to_string()
    };
    for select_refno in select_refnos {
        let Ok(refnos) = query_filter_deep_children(select_refno, &[base_type]).await else {
            continue;
        };
        let refnos_str = refnos
            .into_iter()
            .map(|refno| refno.to_pe_key())
            .collect::<Vec<String>>()
            .join(",");
        let sql = format!("let $ukey = select value UKEY from UDET where DYUDNA = '{}';
        select refno,fn::default_name(id) as name,noun,owner,0 as children_count from [{}] where refno.TYPEX in $ukey;", &uda_type, refnos_str);
        match super::staging::data_db().query(&sql).await {
            Ok(mut response) => match response.take::<Vec<EleTreeNode>>(1) {
                Ok(query_r) => {
                    let mut query_r = query_r.into_iter().map(|x| x.into()).collect();
                    result.append(&mut query_r);
                }
                Err(e) => {
                    dbg!(&e.to_string());
                    init_deserialize_error(
                        "Vec<EleTreeNode>",
                        e,
                        &sql,
                        &std::panic::Location::caller().to_string(),
                    );
                }
            },
            Err(e) => {
                init_query_error(&sql, e, &std::panic::Location::caller().to_string());
                continue;
            }
        }
    }
    Ok(result)
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct WallContainsDoor {
    pub refno: RefU64,
    pub wall_name: String,
    pub fitts: Vec<WallDoorResult>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
struct WallDoorResult {
    pub refno: RefU64,
    pub name: String,
    pub wall: RefU64,
}

/// 根据选择节点找到下面的wall和wall上的门
pub async fn query_wall_doors(
    refno: RefU64,
) -> anyhow::Result<HashMap<RefU64, Vec<WallContainsDoor>>> {
    // 找到墙
    let mut walls_q = SUL_DB
        .query(format!(
            "select fn::find_deep_children_types(id,['STWALL', 'GWALL', 'WALL']) from {}",
            refno.to_pe_key()
        ))
        .await?;
    let walls: Vec<RefU64> = walls_q.take(0)?;
    let walls_key = walls.into_iter().map(|wall| wall.to_pe_key()).join(",");
    // 查询墙的name
    let mut name_q = SUL_DB
        .query(format!(
            "select fn::default_full_name(id) as name,id from [{}]",
            &walls_key
        ))
        .await?;
    let wall_names: Vec<ModelDataIndex> = name_q.take(0)?;
    let wall_names_map = wall_names
        .into_iter()
        .map(|wall| (wall.refno, wall.name))
        .collect::<HashMap<RefU64, String>>();
    // 找到墙下面的门洞
    let mut fitts_q = SUL_DB
        .query(format!("fn::find_door_from_wall([{}])", walls_key))
        .await?;
    let fitts: Vec<WallDoorResult> = fitts_q.take(0)?;
    // 将数据按墙分类
    let mut fitts_map = HashMap::new();
    for fitt in fitts {
        fitts_map
            .entry(fitt.wall)
            .or_insert_with(Vec::new)
            .push(fitt);
    }
    let mut map = HashMap::new();
    for (wall, wall_name) in wall_names_map {
        let Some(fitts) = fitts_map.get(&wall) else {
            continue;
        };
        map.entry(wall)
            .or_insert_with(Vec::new)
            .push(WallContainsDoor {
                refno,
                wall_name,
                fitts: fitts.clone(),
            })
    }
    Ok(map)
}
