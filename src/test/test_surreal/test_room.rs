use crate::{rs_surreal, SUL_DB, tool::math_tool};

use std::sync::Arc;
use glam::Mat3;
use surrealdb::types::RecordId as Thing;
use crate::room::data::RoomElement;

#[tokio::test]
async fn test_load_rooms() -> anyhow::Result<()> {
    crate::init_test_surreal().await;
    // let tree = rs_surreal::load_aabb_tree()
    //     .await
    //     .unwrap();


    Ok(())
}

#[tokio::test]
async fn test_save_room_data() -> anyhow::Result<()> {
    crate::init_test_surreal().await;
    let d = RoomElement{
        name: "ROOM-1".to_string(),
        ..Default::default()
    };
    SUL_DB.query(format!("insert ignore into room_ele {}", serde_json::to_string(&d).unwrap())).await.unwrap();

    Ok(())
}
