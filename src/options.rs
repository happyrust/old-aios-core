use std::path::{Path, PathBuf};

use crate::{RefU64, RefnoEnum};
use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Parser, Serialize, Deserialize)]
pub struct DbOption {
    /// 是否启用日志
    #[clap(long, default_value = "false")]
    pub enable_log: bool,
    /// 是否全量同步
    #[clap(long)]
    pub total_sync: bool,
    /// 是否启用索引
    #[clap(long)]
    pub enable_index: Option<bool>,
    /// 是否同步图数据库
    #[clap(long)]
    pub sync_graph_db: Option<bool>,
    /// 是否同步TiDB
    #[clap(long)]
    pub sync_tidb: Option<bool>,
    /// 是否同步版本化数据,默认为true
    #[clap(long, default_value = "true")]
    pub sync_versioned: Option<bool>,
    /// 是否同步实时数据
    #[clap(long)]
    pub sync_live: Option<bool>,
    /// 是否同步历史数据
    #[clap(long)]
    pub sync_history: Option<bool>,
    /// 是否增量同步
    #[clap(long)]
    pub incr_sync: bool,
    /// 是否只同步系统数据
    #[clap(long)]
    pub sync_only_sys: Option<bool>,
    // #[clap(long)]
    // pub replace_insert: Option<bool>,
    /// 同步的chunk size
    #[clap(long, default_value = "10_0000")]
    pub sync_chunk_size: Option<u32>,

    /// 是否使用tidb
    #[clap(long)]
    pub use_tidb: Option<bool>,

    /// 版本库的ip
    #[clap(long)]
    pub v_ip: String,
    /// 版本库的用户
    #[clap(long)]
    pub v_user: String,
    /// 版本库的密码
    #[clap(long)]
    pub v_password: String,
    /// 版本库的端口
    #[clap(long)]
    pub v_port: u16,
    // #[clap(long)]
    // pub kv_ip: String,
    // #[clap(long)]
    // pub kv_port: String,
    /// mqtt的host
    #[clap(long)]
    pub mqtt_host: String,
    /// mqtt的端口
    #[clap(long)]
    pub mqtt_port: u16,
    /// 需要同步的location
    #[clap(long)]
    pub location: String,
    /// 需要同步的location的db
    #[clap(long)]
    pub location_dbs: Option<Vec<u32>>,

    #[clap(long)]
    pub remote_file_server_hosts: Vec<String>,

    #[clap(long)]
    pub file_server_host: String,

    #[clap(long)]
    pub replace_dbs: bool,
    #[clap(skip)]
    pub replace_types: Option<Vec<String>>,
    #[clap(long)]
    pub gen_model: bool,
    pub build_cate_relate: Option<bool>,
    #[clap(long)]
    pub mesh_tol_ratio: Option<f32>,
    #[clap(long)]
    pub apply_boolean_operation: bool,
    #[clap(long)]
    pub save_model_mesh_to_graph_db: bool,
    #[clap(long)]
    pub gen_spatial_tree: bool,
    #[clap(long)]
    pub load_spatial_tree: bool,
    #[clap(long, default_value = "12.1SP4Projects")]
    pub project_path: String,
    pub included_projects: Vec<String>,
    //覆盖project的目录名
    pub project_dirs: Option<Vec<String>>,
    /// 包含的数据库文件列表
    #[clap(skip)]
    pub included_db_files: Option<Vec<String>>,
    /// MDB数据库名称
    #[clap(long)]
    pub mdb_name: String,
    /// 模块名称
    #[clap(long)]
    pub module: String,
    /// 项目名称
    #[clap(long)]
    pub project_name: String,
    /// 项目代码
    #[clap(long)]
    pub project_code: String,
    /// SurrealDB命名空间
    #[clap(skip)]
    pub surreal_ns: String,
    /// 手动指定的数据库编号列表
    #[clap(skip)]
    pub manual_db_nums: Option<Vec<u32>>,
    /// 需要排除的数据库编号列表
    #[clap(skip)]
    pub exclude_db_nums: Option<Vec<u32>>,
    /// 是否重置MDB项目
    #[clap(long)]
    pub reset_mdb_project: Option<bool>,

