pub mod datacenter_query;
pub mod geom;
pub mod graph;
pub mod index;
pub mod mdb;
pub mod query;

// 新的重构模块
// pub mod query_builder;
// pub mod error_handler;
// pub mod cache_manager;
// pub mod queries;

pub mod cate;
pub mod resolve;
pub mod spatial;
mod table_const;
pub mod uda;

pub mod pbs;

pub mod inst;

pub mod point;

pub mod function;

pub mod version;

pub mod e3d_db;
pub mod topology;

pub mod operation;

/// ADR-017 暂存读路由（STAGE_DB 句柄与批次执行上下文）。
pub mod staging;

/// ADR-053 direct 读路由（provider trait 与生成期执行上下文）。
pub mod direct;

pub use cate::*;
pub use direct::*;
pub use e3d_db::*;
pub use geom::*;
pub use graph::*;
pub use index::*;
pub use inst::*;
pub use mdb::*;
pub use pbs::*;
pub use point::*;
pub use query::*;
pub use resolve::*;
pub use spatial::*;
pub use staging::*;
pub use topology::*;
pub use uda::*;

use once_cell::sync::Lazy;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use surrealdb::opt::auth::Root;

// pub type SurlValue = surrealdb::Value;
pub type SurlValue = surrealdb::sql::Value;
pub type SurlStrand = surrealdb::sql::Strand;
pub static SUL_DB: Lazy<Surreal<Any>> = Lazy::new(Surreal::init);
pub static SECOND_SUL_DB: Lazy<Surreal<Any>> = Lazy::new(Surreal::init);
pub static KV_DB: Lazy<Surreal<Any>> = Lazy::new(Surreal::init);

///连接surreal
pub async fn connect_surdb(
    conn_str: &str,
    ns: &str,
    db: &str,
    username: &str,
    password: &str,
) -> Result<(), surrealdb::Error> {
    // 创建配置
    let config = surrealdb::opt::Config::default()
        .ast_payload()  // 启用AST格式
        ; // 设置容量
    SUL_DB
        .connect((conn_str, config))
        .with_capacity(1000)
        .await?;
    SUL_DB.use_ns(ns).use_db(db).await?;
    SUL_DB.signin(Root { username, password }).await?;
    Ok(())
}

pub async fn connect_kvdb(
    conn_str: &str,
    ns: &str,
    db: &str,
    username: &str,
    password: &str,
) -> Result<(), surrealdb::Error> {
    SUL_DB.connect(conn_str).with_capacity(1000).await?;
    SUL_DB.use_ns(ns).use_db(db).await?;
    SUL_DB.signin(Root { username, password }).await?;
    Ok(())
}

pub fn convert_to_sql_str_array(nouns: &[&str]) -> String {
    let nouns_str = nouns
        .iter()
        .map(|s| format!("'{s}'"))
        .collect::<Vec<_>>()
        .join(",");
    nouns_str
}
