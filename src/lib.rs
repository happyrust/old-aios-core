#![feature(let_chains)]
#![feature(trivial_bounds)]
#![feature(result_flattening)]
#![allow(warnings)]

use crate::error::HandleError;
use config::{Config, File};
use dashmap::DashMap;
#[allow(unused_mut)]
use std::collections::BTreeMap;
use std::io::Read;

pub use types::db_info::PdmsDatabaseInfo;

extern crate bitflags;
extern crate core;
extern crate phf;
extern crate serde_json;
extern crate futures;

pub mod accel_tree;
pub mod aios_db_mgr;
pub mod aql_types;
pub mod axis_param;

pub mod basic;

pub mod parse;

pub mod bevy_types;
pub mod cache;
pub mod consts;
pub mod csg;
pub mod error;
pub mod geom_types;
pub mod table_const;

pub mod geometry;

pub mod helper;
pub mod parsed_data;
pub mod pdms_data;
pub mod pdms_types;
pub mod plot_struct;
pub mod prim_geo;
pub mod shape;
pub mod tiny_expr;
pub mod tool;
// 全自动出图所需的结构体
pub mod bin_data;
pub mod create_attas_structs;
pub mod data_center;
pub mod datacenter_options;
pub mod metadata;
pub mod metadata_manager;
pub mod negative_mesh_type;
pub mod options;
pub mod pdms_pluggin;
pub mod pdms_user;
pub mod penetration;
pub mod plat_user;
pub mod rvm_types;
pub mod ssc_setting;
pub mod three_dimensional_review;
pub mod vague_search;
pub mod version_control;
pub mod virtual_hole;

pub mod achiver;
pub mod enso_types;
pub mod plugging_material;
pub mod room_setting;
pub mod water_calculation;

pub mod noun_graph;

pub mod data_state;
pub mod threed_review;

pub mod transform;

#[cfg(not(feature = "web"))]
pub mod test;

#[cfg(feature = "sea-orm")]
pub mod orm;
pub mod rs_surreal;
pub mod schema;
pub mod types;

pub mod material;
pub mod math;
pub mod room;

pub mod file_helper;

pub mod petgraph;

pub mod db;

#[cfg(not(target_arch = "wasm32"))]
pub mod spatial;

pub mod dblist;

pub mod expression;

pub mod export_data;

pub use crate::types::*;
pub use rs_surreal::*;

pub type BHashMap<K, V> = BTreeMap<K, V>;

use crate::function::define_common_functions;
use crate::options::{DbOption, SecondUnitDbOption};
use once_cell_serde::sync::OnceCell;
use surrealdb::opt::auth::Root;

///获得db option
#[inline]
pub fn get_db_option() -> &'static DbOption {
    static INSTANCE: OnceCell<DbOption> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        use config::{Config, ConfigError, Environment, File};
        let s = Config::builder()
            .add_source(File::with_name("DbOption"))
            .build()
            .unwrap();
        s.try_deserialize::<DbOption>().unwrap()
    })
}

///获取默认的数据库属性元数据信息
pub fn get_default_pdms_db_info() -> &'static PdmsDatabaseInfo {
    static INSTANCE: OnceCell<PdmsDatabaseInfo> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        //会动态维护这个json，所以需要通过文件来加载
        //使用feature，来选择是否加载文件，还是使用include_str
        let mut string = String::new();
        #[cfg(feature = "load_file")]
        {
            let mut file = File::open("all_attr_info.json").unwrap();
            file.read_to_string(&mut string);
        }

        #[cfg(not(feature = "load_file"))]
        {
            string = include_str!("../all_attr_info.json").to_string();
        }

        let mut db_info = serde_json::from_str::<PdmsDatabaseInfo>(&string).unwrap();
        db_info.fill_named_map();
        // dbinfo.fix();
        // dbinfo.save(None);
        db_info
    })
}

pub fn get_uda_info() -> &'static (DashMap<u32, String>, DashMap<String, u32>) {
    static INSTANCE: OnceCell<(DashMap<u32, String>, DashMap<String, u32>)> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        let mut ukey_udna_map = DashMap::new();
        let mut udna_ukey_map = DashMap::new();
        use config::{Config, ConfigError, Environment, File};
        let Ok(s) = Config::builder()
            .add_source(File::with_name("DbOption"))
            .build()
        else {
            return (DashMap::new(), DashMap::new());
        };
        let db_option: DbOption = s.try_deserialize().unwrap();
        for project in db_option.included_projects {
            let path = format!("{}_uda.bin", project);
            if let Ok(mut file) = std::fs::File::open(path) {
                let mut data = Vec::new();
                let _ = file.read_to_end(&mut data);
                let map = bincode::deserialize::<DashMap<u32, String>>(&data).unwrap_or_default();
                for (k, v) in map {
                    ukey_udna_map.entry(k).or_insert(v.to_string());
                    udna_ukey_map.entry(v).or_insert(k);
                }
            }
        }
        (ukey_udna_map, udna_ukey_map)
    })
}

