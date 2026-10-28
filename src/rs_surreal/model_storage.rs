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
//! ## 落在哪：整库一条记录
//!
//! 表 `storage_version`，只有一条记录 [`STORAGE_VERSION_RECORD`]：
//!
//! ```text
//! storage_version:current = {
//!     version: 1,          // 行形状版本，与 MODEL_STORAGE_VERSION 同值
//!     shape:   "…",        // 这一版的行形状说明（人读）
//!     rebuild: "…",        // 改版本号之后必须做的事（人读）
//! }
//! ```
//!
//! 行形状是整个模型面共用的一份约定，不是某张表、也不是某个 dbnum 的属性：
//! 同一份代码写出来的行不会有第二种形状。所以只有这一条记录，没有逐库、也没有逐行的副本。
//! 重建的单位仍然是 dbnum（`POST /api/v1/dbnums/{dbnum}/model/rebuild`），
//! 但「重建完没有」不靠另一处版本号回答——重建会重写这个库的全部模型行，
//! 逐库进度看的是水位（`dbnum_watermark.model_sesno`），不是版本。
//!
//! ## 三条正交的版本轴（不要混用）
//!
//! | 轴 | 常量 / 字段 | 回答的问题 | 定义位置 |
//! |---|---|---|---|
//! | 存储架构版本 | [`MODEL_STORAGE_VERSION`]，落在 [`STORAGE_VERSION_RECORD`]`.version` | 行形状**能不能读** | 本文件 |
//! | 点集求值版本 | `e3d_model::PTSET_EVAL_VERSION`，落在 `pe_ptset.generator` | 点**算得对不对** | `vendor/e3d-model/src/catalogue_point.rs` |
//! | 几何生成器指纹 | `dbnum_watermark:{dbnum}.model_generator` | 哪一版 e3d-model **算的几何** | `gen-model/src/data_interface/task_end.rs` |
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
//! ## 缺记录 = 老库
//!
//! 这份记录是后加的。没有它的库读出来是 `None`，语义等同于「前版本化」，
//! 也就是 [`MODEL_STORAGE_VERSION`] 的任何一个正式版本都不等于它。
//!
//! 「没有 `pe_ptset` 行」不等于「老库」：一根管件还没按新版发布过、或者这件本来
//! 就没有 P 点，都会没有行。判老库只看 [`STORAGE_VERSION_RECORD`]。

/// 当前进程能读的模型存储架构版本。
///
/// 加一的条件见模块注释「何时加一」三条。改这个数必须同时改 [`SHAPE_NOTE`]
/// 与 [`REBUILD_NOTE`]：那两格是给人读的，写的是这一版的实际形状与重建要求。
pub const MODEL_STORAGE_VERSION: u32 = 1;

/// 版本记录的表名。
pub const STORAGE_VERSION_TABLE: &str = "storage_version";

/// 版本记录的唯一一条记录的 id（`storage_version:current`）。
pub const STORAGE_VERSION_RECORD: &str = "storage_version:current";

/// 记录里存版本号的字段名。
pub const VERSION_COLUMN: &str = "version";

/// 记录里存行形状说明的字段名。
pub const SHAPE_COLUMN: &str = "shape";

/// 记录里存重建要求的字段名。
pub const REBUILD_COLUMN: &str = "rebuild";

/// 本版本的行形状说明，随版本记录一起落库（人读，不参与任何判断）。
///
/// 写清读侧真正依赖的那几格：字段名、类型、坐标单位与参考系。
/// 版本加一时这段跟着改写，别让它和代码分叉。
pub const SHAPE_NOTE: &str = "\
模型面行形状 v1。\
pe_ptset:{pe} = { pe, root, dbnum, noun, \
ptset: { \"<P 号>\": { pt: [x,y,z], dir: [x,y,z], bore, noun } }, \
count, unresolved: [{ number, reason }], sesno, generator, reason, written_at }。\
坐标是管件局部系，单位 mm。没有行 = 这根还没按本版发布过；\
有行但 ptset: {} = 这件真没有 P 点（reason 说明）。";

/// 改版本号之后必须做的事，随版本记录一起落库（人读，不参与任何判断）。
pub const REBUILD_NOTE: &str = "\
行形状变了，老库的模型行读不懂。逐库重建：\
POST /api/v1/dbnums/{dbnum}/model/rebuild（低峰、串行，每库重建期间该库不可见）。\
重建前老库的 P 点长度函数会报 MODEL_STORAGE_VERSION_MISMATCH，不再静默回 0。";

/// 版本不匹配属于哪一种。
///
/// 只有两种：整份库根本没有这份记录（前版本化），或者记录里的版本不是本进程读得懂的那一版。
/// 没有第三处可查——版本只有这一个来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MismatchSource {
    /// 库里没有 `storage_version:current` 这一条 = 前版本化。
    Missing,
    /// 有记录，但 `version` 不等于本进程的 [`MODEL_STORAGE_VERSION`]。
    Version,
}

impl MismatchSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Version => "version",
        }
    }
}

