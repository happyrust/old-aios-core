use crate::rs_surreal::spatial::query_neareast_below_points;
use crate::{RefnoEnum, SUL_DB, query_neareast_along_axis};
use glam::Vec3;
use parry3d::bounding_volume::{Aabb, BoundingVolume};

/// 一个请求里塞多少条 `RELATE`。532 个阀门量级，分批只为限制单请求体积。
const WRITE_BATCH: usize = 200;

/// 一条 SQL 里点名多少个阀门去取 `inst_relate.aabb.d`。阀门量级 600 上下，一次取完约 60KB。
const AABB_QUERY_BATCH: usize = 500;

/// 阀门底面探测点：4 角 + 底面中心，按 0.1mm 去重。
///
/// 阀门以前只从 `world_transform.translation`（原点）打一条 `-Z` 射线。真库实测
/// `pe:24381_177593 / 177639 / 177681` 三台穿孔阀门就是这么落空的：原点在阀体中心，
/// 正下方正好是开孔 / 管嘴镂空处，而阀体底面四角下方是实的。同时原点射线也没扣掉阀体
/// 下半部高度，算出来的距离比「底面到楼板」大半个阀体。
fn bottom_points(aabbs: &[Aabb]) -> Vec<Vec3> {
    let mut seen = std::collections::HashSet::new();
    let mut points = Vec::new();
    let mut push = |pt: Vec3, points: &mut Vec<Vec3>| {
        let key = (
            (pt.x * 10.0).round() as i64,
            (pt.y * 10.0).round() as i64,
            (pt.z * 10.0).round() as i64,
        );
        if seen.insert(key) {
            points.push(pt);
        }
    };
    for aabb in aabbs {
        for corner in &aabb.vertices()[..4] {
            push(Vec3::from(*corner), &mut points);
        }
        let center = aabb.center();
        push(Vec3::new(center.x, center.y, aabb.mins.z), &mut points);
    }
    points
}

/// 单台阀门的最近楼板：阀体底面 4 角 + 底面中心向 -Z 打射线，没有 AABB 时回退原点射线。
///
/// 全量（[`cal_valve_nearest_floor`]）与增量（主仓 `refresh_one_valve`）**必须**走同一个
/// 函数：两条路径口径不一致时，同一台阀门在启动全量与增量重算之间会算出不同距离，
/// `nearest_relate` 的 `dist` 就会随触发方式跳变。
pub async fn valve_nearest_floor(valve: RefnoEnum) -> anyhow::Result<Option<(RefnoEnum, f32)>> {
    let Some(aabb) = valve_own_aabb(valve).await? else {
        return query_neareast_along_axis(valve, Vec3::NEG_Z, "FLOOR").await;
    };
    let points = bottom_points(std::slice::from_ref(&aabb));
    let hits = query_neareast_below_points(&points, "FLOOR").await?;
    let mut best: Option<(RefnoEnum, f32)> = None;
    for hit in hits.into_iter().flatten() {
        let better = match best {
            None => true,
            // 同距时按 refno 定序，保证同一台阀门重跑连到同一块楼板。
            Some(current) => hit.1 < current.1 || (hit.1 == current.1 && hit.0 < current.0),
        };
        if better {
            best = Some(hit);
        }
    }
    Ok(best)
}

/// 一个阀门自己的 `inst_relate.aabb.d`；没有几何时为 `None`。
async fn valve_own_aabb(valve: RefnoEnum) -> anyhow::Result<Option<Aabb>> {
    #[derive(serde::Deserialize)]
    struct Row {
        aabb: Option<Aabb>,
    }
    let mut response = SUL_DB
        .query(format!(
            "SELECT type::thing('inst_relate', record::id(id)).aabb.d AS aabb FROM ONLY {};",
            valve.to_pe_key()
        ))
        .await?
        .check()?;
    let row: Option<Row> = response.take(0)?;
    Ok(row.and_then(|row| row.aabb))
}

