use log::{LevelFilter, error};
use simplelog::{CombinedLogger, Config, WriteLogger};
use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum HandleError {
    #[error("Query Error SQL: {sql} ;\n  Error Message: {msg} ;\n  at {position}")]
    QueryErr {
        sql: String,
        msg: String,
        position: String,
    },
    #[error("Failed to connection {connection_name} ;\n connection_url is {connection_url}")]
    ConnectionErr {
        connection_name: String,
        connection_url: String,
    },
    #[error(
        "Failed to deserialize {struct_name} ;\n Error Message: {msg} ;\n SQL:{sql};  at {position}"
    )]
    DeserializeErr {
        struct_name: String,
        msg: String,
        sql: String,
        position: String,
    },
    // 查询为空异常
    #[error("Query Null: {0} , at {position}")]
    QueryNullErr { sql: String, position: String },
    #[error("Failed to save: {0} , at {position}")]
    SaveDatabaseErr { sql: String, position: String },
    #[error("Surreal Database Error: {msg}")]
    SurrealError { msg: String },
    #[error("Invalid Error: {0} \n , at {position}")]
    InvalidErr { msg: String, position: String },
}

impl HandleError {
    pub fn init_log(self) {
        error!("{}", format!("{:?}", self));
    }
}

/// 将query error注册到日志中
pub fn init_query_error<E: ToString>(sql: &str, error_msg: E, position: &str) {
    HandleError::QueryErr {
        sql: sql.to_string(),
        msg: error_msg.to_string(),
        position: position.to_string(),
    }
    .init_log();
}

/// 将 deserialize error注册到日志中
pub fn init_deserialize_error<E: ToString>(
    struct_name: &str,
    error_msg: E,
    sql: &str,
    position: &str,
) {
    HandleError::DeserializeErr {
        struct_name: struct_name.to_string(),
        msg: error_msg.to_string(),
        sql: sql.to_string(),
        position: position.to_string(),
    }
    .init_log();
}

/// 将查询为空注册到日志中
pub fn init_query_null_error(sql: &str, position: &str) {
    HandleError::QueryNullErr {
        sql: sql.to_string(),
        position: position.to_string(),
    }
    .init_log()
}

/// 将保存到数据库中错误注册到日志中
pub fn init_save_database_error(sql: &str, position: &str) {
    HandleError::SaveDatabaseErr {
        sql: sql.to_string(),
        position: position.to_string(),
    }
    .init_log()
}

fn create_error() -> Result<(), HandleError> {
    let e = HandleError::QueryErr {
        sql: "hello".to_string(),
        msg: "test".to_string(),
        position: std::panic::Location::caller().to_string(),
    };
    e.clone().init_log();
    Err(e)
}

#[test]
fn test_err() -> anyhow::Result<()> {
    // 配置日志文件
    let log_file = File::create("error.log")?;

    // 初始化日志系统
    CombinedLogger::init(vec![WriteLogger::new(
        LevelFilter::Error,
        Config::default(),
        log_file,
    )])
    .unwrap();

    // create_error()?;
    let json = "[]";
    let r = serde_json::from_str::<HashMap<String, String>>(json).unwrap();
    Ok(())
}
