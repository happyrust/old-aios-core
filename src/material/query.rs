use anyhow::anyhow;
use serde_json::Value;
#[cfg(feature = "sql")]
use sqlx::Executor;
#[cfg(feature = "sql")]
use sqlx::{MySql, Pool};
use std::collections::HashMap;

#[cfg(feature = "sql")]
/// 将材料表单数据保存到mysql中
pub async fn save_material_data_to_mysql(
    table_field: &[&str],
    table_name: &str,
    data_field: &[&str],
    data: Vec<HashMap<String, String>>,
    pool: Pool<MySql>,
) -> anyhow::Result<()> {
    match create_table_sql(&pool, &table_name, table_field).await {
        Ok(_) => {
            // 保存到数据库
            if !data.is_empty() {
                match save_material_value(&pool, &table_name, data_field, data).await {
                    Ok(_) => {}
                    Err(e) => {
                        dbg!(e.to_string());
                    }
                }
            }
        }
        Err(e) => {
            dbg!(&e.to_string());
        }
    }
    Ok(())
}

#[cfg(feature = "sql")]
/// 将两个不同结构的数据保存到mysql的同一张表中
pub async fn save_two_material_data_to_mysql(
    table_name: &str,
    chinese_name_map: &HashMap<&str, &str>,
    data_field_1: &[&str],
    data_1: Vec<HashMap<String, Value>>,
    data_field_2: &[&str],
    data_2: Vec<HashMap<String, Value>>,
    pool: &Pool<MySql>,
) -> anyhow::Result<()> {
    // 保存到数据库
    if !data_1.is_empty() {
        match save_material_value_test(&pool, &table_name, &data_field_1, chinese_name_map, data_1)
            .await
        {
            Ok(_) => {}
            Err(e) => {
                dbg!(e.to_string());
            }
        }
    }
    if !data_2.is_empty() {
        match save_material_value_test(&pool, &table_name, data_field_2, chinese_name_map, data_2)
            .await
        {
            Ok(_) => {}
            Err(e) => {
                dbg!(e.to_string());
            }
        }
    }
    Ok(())
}

#[cfg(feature = "sql")]
pub(crate) async fn create_table_sql(
    pool: &Pool<MySql>,
    table_name: &str,
    fields: &[&str],
) -> anyhow::Result<()> {
    // 生成创建表sql
    let mut create_table_sql = format!("CREATE TABLE IF NOT EXISTS {table_name} ( ");
    for &k in fields {
        if k == "参考号" {
            create_table_sql.push_str(format!("`{}` VARCHAR(50) NOT NULL ,", k).as_str())
        } else {
            create_table_sql.push_str(format!("`{}` VARCHAR(255),", k).as_str())
        }
    }
    create_table_sql.remove(create_table_sql.len() - 1);
    create_table_sql.push_str(")");
    // 创建表
    let mut conn = pool.acquire().await?;
    conn.execute(create_table_sql.clone().as_str()).await?;
    Ok(())
}

/// 保存材料表单数据
#[cfg(feature = "sql")]
pub(crate) async fn save_material_value(
    pool: &Pool<MySql>,
    table_name: &str,
    filed: &[&str],
    data: Vec<HashMap<String, String>>,
) -> anyhow::Result<()> {
    let mut sql = format!("INSERT IGNORE INTO `{}` (", table_name);
    for f in filed {
        sql.push_str(format!("`{}`,", f).as_str());
    }
    sql.remove(sql.len() - 1);
    // 放入值
    sql.push_str(") VALUES ");
    for d in data {
        sql.push_str("(");
        for f in filed {
            let value = d.get(*f).map_or("".to_string(), |x| x.to_string());
            sql.push_str(format!("'{}' ,", value).as_str());
        }
        sql.remove(sql.len() - 1);
        sql.push_str("),")
    }
    sql.remove(sql.len() - 1);
    let mut conn = pool.acquire().await?;
    match conn.execute(sql.clone().as_str()).await {
        Ok(_) => Ok(()),
        Err(_e) => Err(anyhow!(sql)),
    }
}

/// 保存材料表单数据
#[cfg(feature = "sql")]
pub(crate) async fn save_material_value_test(
    pool: &Pool<MySql>,
    table_name: &str,
    filed: &[&str],
    chinese_name_filed: &HashMap<&str, &str>,
    data: Vec<HashMap<String, Value>>,
) -> anyhow::Result<()> {
    let mut sql = format!("INSERT IGNORE INTO `{}` (", table_name);
    for f in filed {
        let Some(chinese) = chinese_name_filed.get(f) else {
            continue;
        };
        sql.push_str(format!("`{}`,", chinese).as_str());
    }
    sql.remove(sql.len() - 1);
    // 放入值
    sql.push_str(") VALUES ");
    for d in data {
        sql.push_str("(");
        for f in filed {
            let value = d
                .get(*f)
                .map_or("".to_string(), |x| x.to_string())
                .replace("\"", "");
            sql.push_str(format!("'{}',", value).as_str());
        }
        sql.remove(sql.len() - 1);
        sql.push_str("),")
    }
    sql.remove(sql.len() - 1);
    let mut conn = pool.acquire().await?;
    match conn.execute(sql.clone().as_str()).await {
        Ok(_) => Ok(()),
        Err(_e) => Err(anyhow!(sql)),
    }
}
