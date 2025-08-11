use crate::aios_db_mgr::aios_mgr::AiosDBMgr;
use crate::material::dq::save_dq_material;
use crate::material::gps::save_gps_material_dzcl;
use crate::material::gy::{save_gy_material_dzcl, save_gy_material_equi, save_gy_material_valv};
use crate::material::nt::save_nt_material_dzcl;
use crate::material::sb::save_sb_material_dzcl;
use crate::material::tf::{define_tf_surreal_functions, save_tf_material_hvac};
use crate::material::tx::save_tx_material_equi;
use crate::material::yk::{save_yk_material_dzcl, save_yk_material_equi, save_yk_material_pipe};
use crate::pdms_user::RefnoMajor;
use crate::ssc_setting::{gen_pdms_major_table, query_all_site_with_major, set_pdms_major_code};
use crate::{query_filter_ancestors, RefU64, SUL_DB};
use std::collections::HashMap;
use std::io::Read;
use strum::IntoEnumIterator;
use surrealdb::engine::any::Any;
use surrealdb::Surreal;

pub mod dq;
pub mod gps;
pub mod gy;
pub mod nt;
pub(crate) mod query;
pub mod sb;
pub mod tf;
pub mod tx;
pub mod yk;

//使用enum，给每个选项一个名字
//使用 strum_macros::EnumString，实现strum::VariantNames

use strum::EnumIter;
use strum_macros::{AsRefStr, Display, EnumString};

#[derive(Debug, PartialEq, EnumString, Display, AsRefStr, EnumIter, Clone, Copy)]
pub enum MatMajorType {
    #[strum(to_string = "工艺专业-大宗材料清单")]
    GyDz,
    #[strum(to_string = "工艺专业-设备清单")]
    GyEquip,
    #[strum(to_string = "电气专业")]
    Dq,
    #[strum(to_string = "仪控专业")]
    Yk,
}

