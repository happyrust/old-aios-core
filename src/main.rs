use aios_core::aios_db_mgr::aios_mgr::AiosDBMgr;
use aios_core::room::room::load_aabb_tree;
use aios_core::tool::math_tool::*;
use aios_core::{
    RefU64, get_uda_type_refnos_from_select_refnos, init_test_surreal,
    tool::{
        db_tool::{db1_dehash, db1_hash},
        dir_tool::parse_ori_str_to_quat,
    },
};
use anyhow::Ok;
use glam::{Mat3, Quat};

// #[tokio::main]
// async fn main() -> anyhow::Result<()> {
//     //创建保存元数据里的空间关系结果
//     init_test_surreal().await;
//     // load_aabb_tree().await.unwrap();
//     // create_valve_floor_relations().await.unwrap();
//     dbg!(db1_hash("FJFD"));
//     dbg!(db1_hash("DAMP"));
//     dbg!(db1_hash("MESH"));
//     dbg!(db1_dehash(642952140));
//     // get_uda_type_refnos_from_select_refnos(vec![RefU64::from_str("")])
//     Ok(())
// }

#[test]
fn get_noun_hash() {
    let noun = "CAP";
    let hash = db1_hash(noun);
    dbg!(hash);
    let noun = "WELDTY";
    let hash = db1_hash(noun);
    dbg!(hash);
    let hashes = [534980, 531454, 369970574];
    for hash in hashes {
        let str = db1_dehash(hash);
        dbg!(&hash);
        dbg!(str);
    }
}

fn main() {}
