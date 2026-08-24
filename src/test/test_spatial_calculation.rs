use crate::init_test_surreal;
use crate::room::query::*;
use crate::rs_surreal;
use glam::Vec3;

#[tokio::test]
async fn test_query_point_room() -> anyhow::Result<()> {
    init_test_surreal().await;
    // let refno = "13292_92".into();
    // let pe = rs_surreal::get_pe(refno).await.unwrap();
    // dbg!(pe);
    test_query_room_by_point(Vec3::new(-20834.78, 9160.44, 17850.0)).await;
    test_query_room_by_point(Vec3::new(19381.85, 4055.68, 16679.0)).await;

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
async fn test_query_room_by_point(point: Vec3) {
    let mut time = std::time::Instant::now();
    let result = query_room_number_by_point(point).await.unwrap();
    dbg!(&result);
    println!(
        "query_room_number_by_point花费时间: {} ms",
        time.elapsed().as_millis()
    );
}
