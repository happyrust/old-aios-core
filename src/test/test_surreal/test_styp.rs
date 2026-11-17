use crate::rs_surreal;

/// 连上库时把数字样子的 `DB.STYP` 改成整数。第二次调用必须是空操作。
#[tokio::test]
async fn test_repair_string_db_styp_stores_integers() {
    crate::init_test_surreal().await.unwrap();
    let updated = rs_surreal::mdb::repair_string_db_styp().await.unwrap();
    assert_eq!(updated, 0, "连上库之后不应再剩数字字符串形式的 STYP");

    let mut response = crate::SUL_DB
        .query("SELECT count() AS n FROM DB WHERE type::is::string(STYP) GROUP ALL;")
        .await
        .unwrap();
    let rows: Vec<serde_json::Value> = response.take(0).unwrap();
    let leftover = rows
        .first()
        .and_then(|row| row.get("n"))
        .and_then(|n| n.as_u64())
        .unwrap_or(0);
    assert_eq!(leftover, 0, "DB.STYP 仍有字符串: {rows:?}");

    let world = rs_surreal::get_world("/ALL".to_string()).await.unwrap();
    assert!(world.is_some(), "STYP 改成整数后 /ALL 应能打开世界");
}