    /// 是否打印世界坐标系变换矩阵
    #[clap(long)]
    pub debug_print_world_transform: bool,
    /// 调试用的根节点引用号列表
    #[clap(skip)]
    pub debug_root_refnos: Option<Vec<String>>,
    /// 是否生成历史模型
    #[clap(long)]
    pub gen_history_model: Option<bool>,
    /// 测试用的引用号
    pub test_refno: Option<String>,
    /// 使用特定引用号生成模型
    pub gen_using_spref_refnos: Option<Vec<String>>,
    /// 手动同步的引用号列表
    #[clap(skip)]
    pub manual_sync_refnos: Option<Vec<String>>,
    /// 房间根节点引用号列表
    #[clap(skip)]
    pub room_root_refnos: Option<Vec<String>>,
    /// 调试用的引用号类型列表
    #[clap(skip)]
    pub debug_refno_types: Vec<String>,
    /// 是否替换网格
    #[clap(long)]
    pub replace_mesh: Option<bool>,
    /// 是否生成网格
    #[clap(long)]
    pub gen_mesh: bool,
    /// 是否生成材质
    #[clap(skip)]
    pub gen_material: Option<bool>,
    /// 是否保存到数据库
    #[clap(long)]
    pub save_db: Option<bool>,
    /// 是否需要同步基础引用号
    #[clap(long)]
    pub need_sync_refno_basic: bool,
    /// 是否仅更新数据库信息
    #[clap(long)]
    pub only_update_dbinfo: bool,
    /// 数据库IP地址
    #[clap(long)]
    pub ip: String,
    /// 数据库用户名
    #[clap(long)]
    pub user: String,
    /// 数据库密码
    #[clap(long)]
    pub password: String,
    /// 数据库端口号
    #[clap(long)]
    pub port: String,
    /// SQL线程数量
    #[clap(short)]
    pub sql_threads_number: u32,
    /// 是否重建SSC树
    #[clap(short)]
    pub rebuild_ssc_tree: bool,
    /// 批量插入SQL语句的数量
    #[clap(short)]
    pub batch_insert_sql_cnt: u32,
    /// PE块大小
    #[clap(short)]
    pub pe_chunk: u32,
    /// 属性块大小
    #[clap(short)]
    pub att_chunk: u32,
    /// 生成模型的批处理大小
    #[clap(short)]
    pub gen_model_batch_size: usize,
    /// ArangoDB数据库URL地址
    #[clap(long)]
    pub arangodb_url: String,
    /// 服务器发布IP地址
    #[clap(long)]
    pub server_release_ip: String,
    /// ArangoDB数据库用户名
    #[clap(long)]
    pub arangodb_user: String,
    /// ArangoDB数据库密码
    #[clap(long)]
    pub arangodb_password: String,
    /// ArangoDB数据库名称
    #[clap(long)]
    pub arangodb_database: String,
    /// 房间内的引用号列表
    #[clap(skip)]
    pub withing_room_refnos: Option<String>,
    /// 建筑数据库编号列表
    #[clap(skip)]
    pub arch_db_nums: Option<Vec<i32>>,
    /// 是否将空间树保存到数据库
    #[clap(long)]
    pub save_spatial_tree_to_db: bool,
    /// 是否启用多线程
    #[clap(long)]
    pub multi_threads: bool,
    /// 是否仅同步系统
    #[clap(short)]
    pub only_sync_sys: bool,
    /// 平台URL地址
    #[clap(long)]
    pub plat_url: String,
    /// 普华数据库IP地址
    #[clap(long)]
    pub puhua_database_ip: String,
    /// 普华数据库用户名
    #[clap(long)]
    pub puhua_database_user: String,
    /// 普华数据库密码
    #[clap(long)]
    pub puhua_database_password: String,

    /// 房间关键字（D10）：历史 toml 大量写成 `room_keyword`，与字段名对不上时 serde
    /// 会静默丢弃、退回默认值 `-RM`。alias 让两种键名都生效，老配置不用改。
    #[serde(alias = "room_keyword")]
    pub room_key_word: Option<Vec<String>>,

    pub meshes_path: Option<String>,
    // pub geom_live: Option<bool>,
}

impl DbOption {
    // #[inline]
    // pub fn is_geom_live(&self) -> bool {
    //     self.geom_live.unwrap_or(false)
    // }

    pub fn get_test_refno(&self) -> Option<RefnoEnum> {
        self.test_refno.as_ref().map(|x| x.as_str().into())
    }

    pub fn build_cate_relate(&self) -> bool {
        self.build_cate_relate.unwrap_or(false)
    }

    #[inline]
    pub fn is_replace_mesh(&self) -> bool {
        self.replace_mesh.unwrap_or(false)
    }

    #[inline]
    pub fn is_gen_mesh_or_model(&self) -> bool {
        self.gen_mesh || self.gen_model
    }

    #[inline]
    pub fn is_sync_history(&self) -> bool {
        self.sync_history.unwrap_or(false)
    }

    #[inline]
    pub fn mdb_name(&self) -> String {
        if self.mdb_name.starts_with("/") {
            self.mdb_name.clone()
        } else {
            format!("/{}", self.mdb_name)
        }
    }

    #[inline]
    pub fn get_room_key_word(&self) -> Vec<String> {
        self.room_key_word
            .clone()
            .unwrap_or(vec!["-RM".to_string()])
    }

    #[inline]
    pub fn get_project_path(&self, project: &str) -> Option<PathBuf> {
        let mut data_dir = Path::new(&self.project_path);
        if self.project_dirs.is_none() {
            Some(data_dir.join(project))
        } else {
            let index = self.included_projects.iter().position(|x| x == project)?;
            Some(data_dir.join(&self.project_dirs.as_ref().unwrap()[index]))
        }
    }

