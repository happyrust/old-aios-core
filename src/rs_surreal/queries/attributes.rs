//! 属性查询模块
//! 
//! 提供元素属性相关的查询功能，包括基础属性、UDA 属性、UI 属性等。

use crate::rs_surreal::cache_manager::QUERY_CACHE;
use crate::rs_surreal::error_handler::{QueryError, QueryErrorHandler};
use crate::rs_surreal::query_builder::{PeQueryBuilder, QueryBuilder};
use crate::tool::db_tool::db1_dehash;
use crate::tool::math_tool::*;
use crate::types::*;
use crate::{NamedAttrMap, SurlValue, SUL_DB};
use anyhow::Result;
use cached::proc_macro::cached;
use indexmap::IndexMap;
use itertools::Itertools;
use std::time::Instant;

/// 属性查询服务
pub struct AttributeQueryService;

impl AttributeQueryService {
    /// 通过 SurrealQL 查询属性数据
    /// 
    /// # 参数
    /// * `refno` - 要查询的参考号
    /// 
    /// # 返回值
    /// * `Result<NamedAttrMap>` - 属性映射
    /// 
    /// # 错误
    /// * 如果查询失败会返回错误
    pub async fn get_named_attmap(refno: RefnoEnum) -> Result<NamedAttrMap> {
        let start_time = Instant::now();
        
        // 先检查缓存
        if let Some(cached_attrs) = QUERY_CACHE.get_attributes(&refno).await {
            return Ok(cached_attrs);
        }

        // 构建查询
        let query = PeQueryBuilder::new(refno).attributes_query();
        let sql = query.build().to_string();

        // 执行查询
        match query.fetch_value().await {
            Ok(value) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                QueryErrorHandler::log_query_execution(&sql, execution_time);

                let named_attmap: NamedAttrMap = value.into_inner().into();
                
                // 缓存结果
                QUERY_CACHE.set_attributes(refno, named_attmap.clone()).await;

                Ok(named_attmap)
            }
            Err(error) => {
                let query_error = QueryErrorHandler::handle_execution_error(&sql, error);
                Err(query_error.into())
            }
        }
    }

    /// 通过 SurrealQL 查询属性数据，包含 UDA 数据
    /// 
    /// # 参数
    /// * `refno_enum` - 要查询的参考号
    /// 
    /// # 返回值
    /// * `Result<NamedAttrMap>` - 包含 UDA 的属性映射
    /// 
    /// # 错误
    /// * 如果查询失败会返回错误
    pub async fn get_named_attmap_with_uda(refno_enum: RefnoEnum) -> Result<NamedAttrMap> {
        let start_time = Instant::now();
        let sql = format!(
            r#"
            --通过传递refno，查询属性值
            SELECT fn::default_full_name(REFNO) as NAME, * FROM ONLY {0}.refno FETCH pe;
            SELECT string::concat(':', if UDNA==none || string::len(UDNA)==0 {{ DYUDNA }} else {{ UDNA }}) as u, DFLT as v, UTYP as t FROM UDA WHERE !UHIDE AND {0}.noun IN ELEL;
            -- uda 单独做个查询？
            SELECT string::concat(':', if u.UDNA==none || string::len( u.UDNA)==0 {{ u.DYUDNA }} else {{ u.UDNA }}) as u, u.UTYP as t, v FROM (ATT_UDA:{1}).udas WHERE u.UTYP != none;
            "#,
            refno_enum.to_pe_key(),
            refno_enum.refno()
        );

        let query = QueryBuilder::from_sql(&sql);

        match query.execute().await {
            Ok(mut response) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                QueryErrorHandler::log_query_execution(&sql, execution_time);

                // 获得 UDA 的 map
                let o: surrealdb::types::Value = response.take(0)?;
                let mut named_attmap: NamedAttrMap = o.into_inner().into();

                // 处理 UDA 数据
                let o: surrealdb::types::Value = response.take(1)?;
                let array: Vec<SurlValue> = o.into_inner().try_into().unwrap();
                let uda_kvs: Vec<surrealdb::types::Object> =
                    array.into_iter().map(|x| x.try_into().unwrap()).collect();

                for map in uda_kvs {
                    let uname: String = map.get("u").unwrap().clone().try_into().unwrap();
                    let utype: String = map.get("t").unwrap().clone().try_into().unwrap();
                    if uname.as_str() == ":NONE" || uname.as_str() == ":unset" || uname.is_empty() {
                        continue;
                    }
                    let v = map.get("v").unwrap().clone();
                    let att_value = NamedAttrValue::from((utype.as_str(), v));
                    named_attmap.insert(uname, att_value);
                }

                // 处理覆盖数据
                let o: surrealdb::types::Value = response.take(2)?;
                let array: Vec<SurlValue> = o.into_inner().try_into().unwrap();
                let overwrite_kvs: Vec<surrealdb::types::Object> =
                    array.into_iter().map(|x| x.try_into().unwrap()).collect();

                for map in overwrite_kvs {
                    let uname: String = map.get("u").unwrap().clone().try_into().unwrap();
                    let utype: String = map.get("t").unwrap().clone().try_into().unwrap();
                    if uname.as_str() == ":NONE" || uname.as_str() == ":unset" || uname.is_empty() {
                        continue;
                    }
                    let v = map.get("v").unwrap().clone();
                    let att_value = NamedAttrValue::from((utype.as_str(), v));
                    named_attmap.insert(uname, att_value);
                }

                Ok(named_attmap)
            }
            Err(error) => {
                let query_error = QueryErrorHandler::handle_execution_error(&sql, error);
                Err(query_error.into())
            }
        }
    }

    /// 获取 UI 显示用的属性映射
    /// 
    /// # 参数
    /// * `refno_enum` - 要查询的参考号
    /// 
    /// # 返回值
    /// * `Result<NamedAttrMap>` - UI 显示用的属性映射
    /// 
    /// # 错误
    /// * 如果查询失败会返回错误
    pub async fn get_ui_named_attmap(refno_enum: RefnoEnum) -> Result<NamedAttrMap> {
        let mut attmap = Self::get_named_attmap_with_uda(refno_enum).await?;
        attmap.fill_explicit_default_values();
        
        let mut refno_fields: Vec<RefnoEnum> = vec![];
        let mut keys = vec![];
        let mut unset_keys = vec![];
        let mut new_desp = None;
        let mut tuples = vec![];
        let unip = attmap.get_i32_vec("UNIPAR").unwrap_or_default();

        for (k, v) in &mut attmap.map {
            if k == "REFNO" {
                if let NamedAttrValue::RefnoEnumType(r) = v {
                    *v = NamedAttrValue::RefU64Type(r.refno().into());
                }
                continue;
            }
            if k == "UNIPAR" || k == "SESNO" {
                continue;
            }
            match v {
                NamedAttrValue::RefU64Type(r) => {
                    if r.is_valid() {
                        refno_fields.push((*r).into());
                        keys.push(k.to_owned());
                    } else {
                        unset_keys.push(k.to_owned());
                    }
                }
                NamedAttrValue::RefnoEnumType(r) => {
                    if r.refno().is_valid() {
                        refno_fields.push(*r);
                        keys.push(k.to_owned());
                    } else {
                        unset_keys.push(k.to_owned());
                    }
                }
                NamedAttrValue::Vec3Type(d) => {
                    if k == "ORI" {
                        tuples.push((
                            k.clone(),
                            NamedAttrValue::StringType(dquat_to_pdms_ori_xyz_str(
                                &angles_to_dori(*d).unwrap_or_default(),
                                false,
                            )),
                        ));
                    } else if k.contains("POS") {
                        tuples.push((k.clone(), NamedAttrValue::StringType(vec3_to_xyz_str(*d))));
                    } else {
                        // 默认是方向
                        tuples.push((
                            k.clone(),
                            NamedAttrValue::StringType(convert_to_xyz(&to_pdms_dvec_str(
                                &d.as_dvec3(),
                                false,
                            ))),
                        ));
                    }
                }
                NamedAttrValue::F32VecType(d) => {
                    if k == "DESP" {
                        let mut vec = vec![];
                        for (v, n) in d.iter().zip(&unip) {
                            if *n == 623723 {
                                vec.push(db1_dehash(*v as u32));
                            } else {
                                vec.push(v.to_string());
                            }
                        }
                        new_desp = Some(vec);
                    }
                }
                NamedAttrValue::InvalidType => {
                    unset_keys.push(k.to_owned());
                }
                _ => {}
            }
        }

        if let Some(new_desp) = new_desp {
            attmap.insert("DESP".to_owned(), NamedAttrValue::StringArrayType(new_desp));
            attmap.remove("UNIPAR");
        }

        for (k, v) in tuples {
            attmap.insert(k, v);
        }

        // 查询全名
        use crate::rs_surreal::queries::basic::BasicQueryService;
        let names = if !refno_fields.is_empty() {
            crate::rs_surreal::queries::batch::BatchQueryService::query_full_names(&refno_fields).await.unwrap_or_default()
        } else {
            Vec::new()
        };

        for (k, v) in keys.into_iter().zip(names) {
            attmap.insert(
                k,
                NamedAttrValue::StringType(if v.is_empty() { "unset".to_owned() } else { v }),
            );
        }

        for k in unset_keys {
            attmap.insert(k, NamedAttrValue::StringType("unset".to_owned()));
        }

        attmap.remove("SESNO");
        Ok(attmap)
    }

    /// 查询祖先节点属性数据
    /// 
    /// # 参数
    /// * `refno` - 要查询的参考号
    /// 
    /// # 返回值
    /// * `Result<Vec<NamedAttrMap>>` - 祖先节点的属性数据列表
    /// 
    /// # 错误
    /// * 如果查询失败会返回错误
    pub async fn get_ancestor_attmaps(refno: RefnoEnum) -> Result<Vec<NamedAttrMap>> {
        let start_time = Instant::now();
        let sql = format!("RETURN fn::ancestor({}).refno.*", refno.to_pe_key());
        let query = QueryBuilder::from_sql(&sql);

        match query.fetch_value().await {
            Ok(value) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                QueryErrorHandler::log_query_execution(&sql, execution_time);

                let os: Vec<SurlValue> = value.into_inner().try_into().unwrap();
                let named_attmaps: Vec<NamedAttrMap> = os.into_iter().map(|x| x.into()).collect();
                
                QueryErrorHandler::log_query_results(&sql, named_attmaps.len());
                Ok(named_attmaps)
            }
            Err(error) => {
                let query_error = QueryErrorHandler::handle_execution_error(&sql, error);
                Err(query_error.into())
            }
        }
    }

    /// 获取子元素的属性映射列表
    /// 
    /// # 参数
    /// * `refno` - 父元素的参考号
    /// 
    /// # 返回值
    /// * `Result<Vec<NamedAttrMap>>` - 子元素的属性映射列表
    /// 
    /// # 错误
    /// * 如果查询失败会返回错误
    pub async fn get_children_named_attmaps(refno: RefnoEnum) -> Result<Vec<NamedAttrMap>> {
        let start_time = Instant::now();
        let sql = format!(
            r#"SELECT value in.refno.* FROM {}<-pe_owner WHERE in.id!=none AND !in.deleted"#,
            refno.to_pe_key()
        );
        
        let query = QueryBuilder::from_sql(&sql);

        match query.fetch_value().await {
            Ok(value) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                QueryErrorHandler::log_query_execution(&sql, execution_time);

                let os: Vec<SurlValue> = value.into_inner().try_into().unwrap();
                let named_attmaps: Vec<NamedAttrMap> = os.into_iter().map(|x| x.into()).collect();
                
                QueryErrorHandler::log_query_results(&sql, named_attmaps.len());
                Ok(named_attmaps)
            }
            Err(error) => {
                let query_error = QueryErrorHandler::handle_execution_error(&sql, error);
                Err(query_error.into())
            }
        }
    }

    /// 通过路径查询单个属性
    /// 
    /// # 参数
    /// * `refno` - 要查询的参考号
    /// * `paths` - 属性路径列表
    /// * `fields` - 要返回的字段列表
    /// 
    /// # 返回值
    /// * `Result<NamedAttrMap>` - 查询到的属性映射
    /// 
    /// # 错误
    /// * 如果查询失败会返回错误
    pub async fn query_single_by_paths(
        refno: RefnoEnum,
        paths: &[&str],
        fields: &[&str],
    ) -> Result<NamedAttrMap> {
        let start_time = Instant::now();
        let mut ps = vec![];
        for &path in paths {
            let p = path.replace("->", ".refno.");
            let str = if p.starts_with(".") {
                p[1..].to_owned()
            } else {
                p
            };
            ps.push(str);
        }
        
        let sql = format!(
            r#"(SELECT value refno.* FROM (SELECT value [{}] FROM ONLY {}) WHERE id != none)[0]"#,
            ps.join(","),
            refno.to_pe_key()
        );

        let query = QueryBuilder::from_sql(&sql);

        match query.fetch_value().await {
            Ok(value) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                QueryErrorHandler::log_query_execution(&sql, execution_time);

                let mut map: NamedAttrMap = value.into_inner().into();
                
                // 只保留 fields 里的数据
                if !fields.is_empty() {
                    map.retain(|k, _| fields.contains(&k.as_str()));
                }

                Ok(map)
            }
            Err(error) => {
                let query_error = QueryErrorHandler::handle_execution_error(&sql, error);
                Err(query_error.into())
            }
        }
    }
}

