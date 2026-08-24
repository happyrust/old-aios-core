use std::collections::HashMap;

use crate::RefU64;
use crate::aios_db_mgr::PdmsDataInterface;
use crate::aios_db_mgr::aios_mgr::AiosDBMgr;

/// 获取材料编码，通过分割spre或hstu
///
/// 命名规则为 第二个 / 到 :
pub async fn get_spre_material_code(
    refno: RefU64,
    foreign_name: &str,
    aios_mgr: &AiosDBMgr,
) -> Option<String> {
    let Ok(Some(spre_attr)) = aios_mgr.get_foreign_attr(refno, foreign_name).await else {
        return None;
    };
    let Some(spre_name) = spre_attr.get_name() else {
        return None;
    };
    let material_code = split_spre_material_code(&spre_name).unwrap_or("".to_string());
    Some(material_code)
}

/// 通过spre name 返回材料编码 命名规则为 第二个 / 到 :
///
/// 例如 "/VMB1/CPP00102:P,50" -> "CPP00102"
pub fn split_spre_material_code(spre_name: &str) -> Option<String> {
    if spre_name.contains(" OF ") {
        return None;
    }
    let spre_name_split = spre_name.split("/").collect::<Vec<_>>();
    if spre_name_split.len() < 3 {
        return None;
    }
    let spre_name_last = spre_name_split[2];
    let split = spre_name_last.split(":").collect::<Vec<_>>();
    if split.len() < 2 {
        return None;
    }
    Some(split[0].to_string())
}

/// 通过spre name提取规格，并从size_map中查找对应数据
///
/// 例如 "/VMB1/CPP00102:P,50" -> "50"
pub fn get_spre_size_from_map(
    spre_name: &str,
    size_map: &HashMap<String, String>,
) -> Option<String> {
    // 排除组合名称，和材料编码解析逻辑保持一致。
    if spre_name.contains(" OF ") {
        return None;
    }

    // 取第二个 / 后面的规格段，例如 CPP00102:P,50。
    let spre_name_split = spre_name.split("/").collect::<Vec<_>>();
    if spre_name_split.len() < 3 {
        return None;
    }
    let spre_name_last = spre_name_split[2];

    // 冒号后是参数段，例如 P,50。
    let split = spre_name_last.split(":").collect::<Vec<_>>();
    if split.len() < 2 {
        return None;
    }

    // 逗号后的值作为 size_map 的 key，例如 50。
    let size_split = split[1].split(",").collect::<Vec<_>>();
    if size_split.len() < 2 {
        return None;
    }
    let size = size_split[1].trim();
    size_map.get(size).cloned()
}