impl MatMajorType {
    pub fn major_names() -> Vec<String> {
        MatMajorType::iter().map(|x| x.to_string()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_major_names() {
        let names = MatMajorType::major_names();
        assert_eq!(
            names,
            vec![
                "工艺专业-大宗材料清单".to_string(),
                "工艺专业-设备清单".to_string(),
                "电气专业".to_string(),
                "仪控专业".to_string(),
            ]
        );
    }

    #[test]
    fn test_to_string() {
        assert_eq!(MatMajorType::GyDz.to_string(), "工艺专业-大宗材料清单");
        assert_eq!(MatMajorType::GyEquip.to_string(), "工艺专业-设备清单");
        assert_eq!(MatMajorType::Dq.to_string(), "电气专业");
        assert_eq!(MatMajorType::Yk.to_string(), "仪控专业");
    }

    #[test]
    fn test_from_str() {
        assert_eq!(
            "工艺专业-大宗材料清单".parse::<MatMajorType>(),
            Ok(MatMajorType::GyDz)
        );
        assert_eq!(
            "工艺专业-设备清单".parse::<MatMajorType>(),
            Ok(MatMajorType::GyEquip)
        );
        assert_eq!("电气专业".parse::<MatMajorType>(), Ok(MatMajorType::Dq));
        assert_eq!("仪控专业".parse::<MatMajorType>(), Ok(MatMajorType::Yk));
        assert!("Invalid".parse::<MatMajorType>().is_err());
    }
}

/// 保存所有的材料表单数据
pub async fn save_all_material_data() -> anyhow::Result<()> {
    // 生成专业代码
    // set_pdms_major_code(&aios_mgr).await?;
    gen_pdms_major_table().await?;
    // 提前跑已经创建surreal的方法
    if let Err(e) = define_material_surreal_funtions(SUL_DB.clone()).await {
        dbg!(e.to_string());
        return Ok(());
    }
    define_tf_surreal_functions(&SUL_DB).await;
    let mut handles = Vec::new();
    // 查找所有带专业的site
    let sites = query_all_site_with_major().await?;
    // 处理所有专业表单的数据
    for site in sites {
        dbg!(&site.id);
        let refno = site.id;
        // if site.major != "V".to_string() { continue; };
        match site.major.as_str() {
            // 工艺
            "T" => {
                // 大宗材料
                println!("工艺布置专业-大宗材料");
                handles.append(&mut save_gy_material_dzcl(refno).await);
                // 设备清单
                println!("工艺布置专业-设备清单");
                handles.append(&mut save_gy_material_equi(refno).await);
                // 阀门清单
                println!("工艺布置专业-阀门清单");
                handles.append(&mut save_gy_material_valv(refno).await);
            }
            // 仪控
            "I" => {
                // 大宗材料
                println!("仪控专业-大宗材料");
                handles.append(&mut save_yk_material_dzcl(refno).await);
                // 仪表管道
                println!("仪控专业-仪表管道");
                handles.append(&mut save_yk_material_pipe(refno).await);
                // 设备清单
                println!("仪控专业-设备清单");
                handles.append(&mut save_yk_material_equi(refno).await);
            }
            // 通风
            "V" => {
                // 风管管段
                println!("通风专业-风管管段");
                handles.append(&mut save_tf_material_hvac(refno).await);
            }
            // 电气
            "E" => {
                // 托盘及接地
                println!("电气专业-托盘及接地");
                handles.append(&mut save_dq_material(refno).await);
                // 通信
                // 通信系统
                println!("通信专业-通信系统");
                handles.append(&mut save_tx_material_equi(refno).await);
            }
            // 给排水
            "W" => {
                // 大宗材料
                println!("给排水专业-大宗材料");
                handles.append(&mut save_gps_material_dzcl(refno).await);
            }
            // 设备
            "EQUI" => {
                // 大宗材料
                println!("设备专业-大宗材料");
                handles.append(&mut save_sb_material_dzcl(refno).await);
            }
            // 暖通
            "N" => {
                // 阀门清单
                println!("暖通专业-阀门清单");
                handles.append(&mut save_nt_material_dzcl(refno).await);
            }
            _ => {}
        }
    }
    // 等待保存线程完成
    println!("查询完毕，等待数据库保存完成");
    futures::prelude::future::join_all(handles).await;
    Ok(())
}

/// 提前运行定义好的方法
pub async fn define_surreal_functions(db: Surreal<Any>) -> anyhow::Result<()> {
    // db.query(include_str!("../rs_surreal/tools/bolt.surql")).await?;
    // db.query(include_str!("../rs_surreal/tools/common.surql")).await?;
    // db.query(include_str!("../rs_surreal/tools/fln.surql")).await?;
    // db.query(include_str!("../rs_surreal/tools/formula.surql")).await?;
    // db.query(include_str!("../rs_surreal/tools/hvac.surql")).await?;
    // db.query(include_str!("../rs_surreal/tools/len.surql")).await?;
    // db.query(include_str!("../rs_surreal/tools/stif.surql")).await?;
    // db.query(include_str!("../rs_surreal/tools/washer.surql")).await?;
    Ok(())
}

pub async fn define_material_surreal_funtions(db: Surreal<Any>) -> anyhow::Result<()> {
    db.query(read_surreal_file("rs_surreal/material_list/common.surql")?.as_str()).await?;
    db.query(read_surreal_file("rs_surreal/material_list/gy/gy_bend.surql")?.as_str()).await?;
    db.query(read_surreal_file("rs_surreal/material_list/gy/gy_collect.surql")?.as_str()).await?;
    db.query(read_surreal_file("rs_surreal/material_list/gy/gy_common.surql")?.as_str()).await?;
    db.query(read_surreal_file("rs_surreal/material_list/gy/gy_equip.surql")?.as_str()).await?;
    db.query(read_surreal_file("rs_surreal/material_list/gy/gy_part.surql")?.as_str()).await?;
    db.query(read_surreal_file("rs_surreal/material_list/gy/gy_tubi.surql")?.as_str()).await?;
    db.query(read_surreal_file("rs_surreal/material_list/gy/gy_valve.surql")?.as_str()).await?;

    db.query(read_surreal_file("rs_surreal/material_list/dq/dq_bran.surql")?.as_str()).await?;
    db.query(read_surreal_file("rs_surreal/material_list/dq/dq_common.surql")?.as_str()).await?;
    db.query(read_surreal_file("rs_surreal/material_list/dq/dq_gensec.surql")?.as_str()).await?;
    db.query(read_surreal_file("rs_surreal/material_list/dq/dq_stru.surql")?.as_str()).await?;

    db.query(read_surreal_file("rs_surreal/material_list/eq/eq_common.surql")?.as_str()).await?;
    db.query(read_surreal_file("rs_surreal/material_list/eq/eq_dz.surql")?.as_str()).await?;

    db.query(read_surreal_file("rs_surreal/material_list/gps/gps_bend.surql")?.as_str()).await?;
    db.query(read_surreal_file("rs_surreal/material_list/gps/gps_elbo.surql")?.as_str()).await?;
    db.query(read_surreal_file("rs_surreal/material_list/gps/gps_flan.surql")?.as_str()).await?;
    db.query(read_surreal_file("rs_surreal/material_list/gps/gps_redu.surql")?.as_str()).await?;
    db.query(read_surreal_file("rs_surreal/material_list/gps/gps_tee.surql")?.as_str()).await?;
    db.query(read_surreal_file("rs_surreal/material_list/gps/gps_tubi.surql")?.as_str()).await?;

    db.query(read_surreal_file("rs_surreal/material_list/nt/nt_common.surql")?.as_str()).await?;
    db.query(read_surreal_file("rs_surreal/material_list/nt/nt_valve.surql")?.as_str()).await?;

    db.query(read_surreal_file("rs_surreal/material_list/tx/tx_sb.surql")?.as_str()).await?;

    db.query(read_surreal_file("rs_surreal/material_list/yk/yk_common.surql")?.as_str()).await?;
    db.query(read_surreal_file("rs_surreal/material_list/yk/yk_dzcl.surql")?.as_str()).await?;
    db.query(read_surreal_file("rs_surreal/material_list/yk/yk_equi.surql")?.as_str()).await?;
    db.query(read_surreal_file("rs_surreal/material_list/yk/yk_ybgd.surql")?.as_str()).await?;
    Ok(())
}

pub async fn define_core_material_surreal_funtions(db: Surreal<Any>) -> anyhow::Result<()> {
    db.query(read_surreal_file("src/rs_surreal/material_list/common.surql")?.as_str()).await?;
    db.query(read_surreal_file("src/rs_surreal/material_list/gy/gy_bend.surql")?.as_str()).await?;
    db.query(read_surreal_file("src/rs_surreal/material_list/gy/gy_collect.surql")?.as_str()).await?;
    db.query(read_surreal_file("src/rs_surreal/material_list/gy/gy_common.surql")?.as_str()).await?;
    db.query(read_surreal_file("src/rs_surreal/material_list/gy/gy_equip.surql")?.as_str()).await?;
    db.query(read_surreal_file("src/rs_surreal/material_list/gy/gy_part.surql")?.as_str()).await?;
    db.query(read_surreal_file("src/rs_surreal/material_list/gy/gy_tubi.surql")?.as_str()).await?;
    db.query(read_surreal_file("src/rs_surreal/material_list/gy/gy_valve.surql")?.as_str()).await?;

    db.query(read_surreal_file("src/rs_surreal/material_list/dq/dq_bran.surql")?.as_str()).await?;
    db.query(read_surreal_file("src/rs_surreal/material_list/dq/dq_common.surql")?.as_str()).await?;
    db.query(read_surreal_file("src/rs_surreal/material_list/dq/dq_gensec.surql")?.as_str()).await?;
    db.query(read_surreal_file("src/rs_surreal/material_list/dq/dq_stru.surql")?.as_str()).await?;

    db.query(read_surreal_file("src/rs_surreal/material_list/eq/eq_common.surql")?.as_str()).await?;
    db.query(read_surreal_file("src/rs_surreal/material_list/eq/eq_dz.surql")?.as_str()).await?;

    db.query(read_surreal_file("src/rs_surreal/material_list/gps/gps_bend.surql")?.as_str()).await?;
    db.query(read_surreal_file("src/rs_surreal/material_list/gps/gps_elbo.surql")?.as_str()).await?;
    db.query(read_surreal_file("src/rs_surreal/material_list/gps/gps_flan.surql")?.as_str()).await?;
    db.query(read_surreal_file("src/rs_surreal/material_list/gps/gps_redu.surql")?.as_str()).await?;
    db.query(read_surreal_file("src/rs_surreal/material_list/gps/gps_tee.surql")?.as_str()).await?;
    db.query(read_surreal_file("src/rs_surreal/material_list/gps/gps_tubi.surql")?.as_str()).await?;

    db.query(read_surreal_file("src/rs_surreal/material_list/nt/nt_common.surql")?.as_str()).await?;
    db.query(read_surreal_file("src/rs_surreal/material_list/nt/nt_valve.surql")?.as_str()).await?;

    db.query(read_surreal_file("src/rs_surreal/material_list/tx/tx_sb.surql")?.as_str()).await?;

    db.query(read_surreal_file("src/rs_surreal/material_list/yk/yk_common.surql")?.as_str()).await?;
    db.query(read_surreal_file("src/rs_surreal/material_list/yk/yk_dzcl.surql")?.as_str()).await?;
    db.query(read_surreal_file("src/rs_surreal/material_list/yk/yk_equi.surql")?.as_str()).await?;
    db.query(read_surreal_file("src/rs_surreal/material_list/yk/yk_ybgd.surql")?.as_str()).await?;
    Ok(())
}

fn read_surreal_file(file: &str) -> anyhow::Result<String> {
    let mut file = std::fs::File::open(file)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

/// 查询节点属于哪个专业和专业下的具体分类
pub async fn get_refnos_belong_major(
    refnos: &Vec<RefU64>,
) -> anyhow::Result<HashMap<RefU64, RefnoMajor>> {
    let mut result = HashMap::new();
    for refno in refnos {
        // 向上找到zone
        let zone = query_filter_ancestors((*refno).into(), &["ZONE"]).await?;
        if zone.is_empty() {
            continue;
        };
        let zone = zone[0];
        // 找zone和site对应的专业
        let sql = format!("select value fn::get_uda_value(id,'/CNPEdivco') from {};
        select value fn::get_uda_value(owner,'/CNPEspco') from {};", zone.to_pe_key(), zone.to_pe_key());
        let Ok(mut response) = SUL_DB.query(sql).await else {
            continue;
        };
        let zone_major: Vec<Option<String>> = response.take(0)?;
        let site_major: Vec<Option<String>> = response.take(1)?;
        if zone_major.is_empty() || site_major.is_empty() || zone_major[0].is_none() || site_major[0].is_none() {
            continue;
        };
        let site_major = site_major[0].clone().unwrap();
        let zone_major = zone_major[0].clone().unwrap();
        result.entry(*refno).or_insert(RefnoMajor {
            refno: refno.to_pdms_str(),
            major: site_major,
            major_classify: zone_major,
        });
    }
    Ok(result)
}
