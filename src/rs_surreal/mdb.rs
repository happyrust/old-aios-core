use crate::pdms_types::EleTreeNode;
use crate::pe::SPdmsElement;
use crate::{get_db_option, helper, types::*};
use crate::{NamedAttrMap, RefnoEnum};
use crate::{SurlValue, SUL_DB};
use cached::proc_macro::cached;
use indexmap::IndexMap;
use itertools::Itertools;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::f32::consts::E;
use std::sync::Mutex;

#[derive(IntoPrimitive, TryFromPrimitive, Clone, Copy, Hash, Eq, PartialEq, Debug)]
#[repr(u8)]
pub enum DBType {
    DESI = 1,
    CATA = 2,
    PROP = 3,
    ISOD = 4,
    PADD = 5,
    DICT = 6,
    ENGI = 7,
    SCHE = 14,
    UNSET,
}

/// Executes a query on the SUL_DB database to retrieve information from MDB and DB tables.
///
/// # Arguments
///
/// * `mdb` - The name of the MDB to query.
/// * `db_type` - The type of DB to filter by.
///
/// # Returns
///
/// The response containing the refno, noun, name, owner, and children_count fields from the query.
#[cached(result = true)]
pub async fn get_mdb_world_site_ele_nodes(
    mdb: String,
    module: DBType,
) -> anyhow::Result<Vec<EleTreeNode>> {
    let db_type: u8 = module.into();
    let sql = format!(
        r#"
        let $dbnos = select value (select value DBNO from CURD.refno where STYP == {db_type}) from only MDB where NAME == "{mdb}" limit 1;
        let $a = (select value id from (select REFNO.id as id, array::find_index($dbnos, REFNO.dbnum) as o from WORL where REFNO.dbnum in $dbnos order by o));
        select refno, noun, name, owner, array::len(select value in from <-pe_owner) as children_count from array::flatten(select value in from $a<-pe_owner) where noun='SITE';
        "#,
        db_type = db_type,
        mdb = mdb
    );
    // println!("sql is {}", &sql);
    let mut response = SUL_DB.query(&sql).await.unwrap();
    // dbg!(&response);
    let mut nodes: Vec<EleTreeNode> = response.take(2)?;
    for (i, node) in nodes.iter_mut().enumerate() {
        node.order = i as _;
        if node.name.is_empty() {
            node.name = format!("SITE {}", i + 1);
        }
    }
    //检查名称，如果没有给名字的，需要给上默认值, todo 后续如果是删除了又增加，名称后面的数字可能会继续增加
    Ok(nodes)
}