/// 缓存版本的属性查询函数（保持向后兼容）
#[cached(result = true)]
pub async fn get_named_attmap(refno: RefnoEnum) -> anyhow::Result<NamedAttrMap> {
    AttributeQueryService::get_named_attmap(refno).await
}

#[cached(result = true)]
pub async fn get_named_attmap_with_uda(refno_enum: RefnoEnum) -> anyhow::Result<NamedAttrMap> {
    AttributeQueryService::get_named_attmap_with_uda(refno_enum).await
}

pub async fn get_ui_named_attmap(refno_enum: RefnoEnum) -> anyhow::Result<NamedAttrMap> {
    AttributeQueryService::get_ui_named_attmap(refno_enum).await
}

pub async fn get_ancestor_attmaps(refno: RefnoEnum) -> anyhow::Result<Vec<NamedAttrMap>> {
    AttributeQueryService::get_ancestor_attmaps(refno).await
}

#[cached(result = true)]
pub async fn get_children_named_attmaps(refno: RefnoEnum) -> anyhow::Result<Vec<NamedAttrMap>> {
    AttributeQueryService::get_children_named_attmaps(refno).await
}

pub async fn query_single_by_paths(
    refno: RefnoEnum,
    paths: &[&str],
    fields: &[&str],
) -> anyhow::Result<NamedAttrMap> {
    AttributeQueryService::query_single_by_paths(refno, paths, fields).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_attribute_query_service() {
        let refno = RefnoEnum::from("12345_67890");
        
        // 测试属性查询的错误处理
        match AttributeQueryService::get_named_attmap(refno).await {
            Ok(_) => {
                // 查询成功
            }
            Err(_) => {
                // 预期的错误，因为没有实际的数据库连接
            }
        }
    }
}
