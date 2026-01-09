use crate::aios_db_mgr::PdmsDataInterface;
use crate::init_surreal;
use crate::options::DbOption;
use crate::pdms_types::{EleTreeNode, PdmsElement};
use crate::pe::SPdmsElement;
use crate::table_const::{GLOBAL_DATABASE, PUHUA_MATERIAL_DATABASE};
use crate::{
    AttrMap, NamedAttrMap, RefU64, SUL_DB, SurlValue, get_children_ele_nodes, get_db_option,
    get_named_attmap, get_named_attmap_with_uda, get_next_prev, get_pe, get_world,
    get_world_transform, init_second_unit_surreal,
};
use async_trait::async_trait;
use bevy_transform::components::Transform;
use config::{Config, File};
#[cfg(feature = "sql")]
use sqlx::pool::PoolOptions;
#[cfg(feature = "sql")]
use sqlx::{MySql, Pool};
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use surrealdb::opt::auth::Root;

pub async fn init_surreal_with_signin(db_option: &DbOption) -> anyhow::Result<()> {
    SUL_DB
        .connect(db_option.get_version_db_conn_str())
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
    Ok(())
}

pub struct AiosDBMgr {
    pub db_option: DbOption,
}

impl AiosDBMgr {
    pub async fn init_from_db_option() -> anyhow::Result<Self> {
        let s = Config::builder()
            .add_source(File::with_name("DbOption"))
            .build()
            .unwrap();
        let db_option: DbOption = s.try_deserialize().unwrap();
        println!("=== SurrealDB 连接信息 ===");
        println!("数据库地址: {}:{}", &db_option.v_ip, &db_option.v_port);
        println!("命名空间: {}", &db_option.surreal_ns);
        println!("数据库名: {}", &db_option.project_name);
        println!("========================");
        match init_surreal().await {
            Ok(_) => {}
            Err(e) => {
                dbg!(&e.to_string());
            }
        }
        // 连接二号机组
        match init_second_unit_surreal().await {
            Ok(_) => {
                println!("连接到副机组");
            }
            Err(e) => {
                println!("无法连接到副机组");
                dbg!(&e.to_string());
            }
        }
        Ok(Self { db_option })
    }

    pub async fn get_surreal_db(&self) -> anyhow::Result<Surreal<Any>> {
        init_surreal_with_signin(&self.db_option).await?;
        Ok(SUL_DB.clone())
    }

    /// 获取所属房间的顶标高和底标高
    pub async fn query_own_room_panel_elevations(
        &self,
        refno: RefU64,
    ) -> anyhow::Result<(f32, f32)> {
        let sql = format!(
            "return fn::room_top_height({});
                        return fn::room_height({});",
            refno.to_pe_key(),
            refno.to_pe_key()
        );
        let mut response = SUL_DB.query(sql).await?;
        let top: Vec<f32> = response.take(0).unwrap_or(vec![]);
        let bottom: Vec<f32> = response.take(1).unwrap_or(vec![]);
        let top = top.get(0).map_or(0.0, |x| *x);
        let bottom = bottom.get(0).map_or(0.0, |x| *x);
        Ok((top, bottom))
    }

    /// 获取指定节点附近最近的 own_filter_types
    pub async fn query_around_owner_within_radius(
        &self,
        refno: RefU64,
        is_aabb: bool,
        offset: Option<f32>,
        nearest: bool,
        own_filter_types: &[&str],
    ) -> anyhow::Result<Vec<RefU64>> {
        // todo
        Ok(vec![])
    }
}

#[async_trait]
impl PdmsDataInterface for AiosDBMgr {
    async fn get_world(&self, mdb_name: &str) -> anyhow::Result<Option<PdmsElement>> {
        let Some(world) = get_world(format!("/{}", mdb_name)).await? else {
            return Ok(None);
        };
        Ok(Some(PdmsElement::from(world)))
    }

    async fn get_pdms_element(&self, refno: RefU64) -> anyhow::Result<Option<PdmsElement>> {
        let Some(pe) = get_pe(refno.into()).await? else {
            return Ok(None);
        };
        Ok(Some(PdmsElement::from(pe)))
    }

    async fn get_attr(&self, refno: RefU64) -> anyhow::Result<NamedAttrMap> {
        get_named_attmap_with_uda(refno.into()).await
    }