pub async fn create_mdb_world_site_pes_table(mdb: String, module: DBType) -> anyhow::Result<bool> {
    let db_type: u8 = module.into();
    let mut response = SUL_DB
        .query(r#"
            let $dbnos = select value (select value DBNO from CURD.refno where STYP=$db_type) from only MDB where NAME=$mdb limit 1;
            let $a = (select value id from (select REFNO.id as id, array::find_index($dbnos, REFNO.dbnum) as o from WORL where REFNO.dbnum in $dbnos order by o));
            array::flatten(select value in.* from $a<-pe_owner[? in.noun='SITE'])
        "#)
        .bind(("mdb", mdb))
        .bind(("db_type", db_type))
        .await?;
    let sites: Vec<SPdmsElement> = response.take(2)?;
    if sites.is_empty() {
        return Ok(false);
    }
    let mut relate_sql = String::new();
    let mdb_world = sites[0].owner.to_pe_key();
    for (i, site) in sites.into_iter().enumerate() {
        relate_sql.push_str(&format!(
            "relate {}->site_relate:[{}, {i}]->{};",
            site.refno.to_pe_key(),
            &mdb_world,
            &mdb_world
        ));
    }

    Ok(true)
}

pub async fn query_type_refnos_by_dbnums(
    nouns: &[&str],
    dbnums: &[u32],
) -> anyhow::Result<Vec<RefnoEnum>> {
    let mut result = vec![];
    for noun in nouns {
        let sql = if dbnums.is_empty() {
            format!("select value id from {noun}")
        } else {
            format!(
                "select value id from {noun} where REFNO.dbnum in [{}]",
                dbnums.into_iter().map(|x| x.to_string()).join(",")
            )
        };
        let mut response = SUL_DB.query(&sql).await?;
        let refnos: Vec<RefnoEnum> = response.take(0)?;
        result.extend(refnos);
    }
    Ok(result)
}

///通过dbnum过滤指定类型的参考号
/// 通过has_children 指定是否需要有children，方便跳过一些不变要的节点
/// todo 在属性里直接加上DBNO这个属性，而不是需要去pe里去取
pub async fn query_type_refnos_by_dbnum(
    nouns: &[&str],
    dbnum: u32,
    has_children: Option<bool>,
    only_history: bool,
) -> anyhow::Result<Vec<RefnoEnum>> {
    let mut result = vec![];
    for noun in nouns {
        let table = if only_history {
            format!("{noun}_H")
        } else {
            format!("{noun}")
        };
        let sql = match has_children {
            Some(true) => {
                format!("select value id from {table} where REFNO.dbnum={dbnum} and (REFNO<-pe_owner.in)[0] != none")
            }
            Some(false) => {
                format!("select value id from {table} where REFNO.dbnum={dbnum} and (REFNO<-pe_owner.in)[0] == none")
            }
            None => {
                format!("select value id from {table} where REFNO.dbnum={dbnum}")
            }
        };
        // println!("query_type_refnos_by_dbnum sql: {}", sql);
        let mut response = SUL_DB.query(&sql).await?;
        let refnos: Vec<RefnoEnum> = response.take(0)?;
        result.extend(refnos);
    }
    Ok(result)
}

//额外检查SPRE  和 CATR 不能同时为空
pub async fn query_use_cate_refnos_by_dbnum(
    nouns: &[&str],
    dbnum: u32,
    only_history: bool,
) -> anyhow::Result<Vec<RefnoEnum>> {
    let mut result = vec![];
    for noun in nouns {
        let table = if only_history {
            format!("{noun}_H")
        } else {
            format!("{noun}")
        };
        let sql =
            format!("select value id from {table} where REFNO.dbnum={dbnum} and (SPRE != none or CATR != none)");
        let mut response = SUL_DB.query(&sql).await?;
        let refnos: Vec<RefnoEnum> = response.take(0)?;
        result.extend(refnos);
    }
    Ok(result)
}

//去掉父类型是BRAN 和 HANGER的
// pub async fn query_type_refnos_by_dbnum_exclude_bran_hang(nouns: &[&str], dbnum: u32) -> anyhow::Result<Vec<RefnoEnum>> {
//     let mut result = vec![];
//     for noun in nouns {
//         let sql = format!("select value id from {noun} where REFNO.dbnum={dbnum} and OWNER.noun not in ['BRAN', 'HANG']");
//         let mut response = SUL_DB.query(&sql).await?;
//         let refnos: Vec<RefnoEnum> = response.take(0)?;
//         result.extend(refnos);
//     }
//     Ok(result)
// }

#[cached(result = true)]
pub async fn query_mdb_db_nums(module: DBType) -> anyhow::Result<Vec<u32>> {
    let db_type: u8 = module.into();
    let mdb = &get_db_option().mdb_name;
    let mdb = crate::helper::to_e3d_name(mdb);
    let mut response = SUL_DB
        .query(r#"
            let $dbnos = select value (select value DBNO from CURD.refno where STYP=$db_type) from only MDB where NAME=$mdb limit 1;
            select value dbnum from (select REFNO.dbnum as dbnum, array::find_index($dbnos, REFNO.dbnum) as o
                from WORL where REFNO.dbnum in $dbnos order by o);
        "#)
        .bind(("mdb", mdb))
        .bind(("db_type", db_type))
        .await?;
    let pe: Vec<u32> = response.take(1)?;
    Ok(pe)
}

///查询mdb的world下的所有pe
#[cached(result = true)]
pub async fn get_mdb_world_site_pes(
    mdb: String,
    module: DBType,
) -> anyhow::Result<Vec<SPdmsElement>> {
    let db_type: u8 = module.into();
    let mut response = SUL_DB
        .query(r#"
            let $dbnos = select value (select value DBNO from CURD.refno where STYP=$db_type) from only MDB where NAME=$mdb limit 1;
            let $a = (select value id from (select REFNO.id as id, array::find_index($dbnos, REFNO.dbnum) as o from WORL where REFNO.dbnum in $dbnos order by o));
            array::flatten(select value in.* from $a<-pe_owner)[?noun = 'SITE']
        "#)
        .bind(("mdb", mdb))
        .bind(("db_type", db_type))
        .await?;
    let pe: Vec<SPdmsElement> = response.take(2)?;
    Ok(pe)
}

/// Represents the response obtained from the database query.
#[cached(result = true)]
pub async fn get_world(mdb: String) -> anyhow::Result<Option<SPdmsElement>> {
    let sql = format!(
        " \
            let $f = (select value (select value DBNO from CURD.refno where STYP=1) from only MDB where NAME='{}' limit 1)[0]; \
            (select value REFNO.* from WORL where REFNO.dbnum=$f and REFNO.noun='WORL' limit 1)[0]",
        mdb
    );
    let mut response = SUL_DB.query(sql).await.unwrap();
    let pe: Option<SPdmsElement> = response.take(1)?;
    Ok(pe)
}

/// Represents the response obtained from the database query.
#[cached(result = true)]
pub async fn get_world_refno(mdb: String) -> anyhow::Result<RefnoEnum> {
    let mdb_name = if mdb.starts_with('/') {
        mdb.clone()
    } else {
        format!("/{}", mdb)
    };
    let sql = format!(
        " \
            let $f = (select value (select value DBNO from CURD.refno where STYP=1) from only MDB where NAME='{}' limit 1)[0]; \
            (select value REFNO from WORL where REFNO.dbnum=$f and REFNO.noun='WORL' limit 1)[0]",
        mdb_name
    );
    let mut response = SUL_DB.query(sql).await.unwrap();
    let id: Option<RefnoEnum> = response.take(1)?;
    Ok(id.unwrap_or_default())
}
