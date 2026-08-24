use crate::{RefU64, rs_surreal, tool::math_tool::quat_to_pdms_ori_xyz_str};

use glam::Quat;
use std::{sync::Arc, time::Instant};
use surrealdb::sql::Thing;

#[tokio::test]
async fn test_query_inst_refnos() -> anyhow::Result<()> {
    crate::init_test_surreal().await;
    let mut time = Instant::now();
    let refnos = rs_surreal::query_deep_visible_inst_refnos("17496_296344".into())
        .await
        .unwrap();
    dbg!(time.elapsed().as_secs_f32());
    dbg!(refnos);
    Ok(())
}

#[tokio::test]
async fn test_query_instance() -> anyhow::Result<()> {
    crate::init_test_surreal().await;
    use crate::RefnoEnum;

    // let refno: RefU64 = "16389_8737".into();
    // let insts = rs_surreal::query_history_insts(&[(refno, )])).await.unwrap();
    // dbg!(insts);

    let refno: RefnoEnum = "17496_497034".into();
    let insts = rs_surreal::query_insts(&[refno], true).await.unwrap();
    // dbg!(insts);
    Ok(())
}

#[tokio::test]
async fn test_query_pos_neg() -> anyhow::Result<()> {
    crate::init_test_surreal().await;
    let mut time = Instant::now();
    let refnos = crate::geom::query_refno_has_pos_neg_map("24381/36945".into(), Some(false))
        .await
        .unwrap();

    dbg!(refnos);
    // let refnos = crate::geom::query_refno_has_pos_neg_map("15194/1339".into(), Some(true))
    //     .await
    //     .unwrap();

    dbg!(time.elapsed().as_secs_f32());
    // dbg!(refnos);
    Ok(())
}

#[test]
fn test_quat() {
    let q = Quat::from_xyzw(0., 0., 1., 0.);
    dbg!(quat_to_pdms_ori_xyz_str(&q, false));
}

#[tokio::test]
async fn test_query_la_points() -> anyhow::Result<()> {
    crate::init_test_surreal().await;
    let result = crate::point::query_arrive_leave_points_by_cata_hash(&[
        "24381_105223".into(),
        "24381_105231".into(),
    ])
    .await
    .unwrap();

    dbg!(result);
    Ok(())
}
