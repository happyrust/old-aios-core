use crate::{RefnoEnum, SUL_DB, query_neareast_along_axis};
use glam::Vec3;

/// 一个请求里塞多少条 `RELATE`。532 个阀门量级，分批只为限制单请求体积。
const WRITE_BATCH: usize = 200;

/// 给还没有楼板边的阀门挂上 `nearest_relate`（阀门原点向 -Z 打射线，取最近的 FLOOR）。
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
pub async fn cal_valve_nearest_floor() -> anyhow::Result<usize> {
    let mut response = SUL_DB
        .query("SELECT VALUE REFNO FROM VALV WHERE array::len(REFNO->nearest_relate) = 0;")
        .await?
        .check()?;
    let refnos: Vec<RefnoEnum> = response.take(0)?;

    let mut statements = Vec::new();
    for refno in refnos {
        // 单个阀门查询失败不拖垮整批：留痕后跳过，下次全量重算会再试。
        match query_neareast_along_axis(refno, Vec3::NEG_Z, "FLOOR").await {
            Ok(Some((nearest, dist))) => statements.push(format!(
                "RELATE {}->nearest_relate->FLOOR:{} SET dist = {dist};",
                refno.to_pe_key(),
                nearest
            )),
            Ok(None) => {}
            Err(error) => log::warn!("阀门最近楼板查询失败 refno={refno:?}: {error}"),
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
