//! 模型存储架构版本号。
//!
//! ## 为什么要这个号
//!
//! 模型面（`pe` / `inst_relate` / `pe_ptset` / `dbnum_watermark` …）的行形状会随版本演进：
//! 字段增删、键的语义变化、坐标单位或参考系变化。读侧如果按新形状去读一份老库，
//! 得到的是「查不到」「空集」「0.0」这类静默错误——比报错难查得多。
//! 这个号的作用只有一个：**让读侧能判断一份库的行形状自己读不读得懂**，读不懂就报错，
//! 由人决定何时重建，而不是拿空集冒充「这件没有点」。
//!
//! ## 三条正交的版本轴（不要混用）
//!
//! | 轴 | 常量 / 字段 | 回答的问题 | 定义位置 |
//! |---|---|---|---|
//! | 存储架构版本 | [`MODEL_STORAGE_VERSION`]，落在 `dbnum_watermark:{dbnum}.model_storage_version` 与 `pe_ptset.storage_version` | 行形状**能不能读** | 本文件 |
//! | 点集求值版本 | `e3d_model::PTSET_EVAL_VERSION`，落在 `pe_ptset.generator` | 点**算得对不对** | `vendor/e3d-model/src/catalogue_point.rs:199` |
//! | 几何生成器指纹 | `dbnum_watermark:{dbnum}.model_generator` | 哪一版 e3d-model **算的几何** | `gen-model/src/data_interface/task_end.rs:167` |
//!
//! 求值规则变了（比如弯头弧长公式改）→ 只动 `PTSET_EVAL_VERSION`，本常量不动，
//! 因为老行照样读得懂、只是数值要重算。行形状变了 → 只动本常量。
//!
//! ## 何时加一
//!
//! 只有这三种情况：
//!
//! 1. `pe_ptset` 行的字段形状变了（增删必填字段、改字段类型）；
//! 2. `ptset` 对象里点号的键语义变了（比如从 P 号改成别的编号体系）；
//! 3. `pt` / `dir` 的坐标单位或参考系变了（比如 mm 改 m、局部系改世界系）。
//!
//! 改读法但形状没变（比如换个索引、换个函数名）**不加**。
//!
//! ## 粒度：为什么是每 dbnum
//!
//! 重建的单位是 dbnum（`POST /api/v1/dbnums/{dbnum}/model/rebuild`），不是整库：
//! 一个 SurrealDB namespace 下并存几十个 dbnum，逐个重建、逐个追平。
//! 所以版本也记在 dbnum 上。整库级的 `data_face_signature:current.schema_version`
//! 是另一回事——那条记录的是「这个库归谁、是什么数据面」，不匹配直接拒绝启动，
//! 拿它管行形状会把所有现存库一起拒在门外，也分不清哪个 dbnum 已经重建过。
//!
//! 「没有 `pe_ptset` 行」不等于「老库」：一根管件还没按新版发布过、或者这件本来
//! 就没有 P 点，都会没有行。判老库只看 `dbnum_watermark.model_storage_version`。
//!
//! ## 缺列 = 0
//!
//! 这个列是后加的。没写过它的库读出来是 `None`，语义等同于「前版本化」，
//! 也就是 [`MODEL_STORAGE_VERSION`] 的任何一个正式版本都不等于它。

/// 当前进程能读的模型存储架构版本。
///
/// 加一的条件见模块注释「何时加一」三条。
pub const MODEL_STORAGE_VERSION: u32 = 1;

/// 版本号在 dbnum 水位行上的列名。
pub const STORAGE_VERSION_COLUMN: &str = "model_storage_version";

/// 版本号在 `pe_ptset` 行上的列名。
pub const PTSET_ROW_STORAGE_VERSION_COLUMN: &str = "storage_version";

/// 版本不匹配发生在哪一层。
///
/// `Dbnum` = 整个库都还没重建过；`Row` = 库的版本对得上，但这一行是更早的形状写的
/// （半重建、或手工塞进去的行）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MismatchSource {
    Dbnum,
    Row,
}

impl MismatchSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dbnum => "dbnum",
            Self::Row => "row",
        }
    }
}