pub async fn init_test_surreal() -> Result<DbOption, HandleError> {
    let s = Config::builder()
        .add_source(File::with_name("DbOption"))
        .build()
        .map_err(|e| HandleError::SurrealError {
            msg: format!("Failed to load DbOption config: {}", e),
        })?;
    let db_option: DbOption = s.try_deserialize().map_err(|e| HandleError::SurrealError {
        msg: format!("Failed to deserialize DbOption: {}", e),
    })?;

    // 创建配置
    let config = surrealdb::opt::Config::default().ast_payload(); // 启用AST格式

    // Connect to database
    SUL_DB
        .connect((db_option.get_version_db_conn_str(), config))
        .with_capacity(1000)
        .await
        .map_err(|e| HandleError::SurrealError {
            msg: format!("Failed to connect to database: {}", e),
        })?;

    // Set namespace and database
    SUL_DB
        .use_ns(&db_option.surreal_ns)
        .use_db(&db_option.project_name)
        .await
        .map_err(|e| HandleError::SurrealError {
            msg: format!("Failed to set namespace and database: {}", e),
        })?;

    // Sign in
    SUL_DB
        .signin(Root {
            username: &db_option.v_user,
            password: &db_option.v_password,
        })
        .await
        .map_err(|e| HandleError::SurrealError {
            msg: format!("Failed to sign in: {}", e),
        })?;

    // Define common functions
    define_common_functions()
        .await
        .map_err(|e| HandleError::SurrealError {
            msg: format!("Failed to define common functions: {}", e),
        })?;

    Ok(db_option)
}

pub async fn init_surreal() -> anyhow::Result<()> {
    let s = Config::builder()
        .add_source(File::with_name("DbOption"))
        .build()
        .unwrap();
    let db_option: DbOption = s.try_deserialize()?;
    // 创建配置
    let config = surrealdb::opt::Config::default().ast_payload(); // 启用AST格式
    SUL_DB
        .connect((db_option.get_version_db_conn_str(), config))
        .with_capacity(1000)
        .await?;
    SUL_DB
        .use_ns(&db_option.surreal_ns)
        .use_db(&db_option.project_name)
        .await?;
    SUL_DB
        .signin(Root {
            username: &db_option.v_user,
            password: &db_option.v_password,
        })
        .await?;
    // Define common functions
    define_common_functions()
        .await
        .map_err(|e| HandleError::SurrealError {
            msg: format!("Failed to define common functions: {}", e),
        })?;
    Ok(())
}

/// 连接二号机组
pub async fn init_second_unit_surreal() -> anyhow::Result<()> {
    let s = Config::builder()
        .add_source(File::with_name("SecondUnitDbOption"))
        .build()?;
    let db_option: SecondUnitDbOption = s.try_deserialize()?;
    let config = surrealdb::opt::Config::default().ast_payload(); // 启用AST格式
    SECOND_SUL_DB
        .connect((db_option.get_version_db_conn_str(), config))
        .with_capacity(1000)
        .await?;
    SECOND_SUL_DB
        .use_ns(&db_option.surreal_ns)
        .use_db(&db_option.project_name)
        .await?;
    SECOND_SUL_DB
        .signin(Root {
            username: &db_option.v_user,
            password: &db_option.v_password,
        })
        .await?;
    Ok(())
}

/// 判断是否连接到二号机组
pub async fn b_connected_second_unit() -> anyhow::Result<()> {
    let s = Config::builder()
        .add_source(File::with_name("SecondUnitDbOption"))
        .build()?;
    let db_option: SecondUnitDbOption = s.try_deserialize()?;
    SECOND_SUL_DB
        .signin(Root {
            username: &db_option.v_user,
            password: &db_option.v_password,
        })
        .await?;
    Ok(())
}

/// 初始化测试数据库
pub async fn init_demo_test_surreal() -> Result<DbOption, HandleError> {
    let s = Config::builder()
        .add_source(File::with_name("DbOption"))
        .build()
        .map_err(|e| HandleError::SurrealError {
            msg: format!("Failed to load DbOption config: {}", e),
        })?;
    let db_option: DbOption = s.try_deserialize().map_err(|e| HandleError::SurrealError {
        msg: format!("Failed to deserialize DbOption: {}", e),
    })?;

    // 创建配置
    let config = surrealdb::opt::Config::default().ast_payload(); // 启用AST格式

    // Connect to database
    SUL_DB
        .connect((db_option.get_version_db_conn_str(), config))
        .with_capacity(1000)
        .await
        .map_err(|e| HandleError::SurrealError {
            msg: format!("Failed to connect to database: {}", e),
        })?;

    // Set namespace and database
    SUL_DB
        .use_ns(&db_option.surreal_ns)
        .use_db(&db_option.project_name)
        .await
        .map_err(|e| HandleError::SurrealError {
            msg: format!("Failed to set namespace and database: {}", e),
        })?;

    // Sign in
    SUL_DB
        .signin(Root {
            username: &db_option.v_user,
            password: &db_option.v_password,
        })
        .await
        .map_err(|e| HandleError::SurrealError {
            msg: format!("Failed to sign in: {}", e),
        })?;

    // Define common functions
    define_common_functions()
        .await
        .map_err(|e| HandleError::SurrealError {
            msg: format!("Failed to define common functions: {}", e),
        })?;

    Ok(db_option)
}
