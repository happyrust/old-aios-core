use crate::SUL_DB;
use crate::{rs_surreal, NamedAttrMap, RefU64};
use glam::Vec3;
use std::sync::Arc;
use surrealdb::types::RecordId as Thing;

#[tokio::test]
async fn test_query_pbs_children() -> anyhow::Result<()> {
    crate::init_test_surreal().await;
    let id = Thing::from(("pbs", "2433251624132273407"));
    let children = rs_surreal::get_children_pbs_nodes(&id).await.unwrap();
    dbg!(children);
    Ok(())
}