    pub fn get_meshes_path(&self) -> PathBuf {
        let pathbuf = self
            .meshes_path
            .as_ref()
            .map(|x| Path::new(x).to_path_buf())
            .unwrap_or("assets/meshes".into());
        if !pathbuf.exists() {
            std::fs::create_dir_all(&pathbuf).unwrap();
        }
        pathbuf
    }

    #[inline]
    pub fn get_project_dir_names(&self) -> &Vec<String> {
        self.project_dirs
            .as_ref()
            .unwrap_or(&self.included_projects)
    }

    #[inline]
    pub fn is_save_db(&self) -> bool {
        self.save_db.unwrap_or(true)
    }

    #[inline]
    pub fn is_gen_history_model(&self) -> bool {
        self.gen_history_model.unwrap_or(false)
    }

    #[inline]
    pub async fn get_all_debug_refnos(&self) -> Vec<RefnoEnum> {
        let mut refnos = self
            .debug_root_refnos
            .as_ref()
            .map(|x| x.iter().map(|x| x.as_str().into()).collect::<Vec<_>>())
            .unwrap_or_default();
        if self.is_gen_history_model() {
            let mut h_refnos = vec![];
            for r in refnos.clone() {
                h_refnos.extend(crate::query_history_pes(r).await.unwrap_or_default());
            }
            refnos.extend(h_refnos);
        }
        //还要补充使用了gen_using_spref_refnos的模型
        let mut debug_spref_refnos = self
            .gen_using_spref_refnos
            .as_ref()
            .map(|x| x.iter().map(|x| x.as_str().into()).collect::<Vec<_>>())
            .unwrap_or_default();
        let using_debug_spref_ele_refnos = if !debug_spref_refnos.is_empty() {
            let refnos = crate::query_ele_refnos_by_spres(&debug_spref_refnos)
                .await
                .unwrap();
            refnos
        } else {
            vec![]
        };
        refnos.extend(
            using_debug_spref_ele_refnos
                .into_iter()
                .map(|x| RefnoEnum::Refno(x)),
        );

        refnos
    }

    #[inline]
    pub fn get_manual_sync_refnos(&self) -> Vec<RefU64> {
        self.manual_sync_refnos
            .as_ref()
            .map(|x| x.iter().map(|x| x.as_str().into()).collect::<Vec<_>>())
            .unwrap_or_default()
    }

    #[inline]
    pub fn get_version_db_conn_str(&self) -> String {
        let ip = self.v_ip.as_str();
        let port = self.v_port;
        format!("ws://{ip}:{port}")
    }

    // #[inline]
    // pub fn get_kv_db_conn_str(&self) -> String {
    //     let ip = self.kv_ip.as_str();
    //     let port = self.kv_port.as_str();
    //     format!("ws://{ip}:{port}")
    // }

    #[inline]
    pub fn get_mysql_conn_str(&self) -> String {
        let user = self.user.as_str();
        let pwd = urlencoding::encode(self.password.as_str());
        let ip = self.ip.as_str();
        let port = self.port.as_str();
        format!("mysql://{user}:{pwd}@{ip}:{port}")
    }

    #[inline]
    pub fn get_mysql_project_db_conn_str(&self) -> String {
        let user = self.user.as_str();
        let pwd = urlencoding::encode(self.password.as_str());
        let ip = self.ip.as_str();
        let port = self.port.as_str();
        format!("mysql://{user}:{pwd}@{ip}:{port}/{}", &self.project_name)
    }

    // #[inline]
    // pub fn get_mysql_db_conn_str(&self, db: &str) -> String {
    //     let user = self.user.as_str();
    //     let pwd = urlencoding::encode(self.password.as_str());
    //     let ip = self.ip.as_str();
    //     let port = self.port.as_str();
    //     format!("mysql://{user}:{pwd}@{ip}:{port}/{}", db)
    // }
}

#[derive(Debug, Default, Clone, Parser, Serialize, Deserialize)]
pub struct SecondUnitDbOption {
    // 项目名称
    #[clap(long)]
    pub project_name: String,
    /// 项目代码
    #[clap(long)]
    pub project_code: String,
    /// SurrealDB命名空间
    #[clap(skip)]
    pub surreal_ns: String,
    /// 二号机组的ip
    #[clap(long)]
    pub v_ip: String,
    /// 二号机组的用户
    #[clap(long)]
    pub v_user: String,
    /// 二号机组的密码
    #[clap(long)]
    pub v_password: String,
    /// 二号机组的端口
    #[clap(long)]
    pub v_port: u16,
}

impl SecondUnitDbOption {
    #[inline]
    pub fn get_version_db_conn_str(&self) -> String {
        let ip = self.v_ip.as_str();
        let port = self.v_port;
        format!("ws://{ip}:{port}")
    }
}