    async fn get_children(&self, refno: RefU64) -> anyhow::Result<Vec<EleTreeNode>> {
        get_children_ele_nodes(refno.into()).await
    }

    async fn get_ipara_from_bran(&self, refno: RefU64) -> anyhow::Result<Vec<f32>> {
        let sql = format!("
        select value (select value CATR.refno.PARA from only
        owner.refno.ISPE<-pe_owner<-pe<-pe_owner<-pe[? refno.TYPE = 'SELE'
        and $parent.owner.refno.TEMP >=refno.ANSW and $parent.owner.refno.TEMP <= refno.MAXA ]<-pe_owner<-pe.refno.*
        where $parent.owner.refno.HBOR >=ANSW and $parent.owner.refno.HBOR <= MAXA limit 1) from pe:{};", refno.to_string());
        let mut response = SUL_DB.query(sql).await?;
        let result: Vec<Vec<f32>> = response.take(0).unwrap_or(vec![]);
        Ok(result.into_iter().flatten().collect())
    }

    async fn get_ele_from_name(&self, name: &str) -> anyhow::Result<Option<PdmsElement>> {
        let name = if name.starts_with("/") {
            name.to_string()
        } else {
            format!("/{}", name)
        };
        let sql = format!("select * from pe where name = '{}';", name);
        let mut response = SUL_DB.query(sql).await?;
        let pe: Option<SPdmsElement> = response.take(0)?;
        if pe.is_none() {
            return Ok(None);
        };
        let pe = pe.unwrap();
        Ok(Some(PdmsElement {
            refno: pe.refno(),
            owner: pe.owner.refno(),
            name: pe.name,
            noun: pe.noun,
            version: 0,
            children_count: 0,
        }))
    }

    async fn get_spre_attr(&self, refno: RefU64) -> anyhow::Result<Option<NamedAttrMap>> {
        let sql = format!("(select * from {}.refno.SPRE.refno)[0]", refno.to_pe_key());
        let mut response = SUL_DB.query(sql).await?;
        let o: surrealdb::Value = response.take(0)?;
        let named_attmap: NamedAttrMap = o.into_inner().into();
        if named_attmap.map.is_empty() {
            return Ok(None);
        };
        Ok(Some(named_attmap))
    }

    async fn get_catr_attr(&self, refno: RefU64) -> anyhow::Result<Option<NamedAttrMap>> {
        let sql = format!(
            "(select * from {}.refno.SPRE.refno.CATR.refno)[0]",
            refno.to_pe_key()
        );
        let mut response = SUL_DB.query(sql).await?;
        let o: surrealdb::Value = response.take(0)?;
        let named_attmap: NamedAttrMap = o.into_inner().into();
        if named_attmap.map.is_empty() {
            return Ok(None);
        };
        Ok(Some(named_attmap))
    }

    async fn get_foreign_attr(
        &self,
        refno: RefU64,
        foreign_type: &str,
    ) -> anyhow::Result<Option<NamedAttrMap>> {
        let sql = format!(
            "(select * from {}.refno.{}.refno)[0]",
            refno.to_pe_key(),
            foreign_type
        );
        let mut response = SUL_DB.query(sql).await?;
        let o: surrealdb::Value = response.take(0)?;
        let named_attmap: NamedAttrMap = o.into_inner().into();
        if named_attmap.map.is_empty() {
            return Ok(None);
        };
        Ok(Some(named_attmap))
    }

    async fn get_name(&self, refno: RefU64) -> anyhow::Result<String> {
        let sql = format!(
            "
            return fn::default_name({});
        ",
            refno.to_pe_key()
        );
        let mut response = SUL_DB.query(sql).await?;
        let o: Option<String> = response.take(0)?;
        Ok(o.unwrap_or("".to_string()))
    }

    async fn get_world_transform(&self, refno: RefU64) -> anyhow::Result<Option<Transform>> {
        let mut t = get_world_transform(refno.into()).await?;
        if let Some(ref mut transform) = t {
            transform.translation.x = (transform.translation.x * 100.0).round() / 100.0;
            transform.translation.y = (transform.translation.y * 100.0).round() / 100.0;
            transform.translation.z = (transform.translation.z * 100.0).round() / 100.0;

            transform.rotation.x = (transform.rotation.x * 100.0).round() / 100.0;
            transform.rotation.y = (transform.rotation.y * 100.0).round() / 100.0;
            transform.rotation.z = (transform.rotation.z * 100.0).round() / 100.0;
            transform.rotation.w = (transform.rotation.w * 100.0).round() / 100.0;
        }
        Ok(t)
    }

    async fn get_prev(&self, refno: RefU64) -> anyhow::Result<RefU64> {
        get_next_prev(refno.into(), false).await.map(|x| x.into())
    }

    async fn get_next(&self, refno: RefU64) -> anyhow::Result<RefU64> {
        get_next_prev(refno.into(), true).await.map(|x| x.into())
    }

    async fn get_room_code(&self, refno: RefU64) -> anyhow::Result<Option<String>> {
        let sql = format!("return fn::room_code({})[0];", refno.to_pe_key());
        let mut response = SUL_DB.query(&sql).await?;
        let r: Option<String> = response.take(0)?;
        match r {
            Some(room_code) => {
                if room_code.is_empty() {
                    dbg!("未查找到对应房间号");
                    dbg!(&sql);
                } else {
                    return Ok(Some(room_code));
                }
            }
            None => {
                dbg!("未查找到对应房间号");
                dbg!(&sql);
            }
        }
        Ok(None)
    }
}

impl AiosDBMgr {
    ///获得默认的连接字符串
    pub fn default_mysql_conn_str() -> String {
        let d = get_db_option();
        let user = d.user.as_str();
        let pwd = urlencoding::encode(&d.password);
        let ip = d.ip.as_str();
        let port = d.port.as_str();
        format!("mysql://{user}:{pwd}@{ip}:{port}")
    }

