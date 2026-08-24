use parry3d::bounding_volume::Aabb;

use crate::{RefU64, hash::gen_bytes_hash, tool::db_tool::db1_dehash};

#[test]
fn test_dehash_uda() {
    let hash = 642952055;
    dbg!(db1_dehash(hash));
    let hash = 413837091u32;
    dbg!(db1_dehash(hash));
    let hash = 430614307;
    dbg!(db1_dehash(hash));
    let hash = 738252150;
    dbg!(db1_dehash(hash));
}

#[test]
fn test_gen_hash() {
    let aabb = Aabb::new([-1.0, -1.0, -1.0].into(), [1.0, 1.0, 1.0].into());
    let hash1 = gen_bytes_hash::<_, 512>(&aabb);
    let aabb = Aabb::new([1.0, -1.0, -1.0].into(), [1.0, 1.0, 1.0].into());
    let hash2 = gen_bytes_hash::<_, 512>(&aabb);
    assert_ne!(hash1, hash2);
}
