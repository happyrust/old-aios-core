use crate::aios_db_mgr::PdmsDataInterface;
use crate::aios_db_mgr::aios_mgr::AiosDBMgr;
use crate::options::DbOption;
use crate::pdms_types::UdaMajorType::S;
use crate::pdms_types::{PdmsElement, PdmsNodeTrait};
use crate::pe::SPdmsElement;
use crate::room::algorithm::{RoomInfo, query_all_room_name};
use crate::table_const::{PBS_OWNER, PBS_TABLE, PDMS_MAJOR};
use crate::tool::hash_tool::{hash_str, hash_two_str};
use crate::types::*;
use crate::{
    DBType, SUL_DB, get_db_option, get_mdb_world_site_pes, insert_into_table,
    insert_into_table_with_chunks, insert_pe_into_table_with_chunks, insert_relate_to_table,
    query_ele_filter_deep_children, query_filter_deep_children, rs_surreal,
};
use anyhow::anyhow;
use bevy_ecs::resource::Resource;
use calamine::{RangeDeserializerBuilder, Reader, Xlsx, open_workbook};
use dashmap::DashMap;
use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_with::DisplayFromStr;
use serde_with::serde_as;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::future::Future;
use std::str::FromStr;
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::thread;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use surrealdb::sql::Thing;
use tokio::task;
use tokio::task::JoinHandle;

#[derive(Resource, Serialize, Deserialize, PartialEq, Debug, Default, Clone)]
pub struct SiteData {
    pub refno: RefU64,
    pub name: String,
    pub is_selected: bool,
}

#[derive(Resource, Serialize, Deserialize, Debug, Default, Clone)]
pub struct SiteVec {
    pub data: Vec<SiteData>,
}

#[derive(Resource, Serialize, Deserialize, Debug, Default, Clone)]
pub struct SelectedSiteVec {
    pub data: Vec<SiteData>,
}