    pub fn puhua_conn_str(&self) -> String {
        let d = &self.db_option;
        let user = d.puhua_database_user.as_str();
        let pwd = d.puhua_database_password.as_str();
        let ip = d.puhua_database_ip.as_str();
        format!("mysql://{user}:{pwd}@{ip}")
    }

    #[cfg(feature = "sql")]
    /// 获取项目配置信息pool
    pub async fn get_global_pool() -> anyhow::Result<Pool<MySql>> {
        let connection_str = Self::default_mysql_conn_str();
        let url = &format!("{connection_str}/{}", GLOBAL_DATABASE);
        PoolOptions::new()
            .max_connections(500)
            .acquire_timeout(Duration::from_secs(10 * 60))
            .connect(url)
            .await
            .map_err({ |x| anyhow::anyhow!(x.to_string()) })
    }

    #[cfg(feature = "sql")]
    /// 获取项目pool
    pub async fn get_project_pool() -> anyhow::Result<Pool<MySql>> {
        let connection_str = Self::default_mysql_conn_str();
        let db_option = get_db_option();
        let url = &format!("{connection_str}/{}", db_option.project_name);
        PoolOptions::new()
            .max_connections(500)
            .acquire_timeout(Duration::from_secs(10 * 60))
            .connect(url)
            .await
            .map_err({ |x| anyhow::anyhow!(x.to_string()) })
    }

    #[cfg(feature = "sql")]
    /// 获取include_projects不同项目对应的pool
    pub async fn get_project_pools(&self) -> anyhow::Result<HashMap<String, Pool<MySql>>> {
        let connection_str = Self::default_mysql_conn_str();
        let mut map = HashMap::new();
        for project in &self.db_option.included_projects {
            let url = format!("{connection_str}/{}", project);
            let pool: Pool<MySql> = PoolOptions::new()
                .max_connections(500)
                .acquire_timeout(Duration::from_secs(10 * 60))
                .connect(&url)
                .await
                .map_err({ |x| anyhow::anyhow!(x.to_string()) })?;
            map.entry(project.to_string()).or_insert(pool);
        }
        Ok(map)
    }

    /// 获取外部的数据库
    #[cfg(feature = "sql")]
    pub async fn get_puhua_pool(&self) -> anyhow::Result<Pool<MySql>> {
        let conn = self.puhua_conn_str();
        let url = &format!("{conn}/{}", PUHUA_MATERIAL_DATABASE);
        PoolOptions::new()
            .max_connections(500)
            .acquire_timeout(Duration::from_secs(10 * 60))
            .connect(url)
            .await
            .map_err({ |x| anyhow::anyhow!(x.to_string()) })
    }
}
