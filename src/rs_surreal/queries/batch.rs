//! 批量查询模块
//! 
//! 提供批量数据查询功能，优化多个元素的查询性能。

use crate::rs_surreal::error_handler::{QueryError, QueryErrorHandler};
use crate::rs_surreal::query_builder::{BatchQueryBuilder, QueryBuilder};
use crate::types::*;
use anyhow::Result;
use indexmap::IndexMap;
use itertools::Itertools;
use std::time::Instant;

/// 批量查询服务
pub struct BatchQueryService;

impl BatchQueryService {
    /// 批量查询全名
    /// 
    /// # 参数
    /// * `refnos` - 要查询的参考号列表
    /// 
    /// # 返回值
    /// * `Result<Vec<String>>` - 全名列表
    /// 
    /// # 错误
    /// * 如果查询失败会返回错误
    pub async fn query_full_names(refnos: &[RefnoEnum]) -> Result<Vec<String>> {
        if refnos.is_empty() {
            return Ok(Vec::new());
        }

        let start_time = Instant::now();
        let query = BatchQueryBuilder::new(refnos.to_vec()).full_names_query();
        let sql = query.build().to_string();

        match query.fetch_all::<String>().await {
            Ok(names) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                QueryErrorHandler::log_query_execution(&sql, execution_time);
                QueryErrorHandler::log_query_results(&sql, names.len());
                Ok(names)
            }
            Err(error) => {
                let query_error = QueryErrorHandler::handle_execution_error(&sql, error);
                Err(query_error.into())
            }
        }
    }

    /// 批量查询全名映射
    /// 
    /// # 参数
    /// * `refnos` - 要查询的参考号列表
    /// 
    /// # 返回值
    /// * `Result<IndexMap<RefnoEnum, String>>` - refno 到全名的映射
    /// 
    /// # 错误
    /// * 如果查询失败会返回错误
    pub async fn query_full_names_map(refnos: &[RefnoEnum]) -> Result<IndexMap<RefnoEnum, String>> {
        let names = Self::query_full_names(refnos).await?;
        let map = IndexMap::from_iter(refnos.iter().cloned().zip(names));
        Ok(map)
    }

    /// 查询子元素的全名映射
    /// 
    /// # 参数
    /// * `refno` - 父元素的参考号
    /// 
    /// # 返回值
    /// * `Result<IndexMap<RefnoEnum, String>>` - 子元素 refno 到全名的映射
    /// 
    /// # 错误
    /// * 如果查询失败会返回错误
    pub async fn query_children_full_names_map(
        refno: RefnoEnum,
    ) -> Result<IndexMap<RefnoEnum, String>> {
        let start_time = Instant::now();
        let sql = format!(
            "SELECT value [in, fn::default_full_name(in)] FROM {}<-pe_owner WHERE record::exists(in)",
            refno.to_pe_key()
        );
        
        let query = QueryBuilder::from_sql(&sql);

        match query.fetch_all::<(RefnoEnum, String)>().await {
            Ok(pairs) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                QueryErrorHandler::log_query_execution(&sql, execution_time);
                QueryErrorHandler::log_query_results(&sql, pairs.len());
                
                let map = IndexMap::from_iter(pairs);
                Ok(map)
            }
            Err(error) => {
                let query_error = QueryErrorHandler::handle_execution_error(&sql, error);
                Err(query_error.into())
            }
        }
    }

    /// 批量查询数据并将 refno->name 替换为名称
    /// 
    /// # 参数
    /// * `refno` - 要查询的参考号
    /// 
    /// # 返回值
    /// * `Result<IndexMap<RefnoEnum, String>>` - refno 到名称的映射
    /// 
    /// # 错误
    /// * 如果查询失败会返回错误
    pub async fn query_data_with_refno_to_name(
        refno: RefnoEnum,
    ) -> Result<IndexMap<RefnoEnum, String>> {
        let start_time = Instant::now();
        let sql = format!(
            "SELECT value [in, fn::default_full_name(in)] FROM {}<-pe_owner WHERE record::exists(in)",
            refno.to_pe_key()
        );
        
        let query = QueryBuilder::from_sql(&sql);

        match query.fetch_all::<(RefnoEnum, String)>().await {
            Ok(pairs) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                QueryErrorHandler::log_query_execution(&sql, execution_time);
                QueryErrorHandler::log_query_results(&sql, pairs.len());
                
                let map = IndexMap::from_iter(pairs);
                Ok(map)
            }
            Err(error) => {
                let query_error = QueryErrorHandler::handle_execution_error(&sql, error);
                Err(query_error.into())
            }
        }
    }

    /// 批量查询多个 refno 并将其转换为名称
    /// 
    /// # 参数
    /// * `refnos` - 要查询的参考号列表
    /// 
    /// # 返回值
    /// * `Result<IndexMap<RefnoEnum, String>>` - refno 到名称的映射
    /// 
    /// # 错误
    /// * 如果查询失败会返回错误
    pub async fn query_multiple_refnos_to_names(
        refnos: &[RefnoEnum],
    ) -> Result<IndexMap<RefnoEnum, String>> {
        Self::query_full_names_map(refnos).await
    }

    /// 批量查询多个 refno 并返回其名称列表
    /// 
    /// # 参数
    /// * `refnos` - 要查询的参考号列表
    /// 
    /// # 返回值
    /// * `Result<Vec<String>>` - 名称列表
    /// 
    /// # 错误
    /// * 如果查询失败会返回错误
    pub async fn query_refnos_to_names_list(refnos: &[RefnoEnum]) -> Result<Vec<String>> {
        Self::query_full_names(refnos).await
    }

    /// 批量获取所有子元素的 refno
    /// 
    /// # 参数
    /// * `refnos` - 父元素的参考号列表
    /// 
    /// # 返回值
    /// * `Result<Vec<RefnoEnum>>` - 所有子元素的 refno 列表
    /// 
    /// # 错误
    /// * 如果查询失败会返回错误
    pub async fn get_all_children_refnos(
        refnos: impl IntoIterator<Item = &RefnoEnum>,
    ) -> Result<Vec<RefnoEnum>> {
        let pe_keys = refnos.into_iter().map(|x| x.to_pe_key()).join(",");
        
        if pe_keys.is_empty() {
            return Ok(Vec::new());
        }

        let start_time = Instant::now();
        let sql = format!(
            "array::flatten(SELECT value in FROM [{}]<-pe_owner WHERE record::exists(in.id) AND !in.deleted)",
            pe_keys
        );
        
        let query = QueryBuilder::from_sql(&sql);

        match query.fetch_all::<RefnoEnum>().await {
            Ok(refnos) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                QueryErrorHandler::log_query_execution(&sql, execution_time);
                QueryErrorHandler::log_query_results(&sql, refnos.len());
                Ok(refnos)
            }
            Err(error) => {
                let query_error = QueryErrorHandler::handle_execution_error(&sql, error);
                Err(query_error.into())
            }
        }
    }

    /// 批量查询类型
    /// 
    /// # 参数
    /// * `refnos` - 要查询的参考号列表
    /// 
    /// # 返回值
    /// * `Result<Vec<Option<String>>>` - 类型名称列表
    /// 
    /// # 错误
    /// * 如果查询失败会返回错误
    pub async fn query_types(refnos: &[RefU64]) -> Result<Vec<Option<String>>> {
        if refnos.is_empty() {
            return Ok(Vec::new());
        }

        let start_time = Instant::now();
        let sql = format!(
            r#"SELECT value noun FROM [{}]"#,
            refnos.iter().map(|x| x.to_pe_key()).join(",")
        );
        
        let query = QueryBuilder::from_sql(&sql);

        match query.fetch_all::<Option<String>>().await {
            Ok(types) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                QueryErrorHandler::log_query_execution(&sql, execution_time);
                QueryErrorHandler::log_query_results(&sql, types.len());
                Ok(types)
            }
            Err(error) => {
                let query_error = QueryErrorHandler::handle_execution_error(&sql, error);
                Err(query_error.into())
            }
        }
    }

    /// 按类型过滤子元素
    /// 
    /// # 参数
    /// * `refno` - 父元素的参考号
    /// * `types` - 要过滤的类型列表
    /// 
    /// # 返回值
    /// * `Result<Vec<RefnoEnum>>` - 过滤后的子元素 refno 列表
    /// 
    /// # 错误
    /// * 如果查询失败会返回错误
    pub async fn query_filter_children(
        refno: RefnoEnum,
        types: &[&str],
    ) -> Result<Vec<RefnoEnum>> {
        let start_time = Instant::now();
        let sql = if types.is_empty() {
            format!(
                r#"SELECT value in FROM {}<-pe_owner WHERE record::exists(in.id) AND !in.deleted"#,
                refno.to_pe_key()
            )
        } else {
            let nouns_str = types
                .iter()
                .map(|s| format!("'{s}'"))
                .collect::<Vec<_>>()
                .join(",");
            format!(
                r#"SELECT value in FROM {}<-pe_owner WHERE in.noun IN [{}] AND record::exists(in.id) AND !in.deleted"#,
                refno.to_pe_key(),
                nouns_str
            )
        };

        let query = QueryBuilder::from_sql(&sql);

        match query.fetch_all::<RefnoEnum>().await {
            Ok(children) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                QueryErrorHandler::log_query_execution(&sql, execution_time);
                QueryErrorHandler::log_query_results(&sql, children.len());
                Ok(children)
            }
            Err(error) => {
                let query_error = QueryErrorHandler::handle_execution_error(&sql, error);
                Err(query_error.into())
            }
        }
    }

    /// 按类型过滤子元素属性
    /// 
    /// # 参数
    /// * `refno` - 父元素的参考号
    /// * `types` - 要过滤的类型列表
    /// 
    /// # 返回值
    /// * `Result<Vec<NamedAttrMap>>` - 过滤后的子元素属性列表
    /// 
    /// # 错误
    /// * 如果查询失败会返回错误
    pub async fn query_filter_children_atts(
        refno: RefnoEnum,
        types: &[&str],
    ) -> Result<Vec<NamedAttrMap>> {
        use crate::{NamedAttrMap, SurlValue};
        
        let start_time = Instant::now();
        let sql = if types.is_empty() {
            format!(
                r#"SELECT value in.refno.* FROM {}<-pe_owner WHERE record::exists(in.id) AND !in.deleted"#,
                refno.to_pe_key()
            )
        } else {
            let nouns_str = types
                .iter()
                .map(|s| format!("'{s}'"))
                .collect::<Vec<_>>()
                .join(",");
            format!(
                r#"SELECT value in.refno.* FROM {}<-pe_owner WHERE in.noun IN [{}] AND record::exists(in.id) AND !in.deleted"#,
                refno.to_pe_key(),
                nouns_str
            )
        };

        let query = QueryBuilder::from_sql(&sql);

        match query.fetch_value().await {
            Ok(value) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                QueryErrorHandler::log_query_execution(&sql, execution_time);

                let atts = match value {
                    surrealdb::types::Value::Array(values) => values.into_inner(),
                    value => vec![value],
                };
                let result: Vec<NamedAttrMap> = atts.into_iter().map(|x| x.into()).collect();
                
                QueryErrorHandler::log_query_results(&sql, result.len());
                Ok(result)
            }
            Err(error) => {
                let query_error = QueryErrorHandler::handle_execution_error(&sql, error);
                Err(query_error.into())
            }
        }
    }
}

