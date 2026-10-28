use crate::parsed_data::CateAxisParam;
use crate::types::*;
use glam::Vec3;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::BTreeMap;
//
//

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct InstPointMap {
    pub refno: RefnoEnum,
    pub att_type: String,
    #[serde(default)]
    pub ptset_map: BTreeMap<String, CateAxisParam>,
}

/// 库里 `ptset` 对象的一个点。
///
/// 只有一种形状：gen-model 在发布事务里按 `pe` 同键写的分表 `pe_ptset.ptset["<P 号>"]`
/// （gen-model `docs/plans/2026-10-26-ptset-split-table-plan.md` §3）——
/// `{ pt, dir, bore, noun }`，点号只在键上，`bore` 求不出为 none。
///
/// 旧的 `inst_info.ptset` / `inst_relate.out.ptset` 形状已随 legacy 兜底一起退役，
/// 这里不再认 `number` / `pbore` 这些只有老行才有的字段：老库靠
/// [`crate::model_storage::check_dbnum`] 在更外层就被拦下，不会走到反序列化。
///
/// 消费方（rs-server 的 dq_bran / query / heat_dissipation）按 `.pt` / `.pbore` 取值，
/// 由 [`Self::into_cate_axis_param`] 归一成 [`CateAxisParam`]，调用代码不感知。
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PtsetPointRow {
    #[serde(default)]
    pub noun: Option<String>,
    #[serde(default)]
    pub pt: Vec3,
    #[serde(default)]
    pub dir: Option<Vec3>,
    /// 通径。`None` = 求不出，落成 `pbore = 0.0`（旧语义「0 = 未设」）。
    #[serde(default)]
    pub bore: Option<f32>,
}

impl PtsetPointRow {
    /// `key` 是 `ptset` 对象上的点号（字符串，`"1"` / `"2"` …）。
    pub fn into_cate_axis_param(self, key: &str) -> CateAxisParam {
        let defaults = CateAxisParam::default();
        CateAxisParam {
            refno: defaults.refno,
            number: key.trim().parse().unwrap_or(defaults.number),
            pt: self.pt,
            dir: self.dir,
            dir_flag: defaults.dir_flag,
            ref_dir: None,
            pbore: self.bore.unwrap_or(0.0),
            pwidth: 0.0,
            pheight: 0.0,
            pconnect: String::new(),
        }
    }
}

/// `select … as ptset_row / dbnum_storage_version` 直接落下来的原始行。
///
/// 版本核对在 [`TryFrom`] 里做，所以任何拿到 [`InstPointMap`] 的调用方都可以确信
/// 这份点集是当前存储架构版本写的。
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct InstPointMapRow {
    pub refno: RefnoEnum,
    pub att_type: String,
    /// `pe.dbnum`——库里的权威库号（`resource/surreal/common.surql:12` 的 `pe_dbnum_noun` 索引）。
    ///
    /// 不从 `refno` 解析前缀：record id 的形制会随版本变，库号字段不会。
    #[serde(default)]
    pub dbnum: u32,
    /// `dbnum_watermark:{dbnum}.model_storage_version`；`None` = 没写过这一列 = 老库。
    #[serde(default)]
    pub dbnum_storage_version: Option<u32>,
    /// `pe_ptset:{pe}` 那一行；`None` = 这根还没按新版发布过。
    #[serde(default)]
    pub ptset_row: Option<PePtsetRow>,
}

/// `pe_ptset` 分表的一行（只取读侧要用的两格）。
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PePtsetRow {
    #[serde(default)]
    pub storage_version: Option<u32>,
    #[serde(default)]
    pub ptset: BTreeMap<String, PtsetPointRow>,
}

impl TryFrom<InstPointMapRow> for InstPointMap {
    type Error = crate::model_storage::ModelStorageVersionMismatch;

    /// 版本核对的唯一入口。
    ///
    /// - dbnum 版本对不上 → `Err`，`source = Dbnum`（整库还没重建）；
    /// - dbnum 对得上但这一行是更早的形状写的 → `Err`，`source = Row`；
    /// - 版本对得上但没有行 → `Ok`，空 map（这根还没发布过 / 这件真没有 P 点）。
    fn try_from(row: InstPointMapRow) -> Result<Self, Self::Error> {
        let dbnum = row.dbnum;
        crate::model_storage::check_dbnum(dbnum, row.dbnum_storage_version)?;
        let ptset_map = match row.ptset_row {
            None => BTreeMap::new(),
            Some(pe_ptset_row) => {
                crate::model_storage::check_row(dbnum, pe_ptset_row.storage_version)?;
                pe_ptset_row
                    .ptset
                    .into_iter()
                    .map(|(key, point)| {
                        let param = point.into_cate_axis_param(&key);
                        (key, param)
                    })
                    .collect()
            }
        };
        Ok(Self {
            refno: row.refno,
            att_type: row.att_type,
            ptset_map,
        })
    }
}