/// 存储架构版本不匹配。
///
/// `Display` 的文本是稳定契约：rs-server 的 HTTP 层、`fn::pe_ptset_of` 的
/// SurrealQL `THROW`、日志巡检都按同一个码 `MODEL_STORAGE_VERSION_MISMATCH` 认。
/// 改文本会同时打断这三处，不要改。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelStorageVersionMismatch {
    /// 出问题的库号（记录是整库一条，库号只用来拼重建命令与定位日志）。
    pub dbnum: u32,
    /// 记录里的 `version`；`None` = 没有这条记录 = 前版本化。
    pub stored: Option<u32>,
    /// 本进程期望的版本。
    pub expected: u32,
    /// 缺记录还是版本不符。
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
    /// 给「错误从 SurrealQL `THROW` 里以字符串形式回来」那条路径用。
    pub fn matches_code(text: &str) -> bool {
        text.contains(Self::CODE)
    }
}

/// 核对库里的存储架构版本。
///
/// `stored` 传 `None` 表示库里没有 [`STORAGE_VERSION_RECORD`] 这条记录（老库）。
/// `dbnum` 只用于错误文本与重建命令。
pub fn check(dbnum: u32, stored: Option<u32>) -> Result<(), ModelStorageVersionMismatch> {
    let source = match stored {
        None => MismatchSource::Missing,
        Some(version) if version != MODEL_STORAGE_VERSION => MismatchSource::Version,
        Some(_) => return Ok(()),
    };
    Err(ModelStorageVersionMismatch {
        dbnum,
        stored,
        expected: MODEL_STORAGE_VERSION,
        source,
    })
}

/// 版本记录的三个字段，写侧用它拼 UPSERT。
///
/// `shape` / `rebuild` 两格永远是本文件里的常量：它们与 `version` 必须同代，
/// 让人读的说明跟着代码走，不靠人工在库里改。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageVersionRecord {
    pub version: u32,
    pub shape: &'static str,
    pub rebuild: &'static str,
}

impl StorageVersionRecord {
    /// 本进程要写进库的那一份。
    pub fn current() -> Self {
        Self {
            version: MODEL_STORAGE_VERSION,
            shape: SHAPE_NOTE,
            rebuild: REBUILD_NOTE,
        }
    }
}

impl Default for StorageVersionRecord {
    fn default() -> Self {
        Self::current()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_record_is_legacy_and_only_the_exact_version_reads() {
        assert!(check(7997, Some(MODEL_STORAGE_VERSION)).is_ok());
        assert_eq!(
            check(7997, None).unwrap_err().source,
            MismatchSource::Missing,
            "缺记录 = 老库"
        );
        assert_eq!(
            check(7997, Some(0)).unwrap_err().source,
            MismatchSource::Version,
            "0 不是合法版本"
        );
        assert_eq!(
            check(7997, Some(MODEL_STORAGE_VERSION + 1))
                .unwrap_err()
                .source,
            MismatchSource::Version,
            "比本进程新也读不懂"
        );
    }

    #[test]
    fn mismatch_text_is_a_stable_contract() {
        let error = check(7997, None).unwrap_err();
        let text = error.to_string();
        assert!(text.contains("MODEL_STORAGE_VERSION_MISMATCH"), "{text}");
        assert!(text.contains("dbnum=7997"), "{text}");
        assert!(text.contains("stored=none"), "{text}");
        assert!(text.contains("expected=1"), "{text}");
        assert!(text.contains("source=missing"), "{text}");
        assert!(
            text.contains("POST /api/v1/dbnums/7997/model/rebuild"),
            "{text}"
        );
        assert_eq!(
            error.rebuild_hint(),
            "POST /api/v1/dbnums/7997/model/rebuild"
        );
        assert!(ModelStorageVersionMismatch::matches_code(&text));
    }

    #[test]
    fn a_version_mismatch_reads_differently_from_a_missing_record() {
        let missing = check(7997, None).unwrap_err();
        let version = check(7997, Some(MODEL_STORAGE_VERSION + 1)).unwrap_err();
        assert!(version.to_string().contains("source=version"), "{version}");
        assert_ne!(missing.to_string(), version.to_string());
    }

    #[test]
    fn the_record_carries_the_version_with_its_two_notes() {
        let record = StorageVersionRecord::current();
        assert_eq!(record.version, MODEL_STORAGE_VERSION);
        assert!(!record.shape.is_empty(), "形状说明不能空");
        assert!(!record.rebuild.is_empty(), "重建要求不能空");
        assert!(
            record.rebuild.contains("/model/rebuild"),
            "重建要求要能照抄: {}",
            record.rebuild
        );
        assert!(
            record.shape.contains("pe_ptset"),
            "形状说明要写清读侧依赖的表: {}",
            record.shape
        );
    }

    #[test]
    fn the_record_id_names_the_table_it_lives_in() {
        assert!(STORAGE_VERSION_RECORD.starts_with(STORAGE_VERSION_TABLE));
        assert!(STORAGE_VERSION_RECORD.ends_with(":current"));
    }
}