/// 向后兼容的函数
pub async fn query_full_names(refnos: &[RefnoEnum]) -> anyhow::Result<Vec<String>> {
    BatchQueryService::query_full_names(refnos).await
}

pub async fn query_full_names_map(
    refnos: &[RefnoEnum],
) -> anyhow::Result<IndexMap<RefnoEnum, String>> {
    BatchQueryService::query_full_names_map(refnos).await
}

pub async fn query_children_full_names_map(
    refno: RefnoEnum,
) -> anyhow::Result<IndexMap<RefnoEnum, String>> {
    BatchQueryService::query_children_full_names_map(refno).await
}

pub async fn query_data_with_refno_to_name(
    refno: RefnoEnum,
) -> anyhow::Result<IndexMap<RefnoEnum, String>> {
    BatchQueryService::query_data_with_refno_to_name(refno).await
}

pub async fn query_multiple_refnos_to_names(
    refnos: &[RefnoEnum],
) -> anyhow::Result<IndexMap<RefnoEnum, String>> {
    BatchQueryService::query_multiple_refnos_to_names(refnos).await
}

pub async fn query_refnos_to_names_list(refnos: &[RefnoEnum]) -> anyhow::Result<Vec<String>> {
    BatchQueryService::query_refnos_to_names_list(refnos).await
}

pub async fn get_all_children_refnos(
    refnos: impl IntoIterator<Item = &RefnoEnum>,
) -> anyhow::Result<Vec<RefnoEnum>> {
    BatchQueryService::get_all_children_refnos(refnos).await
}

pub async fn query_types(refnos: &[RefU64]) -> anyhow::Result<Vec<Option<String>>> {
    BatchQueryService::query_types(refnos).await
}

pub async fn query_filter_children(
    refno: RefnoEnum,
    types: &[&str],
) -> anyhow::Result<Vec<RefnoEnum>> {
    BatchQueryService::query_filter_children(refno, types).await
}

pub async fn query_filter_children_atts(
    refno: RefnoEnum,
    types: &[&str],
) -> anyhow::Result<Vec<NamedAttrMap>> {
    BatchQueryService::query_filter_children_atts(refno, types).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_batch_query_service() {
        let refnos = vec![
            RefnoEnum::from("12345_67890"),
            RefnoEnum::from("12345_67891"),
        ];
        
        // 测试批量查询的错误处理
        match BatchQueryService::query_full_names(&refnos).await {
            Ok(_) => {
                // 查询成功
            }
            Err(_) => {
                // 预期的错误，因为没有实际的数据库连接
            }
        }
    }
}