#[test]
fn ptset_row_reads_the_current_shape() {
    let shape = r#"{
        "refno": "pe:24381_101407",
        "att_type": "ELBO",
        "dbnum": 7997,
        "dbnum_storage_version": 1,
        "ptset_row": { "storage_version": 1,
                       "ptset": { "1": { "pt": [1.0, 2.0, 3.0], "dir": [0.0, 0.0, 1.0], "bore": 100.0, "noun": "PTAX" },
                                  "2": { "pt": [4.0, 5.0, 6.0], "dir": [0.0, 1.0, 0.0], "bore": null, "noun": "PTAX" } } }
    }"#;
    let row = serde_json::from_str::<InstPointMapRow>(shape).unwrap();
    let map = InstPointMap::try_from(row).unwrap();
    assert_eq!(map.ptset_map.len(), 2);
    assert_eq!(map.ptset_map["1"].number, 1, "点号从键上补");
    assert_eq!(map.ptset_map["1"].pbore, 100.0);
    assert_eq!(map.ptset_map["1"].pt, Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(map.ptset_map["2"].number, 2);
    assert_eq!(map.ptset_map["2"].pbore, 0.0, "bore 求不出要落成 0 = 未设");
}

#[test]
fn legacy_store_is_refused_at_the_dbnum_level() {
    // 老库：没有 model_storage_version 这一列，也没有 pe_ptset 行。
    let shape = r#"{
        "refno": "pe:24381_101407", "att_type": "ELBO", "dbnum": 7997,
        "dbnum_storage_version": null, "ptset_row": null
    }"#;
    let row = serde_json::from_str::<InstPointMapRow>(shape).unwrap();
    let error = InstPointMap::try_from(row).unwrap_err();
    assert_eq!(error.dbnum, 7997);
    assert_eq!(error.stored, None);
    assert_eq!(error.source, crate::model_storage::MismatchSource::Dbnum);
    assert!(
        error.to_string().contains("model/rebuild"),
        "要告诉调用方怎么修: {error}"
    );
}

#[test]
fn row_from_an_older_shape_is_refused_even_when_the_dbnum_matches() {
    let shape = r#"{
        "refno": "pe:24381_101407", "att_type": "ELBO", "dbnum": 7997,
        "dbnum_storage_version": 1,
        "ptset_row": { "storage_version": null, "ptset": {} }
    }"#;
    let row = serde_json::from_str::<InstPointMapRow>(shape).unwrap();
    let error = InstPointMap::try_from(row).unwrap_err();
    assert_eq!(error.source, crate::model_storage::MismatchSource::Row);
}

#[test]
fn a_missing_row_on_a_current_dbnum_is_an_empty_set_not_an_error() {
    // 版本对得上但没有 pe_ptset 行 = 这根还没按新版发布过 / 这件真没有 P 点。
    let shape = r#"{
        "refno": "pe:24381_101407", "att_type": "ELBO", "dbnum": 7997,
        "dbnum_storage_version": 1, "ptset_row": null
    }"#;
    let row = serde_json::from_str::<InstPointMapRow>(shape).unwrap();
    let map = InstPointMap::try_from(row).unwrap();
    assert!(map.ptset_map.is_empty());
}

#[test]
fn test_de_ser() {
    // 原来这里写的是 `"ptset_map": None`——那不是合法 JSON（`None` 裸字面量），
    // serde_json 报 "expected value"，用例从写下来那天起就是红的。
    // 改成省略字段，验真正有意义的那件事：`ptset_map` 缺席时落成空 map。
    let input = r#"{
    "att_type":"WELD",
    "refno":"pe:17414_27235"
}"#;
    let r = serde_json::from_str::<InstPointMap>(input).unwrap();
    assert_eq!(r.att_type, "WELD");
    assert!(r.ptset_map.is_empty());
}
// #[serde_as]
// #[derive(Serialize, Deserialize, Clone, Debug, Default)]
// pub struct InstPointVec {
//     pub id: RefnoEnum,
//     pub att_type: String,
//     #[serde(default)]
//     pub ptset_map: Vec<CateAxisParam>,
// }
//
// impl InstPointVec {
//     pub fn into_point_map(self) -> InstPointMap {
//         let mut map = BTreeMap::new();
//         for p in self.ptset_map {
//             map.entry(p.number).or_insert(p);
//         }
//         InstPointMap {
//             refno: self.id,
//             att_type: self.att_type,
//             ptset_map: map,
//         }
//     }
// }