/// 给还没有楼板边的阀门挂上 `nearest_relate`（阀门底面 4 角 + 中心向 -Z 打射线，
/// 取最近的 FLOOR）。
///
/// 返回本次新写入的边数。
///
/// 2026-11-01 修：
///
/// 1. 原来按「还没有边」过滤、再用 `START offset` 翻页。每写一批边，过滤结果就变少，
///    偏移量却照样增加，于是跳过一部分行。改成先一次取出全部 id 再算。
/// 2. `take(0).unwrap()` 改成上抛；写库结果原来不看，`query()` 不检查语句级错误，
///    写失败就是静默无边。现在 `.check()`。
/// 3. 原来用 `sqls.join(";")`，而每条本身已带 `;`，拼出来是 `;;`。
///
/// 2026-11-08 修（底面多点探测）：
///
/// 4. 射线起点从「阀门原点」改成「阀体底面 4 角 + 底面中心」，没有 AABB 时才回退原点。
///    真库实测 `pe:24381_177593 / 177639 / 177681` 三台阀门原点正下方是管嘴镂空，
///    单点射线必然落空；底面四角却有实网格。同时底面起点扣掉了阀体下半部高度，
///    距离才是真正的「底面到楼板」。
/// 5. 全部阀门一次性取 AABB、一次性批量打射线（[`query_neareast_below_points`]）：
///    以前每个阀门一次 R-tree 遍历，现在 600 台阀门共用一次包络粗筛。
pub async fn cal_valve_nearest_floor() -> anyhow::Result<usize> {
    let mut response = SUL_DB
        .query("SELECT VALUE REFNO FROM VALV WHERE array::len(REFNO->nearest_relate) = 0;")
        .await?
        .check()?;
    let refnos: Vec<RefnoEnum> = response.take(0)?;
    if refnos.is_empty() {
        return Ok(0);
    }

    // 一次取全部阀门的自身 AABB。缺 AABB 的（没有几何）走原点射线兜底。
    let mut boxes: std::collections::HashMap<RefnoEnum, Aabb> = std::collections::HashMap::new();
    for chunk in refnos.chunks(AABB_QUERY_BATCH) {
        let keys = chunk
            .iter()
            .map(RefnoEnum::to_pe_key)
            .collect::<Vec<_>>()
            .join(",");
        #[derive(serde::Deserialize)]
        struct Row {
            id: RefnoEnum,
            aabb: Option<Aabb>,
        }
        let mut response = SUL_DB
            .query(format!(
                "SELECT id, type::thing('inst_relate', record::id(id)).aabb.d AS aabb FROM [{keys}];"
            ))
            .await?
            .check()?;
        let rows: Vec<Row> = response.take(0)?;
        for row in rows {
            if let Some(aabb) = row.aabb {
                boxes.insert(row.id, aabb);
            }
        }
    }

    // 全部探测点摊平成一张表，记下每个点属于哪个阀门。
    let mut points: Vec<Vec3> = Vec::new();
    let mut owners: Vec<RefnoEnum> = Vec::new();
    for refno in &refnos {
        let Some(aabb) = boxes.get(refno) else {
            continue;
        };
        for pt in bottom_points(std::slice::from_ref(aabb)) {
            points.push(pt);
            owners.push(*refno);
        }
    }

    // 一次批量射线：所有点共用一次包络粗筛。
    let hits = if points.is_empty() {
        Vec::new()
    } else {
        query_neareast_below_points(&points, "FLOOR").await?
    };

    let mut best: std::collections::HashMap<RefnoEnum, (RefnoEnum, f32)> =
        std::collections::HashMap::new();
    for (hit, owner) in hits.into_iter().zip(owners.iter().copied()) {
        let Some((floor, dist)) = hit else {
            continue;
        };
        let better = match best.get(&owner) {
            None => true,
            // 同距时按 refno 定序，保证同一台阀门重跑连到同一块楼板。
            Some(current) => dist < current.1 || (dist == current.1 && floor < current.0),
        };
        if better {
            best.insert(owner, (floor, dist));
        }
    }

    // 没有 AABB 的阀门回退到原点射线（历史行为），只补批量结果里没有的那些。
    for refno in &refnos {
        if best.contains_key(refno) || boxes.contains_key(refno) {
            continue;
        }
        match query_neareast_along_axis(*refno, Vec3::NEG_Z, "FLOOR").await {
            Ok(Some(hit)) => {
                best.insert(*refno, hit);
            }
            Ok(None) => {}
            Err(error) => log::warn!("阀门最近楼板查询失败 refno={refno:?}: {error}"),
        }
    }

    let mut statements = Vec::new();
    for refno in &refnos {
        if let Some((nearest, dist)) = best.get(refno) {
            statements.push(format!(
                "RELATE {}->nearest_relate->FLOOR:{} SET dist = {dist};",
                refno.to_pe_key(),
                nearest
            ));
        }
    }

    let written = statements.len();
    for batch in statements.chunks(WRITE_BATCH) {
        SUL_DB
            .query(batch.concat())
            .await
            .and_then(|response| response.check())
            .map_err(|error| anyhow::anyhow!("写入阀门 nearest_relate 失败: {error}"))?;
    }
    Ok(written)
}