/// 存储架构版本不匹配。
///
/// `Display` 的文本是稳定契约：rs-server 的 HTTP 层、`fn::bran_comp_len` 的
/// SurrealQL `THROW`、日志巡检都按同一个码 `MODEL_STORAGE_VERSION_MISMATCH` 认。
/// 改文本会同时打断这三处，不要改。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelStorageVersionMismatch {
    /// 出问题的库号。
    pub dbnum: u32,
    /// 库里记的版本；`None` = 没写过这一列 = 前版本化。
    pub stored: Option<u32>,
    /// 本进程期望的版本。
    pub expected: u32,
    /// 在 dbnum 层还是行层发现的。
    pub source: MismatchSource,
}

impl std::fmt::Display for ModelStorageVersionMismatch {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let stored = match self.stored {
            Some(version) => version.to_string(),
            None => "none".to_string(),
        };
        write!(
            formatter,
            "{} dbnum={} stored={} expected={} source={}: 该库需 POST /api/v1/dbnums/{}/model/rebuild",
            Self::CODE,
            self.dbnum,
            stored,
            self.expected,
            self.source.as_str(),
            self.dbnum
        )
    }
}

impl std::error::Error for ModelStorageVersionMismatch {}

impl ModelStorageVersionMismatch {
    /// 错误码。调用方按它分流，不要匹配 Display 全文。
    pub const CODE: &'static str = "MODEL_STORAGE_VERSION_MISMATCH";

    /// 可以直接发给运维的重建命令。
    pub fn rebuild_hint(&self) -> String {
        format!("POST /api/v1/dbnums/{}/model/rebuild", self.dbnum)
    }

    /// 这个错误文本里是否带本错误码。
    ///
    /// 给「错误从 SurrealQL `THROW` 里以字符串形式回来」那条路径用
    /// （`fn::bran_comp_len` 经 `rs-server/src/datacenter/query.rs` 调用）。
    pub fn matches_code(text: &str) -> bool {
        text.contains(Self::CODE)
    }
}

/// 核对一个 dbnum 的存储架构版本。
///
/// `stored` 传 `None` 表示那一行没写过这一列（老库）。
pub fn check_dbnum(dbnum: u32, stored: Option<u32>) -> Result<(), ModelStorageVersionMismatch> {
    if stored == Some(MODEL_STORAGE_VERSION) {
        return Ok(());
    }
    Err(ModelStorageVersionMismatch {
        dbnum,
        stored,
        expected: MODEL_STORAGE_VERSION,
        source: MismatchSource::Dbnum,
    })
}

/// 核对 `pe_ptset` 单行的存储架构版本。
///
/// 调用前应当先 [`check_dbnum`]：dbnum 版本对不上时，行版本没有意义。
pub fn check_row(dbnum: u32, stored: Option<u32>) -> Result<(), ModelStorageVersionMismatch> {
    if stored == Some(MODEL_STORAGE_VERSION) {
        return Ok(());
    }
    Err(ModelStorageVersionMismatch {
        dbnum,
        stored,
        expected: MODEL_STORAGE_VERSION,
        source: MismatchSource::Row,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dbnum_version_matches_only_the_exact_current_version() {
        assert!(check_dbnum(7997, Some(MODEL_STORAGE_VERSION)).is_ok());
        assert!(check_dbnum(7997, None).is_err(), "缺列 = 老库");
        assert!(check_dbnum(7997, Some(0)).is_err(), "0 不是合法版本");
        assert!(check_dbnum(7997, Some(2)).is_err(), "比本进程新也读不懂");
    }

    #[test]
    fn mismatch_text_is_a_stable_contract() {
        let error = check_dbnum(7997, None).unwrap_err();
        let text = error.to_string();
        assert!(text.contains("MODEL_STORAGE_VERSION_MISMATCH"), "{text}");
        assert!(text.contains("dbnum=7997"), "{text}");
        assert!(text.contains("stored=none"), "{text}");
        assert!(text.contains("expected=1"), "{text}");
        assert!(text.contains("source=dbnum"), "{text}");
        assert!(text.contains("POST /api/v1/dbnums/7997/model/rebuild"), "{text}");
        assert_eq!(error.rebuild_hint(), "POST /api/v1/dbnums/7997/model/rebuild");
        assert!(ModelStorageVersionMismatch::matches_code(&text));
    }

    #[test]
    fn row_source_is_distinguishable_from_dbnum_source() {
        let row = check_row(7997, Some(0)).unwrap_err();
        assert_eq!(row.source, MismatchSource::Row);
        assert!(row.to_string().contains("source=row"), "{row}");
        assert_ne!(
            row.to_string(),
            check_dbnum(7997, Some(0)).unwrap_err().to_string()
        );
    }
}