#[derive(Resource, Serialize, Deserialize, Debug, Default, Clone)]
pub struct FlagSiteVec {
    pub data: Vec<SiteData>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct PdmsMajor {
    pub site: RefU64,
    pub major: String,
    // site下zone对应的专业代码
    pub zone: HashMap<RefU64, String>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct PbsMajorValue {
    pub id: RefU64,
    pub noun: String,
    pub major: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PbsConfig {
    pub code: Option<String>,
    pub name: Option<String>,
    pub att_type: Option<String>,
    pub site_pdms_name: Option<String>,
    pub zone_code: Option<String>,
    pub zone_name: Option<String>,
    pub zone_att_type: Option<String>,
    pub zone_pdms_name: Option<String>,
}

impl PbsConfig {
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.code.is_some() && self.name.is_some() && self.att_type.is_some()
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub(crate) struct SscLevel {
    pub name: Option<String>,
    pub att_type: Option<String>,
    pub owner: Option<String>,
}

impl SscLevel {
    pub fn is_valid(&self) -> bool {
        if self.name.is_none() || self.att_type.is_none() {
            return false;
        }
        true
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum SaveDatabaseChannelMsg {
    // 插入数据  表名和数据
    InsertSql(String, String),
    // 插入数据  表名和数据
    InsertPbsElements(String, Vec<PbsElement>),
    // 插入relate数据
    InsertRelateSql(Vec<String>),
    // 结束
    Quit,
}

/// 设置site和zone所属的专业
pub async fn gen_pdms_major_table() -> anyhow::Result<()> {
    // 读取专业配置表
    let major_codes = get_room_level_from_excel_refactor().await?.name_code_map;
    // 找到所有的site和zone
    let mut site_children_map = HashMap::new();
    let db_option = get_db_option();
    let mdb = db_option.mdb_name();
    let sites = get_mdb_world_site_pes(mdb, DBType::DESI).await?;
    let mut site_name_map = HashMap::new();
    for site in sites {
        let Ok(children) = rs_surreal::get_children_pes(site.refno).await else {
            continue;
        };
        for child in children {
            if child.noun != "ZONE".to_string() {
                continue;
            };
            site_children_map
                .entry(site.refno)
                .or_insert_with(Vec::new)
                .push(child);
        }
        site_name_map.entry(site.refno).or_insert(site.name);
    }
    // 给site和zone赋上对应的code
    let mut result = Vec::new();
    for codes in major_codes.into_iter().rev() {
        // 给site赋值
        for site in site_children_map.keys() {
            let Some(site_name) = site_name_map.get(site) else {
                continue;
            };
            if site_name.contains(&codes.site_name) {
                // let mut zone_majors = HashMap::new();
                result.push(PbsMajorValue {
                    id: site.refno(),
                    noun: "SITE".to_string(),
                    major: codes.site_code.clone(),
                });
                // 给zone赋值
                for (major_name, major_code) in &codes.zone_map {
                    for zone in site_children_map.get(&site).unwrap() {
                        if zone.name.contains(major_name) {
                            // zone_majors.entry(zone.refno).or_insert(major_code.clone());
                            result.push(PbsMajorValue {
                                id: zone.refno(),
                                noun: zone.noun.clone(),
                                major: major_code.clone(),
                            })
                        }
                    }
                }
                // 方便测试查看使用
                // result.push(PdmsMajor {
                //     site: *site,
                //     major: codes.site_code.clone(),
                //     zone: zone_majors,
                // })
            }
        }
    }
    // 将分配好的专业代码保存到数据库中
    let json = serde_json::to_string(&result)?;
    insert_into_table(&SUL_DB, PDMS_MAJOR, &json).await?;
    Ok(())
}

/// 设置site和zone所属的专业
pub async fn set_pdms_major_code(aios_mgr: &AiosDBMgr) -> anyhow::Result<()> {
    // 读取专业配置表
    let major_codes = get_room_level_from_excel_refactor().await?.name_code_map;
    // 找到所有的site和zone
    let mut site_children_map = HashMap::new();
    let mdb = if aios_mgr.db_option.mdb_name.starts_with("/") {
        aios_mgr.db_option.mdb_name.clone()
    } else {
        format!("/{}", aios_mgr.db_option.mdb_name)
    };
    let sites = get_mdb_world_site_pes(mdb, DBType::DESI).await?;
    let mut site_name_map = HashMap::new();
    for site in sites {
        let Ok(children) = aios_mgr.get_children(site.refno()).await else {
            continue;
        };
        for child in children {
            if child.noun != "ZONE".to_string() {
                continue;
            };
            site_children_map
                .entry(site.refno())
                .or_insert_with(Vec::new)
                .push(child);
        }
        site_name_map.entry(site.refno()).or_insert(site.name);
    }
    // 给site和zone赋上对应的code
    let mut result = Vec::new();
    let mut set = HashSet::new();
    for codes in major_codes.into_iter().rev() {
        // 给site赋值
        for site in site_children_map.keys() {
            let Some(site_name) = site_name_map.get(site) else {
                continue;
            };
            if site_name.contains(&codes.site_name) {
                // let mut zone_majors = HashMap::new();
                result.push(PbsMajorValue {
                    id: *site,
                    noun: "SITE".to_string(),
                    major: codes.site_code.clone(),
                });
                set.insert(*site);
                // 给zone赋值
                for (major_name, major_code) in &codes.zone_map {
                    for zone in site_children_map.get(&site).unwrap() {
                        if zone.name.contains(major_name) {
                            // zone_majors.entry(zone.refno()).or_insert(major_code.clone());
                            result.push(PbsMajorValue {
                                id: zone.refno.refno(),
                                noun: zone.noun.clone(),
                                major: major_code.clone(),
                            })
                        }
                    }
                }
                // 方便测试查看使用
                // result.push(PdmsMajor {
                //     site: *site,
                //     major: codes.site_code.clone(),
                //     zone: zone_majors,
                // })
            }
        }
    }
    // 将分配好的专业代码保存到数据库中
    let json = serde_json::to_string(&result)?;
    insert_into_table(&SUL_DB, PDMS_MAJOR, &json).await?;
    Ok(())
}

/// ssc专业配置excel表 返回的对应数据
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SscMajorCodeConfig {
    /// key : site 的 name (中文名) value : site 下对应的zone 的 name
    pub level: Vec<(String, Vec<String>)>,
    /// 英文 code 对应的中文名
    pub name_map: DashMap<String, String>,
    /// pdms中 site 和 zone name 对应的专业代码
    pub name_code_map: Vec<PbsMajorCode>,
}

/// pdms site 和 zone name 对应的专业代码
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PbsMajorCode {
    /// pdms site 的 name
    pub site_name: String,
    /// 专业代码
    pub site_code: String,
    /// site 下 zone name 对应的 专业代码
    pub zone_map: HashMap<String, String>,
}

/// 读取 专业分类 excel表 ，返回需要的值
pub async fn get_room_level_from_excel_refactor() -> anyhow::Result<SscMajorCodeConfig> {
    let mut level: Vec<(String, Vec<String>)> = Vec::new();
    let mut name_map = DashMap::new();
    let mut pdms_zone_name_map = HashMap::new();
    let mut name_code_map = Vec::new();

    let mut workbook: Xlsx<_> = open_workbook("resource/专业分类.xlsx")?;
    dbg!("加载专业分类.xlsx 成功");
    let range = workbook.worksheet_range("Sheet2")?;
    dbg!("打开Sheet2成功");

    let mut iter = RangeDeserializerBuilder::new().from_range(&range)?;
    let mut b_first = true;
    let mut site_code = "".to_string();
    let mut site_chinese_name = "".to_string();
    let mut pdms_site_name = "".to_string();
    let mut zones = Vec::new();
    // let mut configs = Vec::new();
    let mut configs_sql = String::new();
    let mut index = 0;
    while let Some(result) = iter.next() {
        let config: PbsConfig = result?;
        configs_sql.push_str(&format!(
            "create pbs_config:{index} content {};",
            serde_json::to_string(&config)?
        ));
        // configs.push(config.clone());
        // site 的 name 、code 、att_type
        if config.code.is_some()
            && config.name.is_some()
            && config.att_type.is_some()
            && config.site_pdms_name.is_some()
        {
            let read_site_code = config.code.unwrap();
            let read_site_chinese_name = config.name.unwrap();
            let read_pdms_site_name = config.site_pdms_name.unwrap();
            // code != site_code 代表是下一个site的数据了 , b_first 防止第一个判断就是 != 会导致读取的数据错开，第一个site没值
            if read_pdms_site_name != pdms_site_name && !b_first {
                let code = PbsMajorCode {
                    site_name: pdms_site_name.clone(),
                    site_code: site_code.clone(),
                    zone_map: pdms_zone_name_map.clone(),
                };
                name_code_map.push(code);
                pdms_zone_name_map.clear();

                level.push((site_code, zones.clone()));
                zones.clear();
            }
            b_first = false;
            site_code = read_site_code.clone();
            site_chinese_name = read_site_chinese_name.clone();
            pdms_site_name = read_pdms_site_name.clone();
            // 存储专业编码对应的中文名称
            name_map.insert(read_site_code, site_chinese_name.clone());

            // 存放 site 下 zone 的专业代码
            if config.zone_name.is_some() && config.zone_code.is_some() {
                let read_zone_name = config.zone_name.unwrap();
                let read_zone_code = config.zone_code.unwrap();
                name_map.insert(read_zone_code.clone(), read_zone_name.clone());
                // 存放 pdms的site下 zone name 对应的 专业代码
                if config.zone_pdms_name.is_some() {
                    pdms_zone_name_map
                        .entry(config.zone_pdms_name.unwrap())
                        .or_insert(read_zone_code.clone());
                }
                zones.push(read_zone_code);
            }
        }

        index += 1;
    }

    SUL_DB.query(configs_sql).await?;

    Ok(SscMajorCodeConfig {
        level,
        name_map,
        name_code_map,
    })
}

#[derive(Debug)]
struct PBSRelate {
    pub in_id: Thing,
    pub out_id: Thing,
    pub order_num: u32,
}

impl PBSRelate {
    pub fn to_surreal_relate(self, table: &str) -> String {
        format!(
            "relate {}->{}:[{},{}]->{}",
            self.in_id.to_raw(),
            table,
            self.out_id.to_raw(),
            self.order_num,
            self.out_id.to_raw()
        )
    }
}

pub static PBS_ROOT_ID: Lazy<Thing> = Lazy::new(|| Thing::from(("pbs", "0")));
pub const PBS_STR: &'static str = "PBS";

/// 生成pbs固定节点
pub async fn set_pbs_fixed_node(mut handles: &mut Vec<JoinHandle<()>>) -> anyhow::Result<()> {
    let mut eles = Vec::new();
    let mut edge_results = Vec::new();

    let mut workbook: Xlsx<_> = open_workbook("resource/ssc_level.xlsx")?;
    let range = workbook.worksheet_range("Sheet1")?;

    let mut iter = RangeDeserializerBuilder::new().from_range(&range)?;
    let mut idx = 0;
    eles.push(PbsElement {
        id: PBS_ROOT_ID.clone(),
        name: "/*".to_string(),
        children_cnt: 1,
        ..Default::default()
    });
    while let Some(result) = iter.next() {
        let v: SscLevel = result?;
        if v.is_valid() {
            let name = v.name.unwrap();
            let name_hash = PbsElement::id(&name);
            let owner = if v.owner.is_some() {
                PbsElement::id(&v.owner.unwrap())
            } else {
                0
            };
            let cur: Thing = ("pbs".to_string(), name_hash.to_string()).into();
            let owner: Thing = ("pbs".to_string(), owner.to_string()).into();
            eles.push(PbsElement {
                id: cur.clone(),
                noun: v.att_type.clone(),
                name,
                owner: owner.clone(),
                refno: None,
                children_cnt: 0,
            });

            edge_results.push(
                PBSRelate {
                    in_id: cur.clone(),
                    out_id: owner.clone(),
                    order_num: idx,
                }
                .to_surreal_relate(&PBS_OWNER),
            );
            idx += 1;
        }
    }
    // 保存树节点
    // let task = tokio::task::spawn(async move {
    dbg!(&eles.len());
    if let Err(e) = insert_pe_into_table_with_chunks(&SUL_DB, &PBS_TABLE, eles).await {
        dbg!(&e.to_string());
    }
    if let Err(e) = insert_relate_to_table(&SUL_DB, edge_results).await {
        dbg!(&e.to_string());
    }
    // });
    // handles.push(task);
    Ok(())
}

struct PbsRoomNodeResult {
    pub rooms: HashMap<String, BTreeSet<String>>,
    pub sql: String,
    pub relate_sql: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PbsElement {
    pub id: Thing,
    pub owner: Thing,
    pub refno: Option<RefnoEnum>,
    pub name: String,
    pub noun: Option<String>,
    pub children_cnt: usize,
}

impl Default for PbsElement {
    fn default() -> Self {
        Self {
            id: Thing::from(("pbs", "0")),
            owner: Thing::from(("pbs", "-1")),
            refno: None,
            name: String::default(),
            noun: None,
            children_cnt: 0,
        }
    }
}

impl PbsElement {
    #[inline]
    pub fn id(name: &str) -> u64 {
        hash_str(name)
    }

    pub fn gen_sur_json(&self) -> String {
        let mut json_string = serde_json::to_string_pretty(&serde_json::json!({
            "id": self.id.id.to_raw(),
            "owner": self.owner,
            "refno": self.refno,
            "name": self.name,
            "noun": self.noun,
            "children_cnt": self.children_cnt,
        }))
        .unwrap();
        json_string
    }

    pub fn root_ele() -> Self {
        Self {
            id: PBS_ROOT_ID.clone(),
            name: "/*".to_string(),
            children_cnt: 1,
            ..Default::default()
        }
    }
}

impl PdmsNodeTrait for PbsElement {
    #[inline]
    fn get_id(&self) -> Option<&Thing> {
        Some(&self.id)
    }

    #[inline]
    fn get_name(&self) -> &str {
        self.name.as_str()
    }

    #[inline]
    fn get_children_count(&self) -> usize {
        self.children_cnt
    }
}

/// 生成房间节点
pub async fn set_pbs_room_node(
    mut handles: &mut Vec<JoinHandle<()>>,
) -> anyhow::Result<HashMap<String, BTreeSet<RoomInfo>>> {
    let mut result = Vec::new();
    let mut relate_result = Vec::new();
    let rooms = query_all_room_name().await?;
    let mut name_set = HashSet::new();
    name_set.insert("一号机组".to_string());
    let first_jizhu: Thing = ("pbs".to_string(), PbsElement::id("一号机组").to_string()).into();
    // 将项目中所有的房间，通过厂房 、 层位 、 房间号进行排列和存储
    for (factory_idx, (factory, room)) in rooms.clone().into_iter().enumerate() {
        let factory_hash = PbsElement::id(&factory).to_string();
        let factory_id: Thing = ("pbs".to_string(), factory_hash).into();
        // 存放厂房
        result.push(PbsElement {
            id: factory_id.clone(),
            owner: first_jizhu.clone(),
            name: factory.clone(),
            ..Default::default()
        });
        relate_result.push(
            PBSRelate {
                in_id: factory_id.clone(),
                out_id: first_jizhu.clone(),
                order_num: factory_idx as u32,
            }
            .to_surreal_relate(&PBS_OWNER),
        );
        // 存放厂房下 安装层位 和 安装分区 两个固定节点
        let install_level = PbsElement::id(&format!("{}安装层位", factory)).to_string(); //将厂房放在一起hash，否则不同厂房的这两个节点会重复
        let install_area = PbsElement::id(&format!("{}安装分区", factory)).to_string();
        let install_level_id: Thing = ("pbs".to_string(), install_level).into();
        let install_area_id: Thing = ("pbs".to_string(), install_area).into();
        result.push(PbsElement {
            id: install_level_id.clone(),
            owner: factory_id.clone(),
            name: "安装层位".to_string(),
            ..Default::default()
        });
        relate_result.push(
            PBSRelate {
                in_id: install_level_id.clone(),
                out_id: factory_id.clone(),
                order_num: 0,
            }
            .to_surreal_relate(&PBS_OWNER),
        );
        result.push(PbsElement {
            id: install_area_id.clone(),
            owner: factory_id.clone(),
            name: "安装分区".to_string(),
            ..Default::default()
        });
        relate_result.push(
            PBSRelate {
                in_id: install_area_id.clone(),
                out_id: factory_id.clone(),
                order_num: 1,
            }
            .to_surreal_relate(&PBS_OWNER),
        );
        // 存放层位以及房间信息
        let mut level_map = HashSet::new();
        for (idx, r) in room.into_iter().enumerate() {
            let level = r.name[1..2].to_string(); // 房间号第二位就是层位,之前已经做过长度的判断，所以直接切片
            let Ok(level_num) = level.parse::<u32>() else {
                continue;
            };
            // 将厂房和层位放在一起hash，单独的层位hash id会重复
            let level_hash = PbsElement::id(&format!("{}{}", factory, level)).to_string();
            let level_id: Thing = ("pbs".to_string(), level_hash.clone()).into();
            // 层位
            if !level_map.contains(&level) {
                result.push(PbsElement {
                    id: level_id.clone(),
                    owner: install_level_id.clone(),
                    name: format!("{}层", level),
                    ..Default::default()
                });
                relate_result.push(
                    PBSRelate {
                        in_id: level_id.clone(),
                        out_id: install_level_id.clone(),
                        order_num: level_num,
                    }
                    .to_surreal_relate(&PBS_OWNER),
                );
                level_map.insert(level);
            }
            // 房间
            let room_hash = PbsElement::id(&r.name).to_string();
            let room_id: Thing = ("pbs".to_string(), room_hash).into();
            result.push(PbsElement {
                id: room_id.clone(),
                owner: level_id.clone(),
                name: r.name,
                refno: Some(r.refno),
                noun: Some("FRMW".to_string()),
                ..Default::default()
            });
            relate_result.push(
                PBSRelate {
                    in_id: room_id.clone(),
                    out_id: level_id.clone(),
                    order_num: idx as u32,
                }
                .to_surreal_relate(&PBS_OWNER),
            );
        }
    }
    // 保存树节点
    // let task = tokio::task::spawn(async move {
    if let Err(e) = insert_pe_into_table_with_chunks(&SUL_DB, &PBS_TABLE, result).await {
        dbg!(&e.to_string());
    }
    if let Err(e) = insert_relate_to_table(&SUL_DB, relate_result).await {
        dbg!(&e.to_string());
    }
    // });
    // handles.push(task);
    Ok(rooms)
}

/// 保存房间下的专业
pub async fn set_pbs_room_major_node(
    rooms: &HashMap<String, BTreeSet<RoomInfo>>,
    mut handles: &mut Vec<JoinHandle<()>>,
) -> anyhow::Result<()> {
    let mut result = Vec::new();
    let mut relate_result = Vec::new();
    // 获取 pdms site 和 zone 对应的专业代码
    let pdms_level = get_room_level_from_excel_refactor().await?;
    let major_map = pdms_level.name_map;
    for (_, room) in rooms {
        for r in room {
            let r = r.name.clone();
            // site 下的专业
            for (site_idx, (site_name, zones)) in pdms_level.level.iter().enumerate() {
                let site_hash = PbsElement::id(&format!("{}{}", &r, site_name)).to_string();
                let site_id: Thing = ("pbs".to_string(), site_hash).into();
                let room_hash = PbsElement::id(&r).to_string();
                let room_id: Thing = ("pbs".to_string(), room_hash).into();
                let Some(site_major) = major_map.get(site_name) else {
                    continue;
                };
                result.push(PbsElement {
                    id: site_id.clone(),
                    owner: room_id.clone(),
                    name: site_major.value().to_string(),
                    ..Default::default()
                });
                relate_result.push(
                    PBSRelate {
                        in_id: site_id.clone(),
                        out_id: room_id.clone(),
                        order_num: site_idx as u32,
                    }
                    .to_surreal_relate(&PBS_OWNER),
                );
                // 专业下的子专业
                for (zone_idx, zone) in zones.iter().enumerate() {
                    let zone_hash = PbsElement::id(&format!("{}{}", r, zone)).to_string(); // 避免不同专业下的子专业重复
                    let zone_id: Thing = ("pbs".to_string(), zone_hash).into();
                    let Some(zone_major) = major_map.get(zone) else {
                        continue;
                    };
                    result.push(PbsElement {
                        id: zone_id.clone(),
                        owner: site_id.clone(),
                        name: zone_major.value().to_string(),
                        ..Default::default()
                    });
                    relate_result.push(
                        PBSRelate {
                            in_id: zone_id.clone(),
                            out_id: site_id.clone(),
                            order_num: zone_idx as u32,
                        }
                        .to_surreal_relate(&PBS_OWNER),
                    );
                }
            }
        }
    }
    // 保存树节点
    // let task = tokio::task::spawn(async move {
    if let Err(e) = insert_pe_into_table_with_chunks(&SUL_DB, &PBS_TABLE, result).await {
        dbg!(&e.to_string());
    }
    if let Err(e) = insert_relate_to_table(&SUL_DB, relate_result).await {
        dbg!(&e.to_string());
    }
    // });
    // handles.push(task);
    Ok(())
}

/// 获取所有赋过专业值的site
pub async fn query_all_site_with_major() -> anyhow::Result<Vec<PbsMajorValue>> {
    let mut response = SUL_DB
        .query("select * from pdms_major where noun == 'SITE';")
        .await?;
    let result: Vec<PbsMajorValue> = response.take(0)?;
    Ok(result)
}

/// 获取所有赋过专业值的zone
async fn query_all_zone_with_major() -> anyhow::Result<Vec<PbsMajorValue>> {
    let mut response = SUL_DB
        .query("select * from pdms_major where noun == 'ZONE';")
        .await?;
    let result: Vec<PbsMajorValue> = response.take(0)?;
    Ok(result)
}

/// 保存房间下节点所属的专业
pub async fn set_pbs_node(mut handles: &mut Vec<JoinHandle<()>>) -> anyhow::Result<()> {
    let zones = query_all_zone_with_major().await?;
    let len = zones.len();
    // 查找zone下所有需要进行pbs计算的节点
    for (idx, zone) in zones.iter().enumerate() {
        println!(
            "正在处理 zone: {} ,目前第 {} 个,总共 {} 个",
            zone.id, idx, len
        );
        // if zone.id != RefU64::from_str("24383/68481").unwrap() { continue; };
        // 找到所有需要处理的节点
        // let nodes = query_ele_filter_deep_children(zone.id, vec!["BRAN".to_string(),
        //                                                          "EQUI".to_string(), "STRU".to_string(), "REST".to_string()]).await?;
        let zone_refno: RefnoEnum = zone.id.into();
        match query_filter_deep_children(zone_refno, &["BRAN"]).await {
            Ok(bran_refnos) => {
                // 处理bran
                if let Err(e) = set_pbs_bran_node(&bran_refnos, &zone, &mut handles).await {
                    dbg!(&e.to_string());
                }
            }
            Err(e) => {
                dbg!(&e.to_string());
            }
        }
        // 处理equi
        match query_filter_deep_children(zone_refno, &["EQUI"]).await {
            Ok(equi_refnos) => {
                // 处理equi
                if let Err(e) = set_pbs_equi_node(&equi_refnos, &zone, &mut handles).await {
                    dbg!(&e.to_string());
                }
            }
            Err(e) => {
                dbg!(&e.to_string());
            }
        }
        // 处理支吊架
        match query_filter_deep_children(zone_refno, &["STRU"]).await {
            Ok(stru_refnos) => match query_filter_deep_children(zone_refno, &["REST"]).await {
                Ok(rest_refnos) => {
                    if let Err(e) =
                        set_pbs_supp_and_stru_node(&stru_refnos, &rest_refnos, &zone, &mut handles)
                            .await
                    {
                        dbg!(&e.to_string());
                    }
                }
                Err(e) => {
                    dbg!(&e.to_string());
                }
            },
            Err(e) => {
                dbg!(&e.to_string());
            }
        }
    }
    Ok(())
}

/// 保存房间下bran相关的节点
async fn set_pbs_bran_node(
    refnos: &[RefnoEnum],
    zone: &PbsMajorValue,
    mut handles: &mut Vec<JoinHandle<()>>,
) -> anyhow::Result<()> {
    let mut result = Vec::new();
    let mut relate_result = Vec::new();
    // 查找bran相关的pdms树的数据
    let pdms_nodes = query_pbs_room_nodes(refnos).await?;
    for (idx, node) in pdms_nodes.into_iter().enumerate() {
        // 没有房间号的就跳过
        if node.room_code.is_none() {
            continue;
        };
        let room_code = node.room_code.clone().unwrap();
        let owner = PbsElement::id(&format!("{}{}", room_code, zone.major)).to_string();
        let owner_id: Thing = ("pbs".to_string(), owner).into();
        result.push(PbsElement {
            id: node.id.refno().to_pbs_thing(),
            refno: Some(node.id.clone()),
            owner: owner_id.clone(),
            name: node.name.clone(),
            noun: Some(node.noun.clone()),
            ..Default::default()
        });
        relate_result.push(
            PBSRelate {
                in_id: node.id.refno().to_pbs_thing(),
                out_id: owner_id.clone(),
                order_num: idx as u32,
            }
            .to_surreal_relate(&PBS_OWNER),
        );
        // 存放children
        for (child_idx, child) in node.children.into_iter().enumerate() {
            relate_result.push(
                PBSRelate {
                    in_id: child.id.clone(),
                    out_id: child.owner.clone(),
                    order_num: child_idx as u32,
                }
                .to_surreal_relate(&PBS_OWNER),
            );
            result.push(child);
        }
    }

    // let task = tokio::task::spawn(async move {
    if let Err(e) = insert_pe_into_table_with_chunks(&SUL_DB, &PBS_TABLE, result).await {
        dbg!(&e.to_string());
    }
    if let Err(e) = insert_relate_to_table(&SUL_DB, relate_result).await {
        dbg!(&e.to_string());
    }
    // });
    // handles.push(task);
    Ok(())
}

// /// 保存房间下equi相关的节点
async fn set_pbs_equi_node(
    refnos: &[RefnoEnum],
    zone: &PbsMajorValue,
    mut handles: &mut Vec<JoinHandle<()>>,
) -> anyhow::Result<()> {
    let mut result = Vec::new();
    let mut relate_result = Vec::new();
    // 查找equi相关的pdms树的数据
    let pdms_nodes = query_pbs_room_nodes(refnos).await?;
    // 收集sube
    let mut subes = Vec::new();
    for node in &pdms_nodes {
        for child in &node.children {
            if child.noun.as_deref() != Some("SUBE") {
                continue;
            };
            if let Some(refno) = child.refno {
                subes.push(refno);
            }
        }
    }
    // 查询sube的children
    let sube_children = query_pbs_children_by_refnos(&subes).await?;
    // 将equi节点放到pbs中
    for (idx, node) in pdms_nodes.into_iter().enumerate() {
        // 没有房间号的就跳过
        if node.room_code.is_none() {
            continue;
        };
        let room_code = node.room_code.clone().unwrap();
        let owner = PbsElement::id(&format!("{}{}", room_code, zone.major)).to_string();
        let owner_id: Thing = ("pbs".to_string(), owner).into();
        let node_id = node.id.refno().to_pbs_thing();
        result.push(PbsElement {
            id: node_id.clone(),
            refno: Some(node.id.clone()),
            owner: owner_id.clone(),
            name: node.name.clone(),
            noun: Some(node.noun.clone()),
            ..Default::default()
        });
        relate_result.push(
            PBSRelate {
                in_id: node_id,
                out_id: owner_id.clone(),
                order_num: idx as u32,
            }
            .to_surreal_relate(&PBS_OWNER),
        );
        // 存放children
        for (child_idx, child) in node.children.into_iter().enumerate() {
            relate_result.push(
                PBSRelate {
                    in_id: child.id.clone(),
                    out_id: child.owner.clone(),
                    order_num: child_idx as u32,
                }
                .to_surreal_relate(&PBS_OWNER),
            );
            // 将sube的children放到pbs中的sube下
            if child.noun.as_deref() == Some("SUBE")
                && let Some(refno) = child.refno
            {
                let Some(sube_children) = sube_children.get(&refno) else {
                    continue;
                };
                for (sube_idx, sube) in sube_children.iter().enumerate() {
                    result.push(sube.clone());
                    relate_result.push(
                        PBSRelate {
                            in_id: sube.id.clone(),
                            out_id: sube.owner.clone(),
                            order_num: sube_idx as u32,
                        }
                        .to_surreal_relate(&PBS_OWNER),
                    );
                }
            }
            result.push(child);
        }
    }
    // let task = tokio::task::spawn(async move {
    if let Err(e) = insert_pe_into_table_with_chunks(&SUL_DB, &PBS_TABLE, result).await {
        dbg!(&e.to_string());
    }
    if let Err(e) = insert_relate_to_table(&SUL_DB, relate_result).await {
        dbg!(&e.to_string());
    }
    // });
    // handles.push(task);
    Ok(())
}

/// 保存房间下supp相关的节点
async fn set_pbs_supp_and_stru_node(
    stru_refnos: &[RefnoEnum],
    rest_refnos: &[RefnoEnum],
    zone: &PbsMajorValue,
    mut handles: &mut Vec<JoinHandle<()>>,
) -> anyhow::Result<()> {
    let mut result = Vec::new();
    let mut relate_result = Vec::new();
    // 这几个支架下面只有STRU，不需要找REST
    if ["HVACSU", "ELEMSU", "ELELSU"].contains(&zone.major.as_str()) {
        let pdms_nodes = query_pbs_room_nodes(stru_refnos).await?;
        // 收集 FRMW
        let mut frmws = Vec::new();
        for node in &pdms_nodes {
            for child in &node.children {
                if child.noun.as_deref() != Some("FRMW") {
                    continue;
                };
                if let Some(refno) = child.refno {
                    frmws.push(refno);
                }
            }
        }
        // 查询 FRMW 和 HANG的children
        let frmw_pbs_children_map = query_pbs_children_by_refnos(&frmws).await?;
        for (idx, node) in pdms_nodes.iter().enumerate() {
            if node.room_code.is_none() {
                continue;
            };
            let room_code = node.room_code.clone().unwrap();
            let owner = PbsElement::id(&format!("{}{}", room_code, zone.major)).to_string();
            let owner_id: Thing = ("pbs".to_string(), owner).into();
            let node_id = node.id.refno().to_pbs_thing();
            // 存放 STRU
            result.push(PbsElement {
                id: node.id.refno().to_pbs_thing(),
                refno: Some(node.id.clone()),
                owner: owner_id.clone(),
                name: node.name.clone(),
                noun: Some(node.noun.clone()),
                ..Default::default()
            });
            relate_result.push(
                PBSRelate {
                    in_id: node.id.refno().to_pbs_thing(),
                    out_id: owner_id.clone(),
                    order_num: idx as u32,
                }
                .to_surreal_relate(&PBS_OWNER),
            );
            // 存放children
            for (child_idx, child) in node.children.iter().enumerate() {
                result.push(child.clone());
                relate_result.push(
                    PBSRelate {
                        in_id: child.id.clone(),
                        out_id: node_id.clone(),
                        order_num: child_idx as u32,
                    }
                    .to_surreal_relate(&PBS_OWNER),
                );
                // 将FRMW的children放到pbs中的sube下
                if child.noun.as_deref() == Some("FRMW")
                    && let Some(refno) = child.refno
                {
                    let Some(children) = frmw_pbs_children_map.get(&refno) else {
                        continue;
                    };
                    for (supp_idx, frmw) in children.iter().enumerate() {
                        result.push(frmw.clone());
                        relate_result.push(
                            PBSRelate {
                                in_id: frmw.id.clone(),
                                out_id: frmw.owner.clone(),
                                order_num: supp_idx as u32,
                            }
                            .to_surreal_relate(&PBS_OWNER),
                        );
                    }
                }
            }
        }
    } else {
        let mut refnos = stru_refnos.to_vec();
        refnos.extend(rest_refnos);
        let pdms_nodes = query_pbs_room_nodes(&refnos).await?;
        // 支吊架 pbs结构需要在REST和STRU上面加一层房间号 + 流水号的固定节点
        let mut supp_owner_map = HashSet::new();
        // 收集 FRMW 和 HANG
        let mut hangs = Vec::new();
        for node in &pdms_nodes {
            for child in &node.children {
                if child.noun.as_deref() != Some("FRMW") && child.noun.as_deref() != Some("HANG") {
                    continue;
                };
                if let Some(refno) = child.refno {
                    hangs.push(refno);
                }
            }
        }
        // 查询 FRMW 和 HANG的children
        let hang_children = query_pbs_children_by_refnos(&hangs).await?;
        for (idx, supp) in pdms_nodes.iter().enumerate() {
            // if supp.id != RefU64::from_str("24383/69071").unwrap() {
            //     continue;
            // }
            if supp.room_code.is_none() {
                continue;
            };
            let room_code = supp.room_code.clone().unwrap();
            // let supp_fixed_name_split = supp.name[1..]
            //     .to_string()
            //     .split("/")
            //     .map(|x| x.to_string())
            //     .collect::<Vec<_>>();
            // let Some(supp_fixed_name) = supp_fixed_name_split.last() else {
            //     continue;
            // };

            let Some(supp_fixed_name) = find_supp_fix_room_code(&supp.name) else {
                continue;
            };
            let fixed_hash = PbsElement::id(&supp_fixed_name[0]);
            let node_id: Thing = ("pbs".to_string(), fixed_hash.to_string()).into();
            let owner = PbsElement::id(&format!("{}{}", room_code, zone.major)).to_string();
            let owner_id: Thing = ("pbs".to_string(), owner).into();
            // 存放固定节点
            if !supp_owner_map.contains(&fixed_hash) {
                result.push(PbsElement {
                    id: node_id.clone(),
                    owner: owner_id.clone(),
                    name: supp_fixed_name[0].to_string(),
                    ..Default::default()
                });
                relate_result.push(
                    PBSRelate {
                        in_id: node_id.clone(),
                        out_id: owner_id.clone(),
                        order_num: idx as u32,
                    }
                    .to_surreal_relate(&PBS_OWNER),
                );
                supp_owner_map.insert(fixed_hash);
            }
            // 存放 STRU/REST
            let cur_id = supp.id.refno().to_pbs_thing();
            result.push(PbsElement {
                id: cur_id.clone(),
                owner: node_id.clone(),
                name: supp.noun.clone(),
                noun: Some(supp.noun.clone()),
                refno: Some(supp.id),
                ..Default::default()
            });
            relate_result.push(
                PBSRelate {
                    in_id: cur_id.clone(),
                    out_id: node_id.clone(),
                    order_num: if supp.noun.as_str() == "STRU" { 0 } else { 1 },
                }
                .to_surreal_relate(&PBS_OWNER),
            );
            // 存放children
            for (child_idx, child) in supp.children.iter().enumerate() {
                result.push(child.clone());
                relate_result.push(
                    PBSRelate {
                        in_id: child.id.clone(),
                        out_id: child.owner.clone(),
                        order_num: child_idx as u32,
                    }
                    .to_surreal_relate(&PBS_OWNER),
                );
                // 将FRMW/HANG的children放到pbs中的sube下
                if child.noun.as_deref() == Some("FRMW") || child.noun.as_deref() == Some("HANG") {
                    if let Some(refno) = child.refno {
                        let Some(children) = hang_children.get(&refno) else {
                            continue;
                        };
                        for (supp_idx, supp) in children.iter().enumerate() {
                            result.push(supp.clone());
                            relate_result.push(
                                PBSRelate {
                                    in_id: supp.id.clone(),
                                    out_id: supp.owner.clone(),
                                    order_num: supp_idx as u32,
                                }
                                .to_surreal_relate(&PBS_OWNER),
                            );
                        }
                    }
                }
            }
        }
    }
    // let task = tokio::task::spawn(async move {
    if let Err(e) = insert_pe_into_table_with_chunks(&SUL_DB, &PBS_TABLE, result).await {
        dbg!(&e.to_string());
    }
    if let Err(e) = insert_relate_to_table(&SUL_DB, relate_result).await {
        dbg!(&e.to_string());
    }
    // });
    // handles.push(task);
    Ok(())
}

/// pbs下重新划分的pdms树节点，bran equi等
#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
struct PBSRoomNode {
    pub id: RefnoEnum,
    pub name: String,
    pub noun: String,
    pub room_code: Option<String>,
    pub children: Vec<PbsElement>,
}

/// 查找pbs需要的pdms的节点以及房间号
async fn query_pbs_room_nodes(refnos: &[RefnoEnum]) -> anyhow::Result<Vec<PBSRoomNode>> {
    if refnos.is_empty() {
        return Ok(vec![]);
    };
    let refnos = refnos.into_iter().map(|refno| refno.to_pe_key()).join(",");
    let sql = format!(
        r#"
        select type::thing('pbs',record::id(id)) as id,name,noun,fn::room_code(id)[0] as room_code,
            (select fn::default_name(id) as name, noun, refno, type::thing('pbs',record::id(id)) as id, type::thing('pbs',record::id(owner)) as owner,
            array::len(<-pe_owner) as children_cnt from (select value in from <-pe_owner)
            ) as children
            from [{}]
        "#,
        refnos
    );
    let mut response = SUL_DB.query(&sql).await?;
    match response.take::<Vec<PBSRoomNode>>(0) {
        Ok(r) => {
            return Ok(r);
        }
        Err(e) => {
            dbg!(sql);
            dbg!(&e);
        }
    }
    Ok(vec![])
}

/// 查询多个参考号的children
async fn query_pbs_children_by_refnos(
    refnos: &[RefnoEnum],
) -> anyhow::Result<HashMap<RefnoEnum, Vec<PbsElement>>> {
    if refnos.is_empty() {
        return Ok(HashMap::default());
    };
    let mut map = HashMap::new();
    let pes = refnos.into_iter().map(|refno| refno.to_pe_key()).join(",");
    let sql = format!(
        r#"select fn::default_name(id) as name, noun, refno, type::thing('pbs',record::id(id)) as id, type::thing('pbs',record::id(owner)) as owner, array::len(<-pe_owner) as children_cnt  from array::flatten(select value in from [{}]<-pe_owner)"#,
        pes
    );
    let mut response = SUL_DB.query(sql).await?;
    let result: Vec<PbsElement> = response.take(0)?;
    for r in result {
        map.entry(r.owner.clone().into())
            .or_insert_with(Vec::new)
            .push(r);
    }
    Ok(map)
}

/// 接受保存数据库请求并执行操作
pub async fn execute_save_pbs(rx: mpsc::Receiver<SaveDatabaseChannelMsg>) -> anyhow::Result<()> {
    // let mut handles = vec![];
    for msg in rx {
        // let task = tokio::task::spawn(async move {
        match msg {
            // 保存table数据
            SaveDatabaseChannelMsg::InsertSql(table_name, sql) => {
                if let Err(e) = insert_into_table(&SUL_DB, &table_name, &sql).await {
                    dbg!(&e.to_string());
                }
            }
            SaveDatabaseChannelMsg::InsertPbsElements(table, eles) => {
                let json = eles.iter().map(|x| x.gen_sur_json()).join(",");
                SUL_DB
                    .query(format!("insert ignore into {} [{}];", table, json))
                    .await
                    .unwrap();
            }
            // 保存 relate
            SaveDatabaseChannelMsg::InsertRelateSql(relate_sql) => {
                if let Err(e) = insert_relate_to_table(&SUL_DB, relate_sql).await {
                    dbg!(&e.to_string());
                }
            }
            SaveDatabaseChannelMsg::Quit => {
                continue;
            }
        }
        // });
        // handles.push(task);
    }
    // futures::future::join_all(handles).await;
    Ok(())
}

/// 找到支吊架名称中的房间号 + 流水号
pub fn find_supp_fix_room_code(name: &str) -> Option<[String; 2]> {
    let re = Regex::new(r"([A-Z]\d{3})\.(\d{3})").unwrap();
    match re.captures(name) {
        Some(mat) => {
            let part1 = mat.get(1).map_or("", |m| m.as_str()).to_string();
            let part2 = mat.get(2).map_or("", |m| m.as_str()).to_string();
            Some([part1, part2])
        }
        None => None,
    }
}

#[tokio::test]
async fn test_set_pbs_fixed_node() -> anyhow::Result<()> {
    let aios_mgr = AiosDBMgr::init_from_db_option().await?;
    set_pdms_major_code(&aios_mgr).await?;
    let mut handles = vec![];
    set_pbs_fixed_node(&mut handles).await?;
    let rooms = set_pbs_room_node(&mut handles).await?;
    set_pbs_room_major_node(&rooms, &mut handles).await?;
    set_pbs_node(&mut handles).await?;
    futures::future::join_all(handles).await;
    Ok(())
}

#[tokio::test]
async fn test_set_pdms_major_code() -> anyhow::Result<()> {
    let aios_mgr = AiosDBMgr::init_from_db_option().await?;

    Ok(())
}

#[tokio::test]
async fn test_set_pbs_room_node() -> anyhow::Result<()> {
    AiosDBMgr::init_from_db_option().await?;
    // set_pbs_node().await?;
    Ok(())
}

#[test]
fn test_find_supp_fix_room_code() {
    dbg!(find_supp_fix_room_code("/R710.226"));
}
