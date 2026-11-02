use crate::pdms_types::RoomNodes;
use crate::{RefU64, RefnoEnum, SUL_DB};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use serde_with::DisplayFromStr;
use serde_with::serde_as;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::str::FromStr;
use tokio::sync::RwLock;

#[derive(Serialize, Deserialize, Default, Debug, Clone, Hash, Eq, PartialEq)]
pub struct RoomInfo {
    pub name: String,
    pub refno: RefnoEnum,
}

impl Ord for RoomInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for RoomInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// 查询项目的所有房间
///
/// 返回值: k 厂房 v 房间编号
pub async fn query_all_room_name() -> anyhow::Result<HashMap<String, BTreeSet<RoomInfo>>> {
    let mut map = HashMap::new();
    let mut response = SUL_DB
        .query(r#"
           let $f = array::flatten(select value array::flatten(array::flatten(<-pe_owner.in<-pe_owner.in<-pe_owner[where in.noun == 'FRMW'].in.refno.id) )
            from (select value REFNO from SITE where NAME != NONE && string::contains(NAME,'ARCH')));

            return select id as refno ,NAME as name from array::flatten($f) where NAME != NONE;
        "#)
        .await?;
    let results: Vec<RoomInfo> = response.take(1)?;
    for r in results.clone() {
        let room = r.name;
        let split = room.split("-").collect::<Vec<_>>();
        let Some(first) = split.first() else {
            continue;
        };
        let Some(last) = split.last() else {
            continue;
        };
        // if !match_room_name_hd(last) {
        //     continue;
        // };
        map.entry(first[1..].to_string())
            .or_insert_with(BTreeSet::new)
            .insert(RoomInfo {
                name: last.to_string(),
                refno: r.refno,
            });
    }
    Ok(map)
}

/// 查询多个refno所属的房间号，bran和equi也适用
pub async fn query_room_name_from_refnos(
    owner: Vec<RefnoEnum>,
) -> anyhow::Result<HashMap<RefU64, String>> {
    #[derive(Debug, Serialize, Deserialize)]
    struct RoomNameQueryRequest {
        pub id: RefU64,
        pub room: Option<String>,
    }

    let owners = owner
        .into_iter()
        .map(|o| o.to_pe_key())
        .collect::<Vec<String>>()
        .join(",");
    let sql = format!("select id,fn::room_code(id)[0] as room from [{}]", owners);
    let mut response = SUL_DB.query(sql).await?;
    let result: Vec<RoomNameQueryRequest> = response.take(0)?;
    let r = result
        .into_iter()
        .map(|x| (x.id, x.room.unwrap_or("".to_string())))
        .collect::<HashMap<RefU64, String>>();
    Ok(r)
}

/// 查找设备和阀门所属的楼板（最近一块）与它到该楼板的距离。
///
/// 两个字段都是 `Option`：**「没有楼板边」必须与「距离恰好是 0」分得开**。
///
/// 改前返回 `HashMap<RefU64, (String, f32)>`，缺边时 `floor` 填 `""`、`height` 填
/// `0.0`。于是三条消费点（`gy.rs` 两处、`yk_fixing.rs` 一处）写的兜底
///
/// ```ignore
/// .map_or(("".to_string(), world_transform.translation.z), |x| x.clone())
/// ```
///
/// **一次都执行不到**——`get()` 对每个入参都命中，`map_or` 的默认分支是死代码。
/// 结果就是没挂上楼板的阀门以 `install_height = 0`、`floor = ""` 发布。
///
/// 真库 2026-11-02：在用阀门 588 台里 78 台没有 `nearest_relate` 边（7997 库 22 台、
/// 251095 库 56 台，其中 251095 整库没有 FLOOR 记录，属数据事实）；另有真实贴面边
/// 的 `dist` 就是 0。
///
/// SQL 侧用 `math::min(array::flatten([...]))` 而不是 `[0]`：唯一索引是 `(in, out)`
/// 联合唯一、不是 `in` 唯一，数据库层面允许一个构件挂多条边。`[0]` 在多边时取到哪条
/// 不确定，`math::min` 取最近的那块楼板。
///
/// `array::flatten([...])` 那层不能省：没有边时 `->nearest_relate.dist` 求值是
/// `NONE` 而不是空数组，`math::min(NONE)` 直接抛 `Argument 1 was the wrong type`，
/// 整批查询失败；`<array>(NONE)` 在字段投影里也抛 `cannot convert NONE into a array`。
/// `array::flatten([...])` 把两种情形统一成数组，没边时得到 `[]`，
/// `math::min([])` 返回 `NONE`，正好映射成 `None`。
///
/// `out.REFNO` 保持原样取回（`Thing`），不能在 SQL 里转字符串——`NONE` 转字符串
/// 会抛 `Expected a string but cannot convert NONE into a string`。
pub async fn query_equi_or_valv_belong_floors(
    refnos: Vec<RefnoEnum>,
) -> anyhow::Result<HashMap<RefU64, (Option<String>, Option<f32>)>> {
    #[serde_as]
    #[derive(Serialize, Deserialize, Debug)]
    struct BelongFloorResponse {
        /// `id` 是 `from [pe:..]` 里那个 record 本身，反序列化成 `Thing` 再取 id-part。
        ///
        /// 原来这里用 `DisplayFromStr` 转 `RefU64`，靠的是「字段恰好是字符串」这条
        /// 隐含前提；`floor` 一旦从 `String` 换成 `Thing`，同一响应里两种形态混用就会
        /// 报 `expected a string, found Thing`。统一成 `Thing` 后两处都不再依赖
        /// SurrealDB 的隐式字符串化。
        pub id: surrealdb::sql::Thing,
        /// `REFNO` 字段本身存的是 `pe:<ref0>_<ref1>` 形态的 record，不是字符串。
        ///
        /// 不能在 SQL 里写 `<string>(...)` / `type::string(...)`：`NONE` 转字符串
        /// 会直接抛 `Expected a string but cannot convert NONE into a string`，
        /// 整批失败。所以保持原样取回，由 Rust 侧 `Thing` 反序列化后转成
        /// `to_pdms_str()` 的展示形态。
        pub floor: Option<surrealdb::sql::Thing>,
        pub height: Option<f32>,
    }

    let refnos = refnos
        .into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>();
    // 拼成 `[pe:a, pe:b]` 的 **record 字面量**，不能用 `serde_json::to_string`。
    //
    // `serde_json` 产出的是 `["pe:a", "pe:b"]`——字符串数组。在 `from [...]` 里，
    // 字符串元素不是 record，`->nearest_relate` 对它求值得到 `NONE`，
    // `array::flatten([NONE])` 得到 `[null]`（长度 1 而不是 0），于是
    // `math::min([null])` 抛 `Expected a array<number> but found NONE`，整批失败。
    //
    // 2026-11-02 真库实测：`["pe:x"]` 三个都返回 `raw: [null]`、`len: 1`；
    // 换成 `[pe:x]` 后正确返回 `[350]` / `[]` / `[668.3198]`。
    // 改前用 `(->nearest_relate.dist)[0]` 时不暴露——索引取首元素对字符串起点
    // 恰好也能拿到，所以这个坑一直藏着。
    let request = refnos.join(",");
    let sql = format!(
        "select id,(->nearest_relate.out.REFNO)[0] as floor,math::min(array::flatten([->nearest_relate.dist])) as height from [{request}]"
    );
    let mut response = SUL_DB.query(sql).await?;
    let result: Vec<BelongFloorResponse> = response.take(0)?;
    let missing = result.iter().filter(|x| x.height.is_none()).count();
    if missing > 0 {
        println!(
            "[nearest_relate] {}/{} 个构件没有楼板边，调用方应回落到世界坐标 z",
            missing,
            result.len()
        );
    }
    let r = result
        .into_iter()
        .map(|x| {
            let id = RefU64::from_str(&x.id.id.to_string())
                .unwrap_or_else(|_| RefU64::from(0u64));
            (
                id,
                (
                    // `Thing` 的 id-part 是 `24381_39088` 形态，转成发布用的
                    // `24381/39088` 展示形态。`REFNO` 读不出来就是 `None`，
                    // 不再落成空串——空串在发布数据里和「楼板号确实为空」不可区分。
                    x.floor
                        .and_then(|x| RefU64::from_str(&x.id.to_string()).ok())
                        .map(|x| x.to_pdms_str()),
                    x.height,
                ),
            )
        })
        .collect::<HashMap<RefU64, (Option<String>, Option<f32>)>>();
    Ok(r)
}

#[tokio::test]
async fn test_query_all_room_name() {
    // init_test_surreal().await;
    // let r = query_all_room_name().await.unwrap();
    // dbg!(&r);
}
