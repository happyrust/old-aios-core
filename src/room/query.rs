use crate::accel_tree::acceleration_tree::RStarBoundingBox;
use crate::room::data::RoomElement;
#[cfg(not(target_arch = "wasm32"))]
use crate::room::room::{load_aabb_tree, GLOBAL_AABB_TREE};
use crate::shape::pdms_shape::PlantMesh;
use crate::{query_insts, RefU64, RefnoEnum, SUL_DB};
use glam::Vec3;
use nalgebra::Point3;
use parry3d::bounding_volume::Aabb;
use parry3d::math::Isometry;
use parry3d::query::PointQuery;
use parry3d::shape::TriMeshFlags;

#[cfg(not(target_arch = "wasm32"))]
pub async fn query_room_number_by_point(point: Vec3) -> anyhow::Result<Option<String>> {
    let Some(refno) = query_room_panel_by_point(point).await? else {
        return Ok(None);
    };
    let mut response = SUL_DB
        .query(format!(
            r#"
            select value room_num from only {}<-room_panel_relate limit 1;
        "#,
            refno.to_pe_key()
        ))
        .await?;
    // dbg!(&response);
    let room_number: Option<String> = response.take(0)?;
    Ok(room_number)
}

//传进来的是世界坐标系下的点
///
/// 候选面板取自全局 `GLOBAL_AABB_TREE` 并按 `noun` 过滤，不再维护第二棵只装面板的树
/// （ADR-010 §6）。原先那棵 `GLOBAL_ROOM_AABB_TREE` 唯一的填充入口 `load_room_aabb_tree`
/// 里 SQL 括号未闭合、内层 select 无主表，解析必失败，而这里又是 `.unwrap()`——
/// 整条反向路径实际是 panic 的死代码。一棵树也省掉了两棵互相漂移的可能。
#[cfg(not(target_arch = "wasm32"))]
pub async fn query_room_panel_by_point(point: Vec3) -> anyhow::Result<Option<RefnoEnum>> {
    //通过rtree 找到所在的几个房间可能
    load_aabb_tree().await?;
    let pt: Point3<f32> = point.into();
    let point_aabb = Aabb::new(pt, pt);
    let read = GLOBAL_AABB_TREE.read().await;
    let contains_query = read
        .locate_intersecting_bounds(&point_aabb)
        .filter(|b| b.noun == "PANE")
        .collect::<Vec<_>>();

    // dbg!(&contains_query);
    let refnos: Vec<RefnoEnum> = contains_query
        .iter()
        .map(|r| r.refno.into())
        .collect::<Vec<_>>();
    let insts = query_insts(&refnos, true).await?;
    // dbg!(&insts);
    for RStarBoundingBox { refno, aabb, .. } in contains_query {
        let Some(geom_inst) = insts.iter().find(|x| x.refno.refno() == *refno) else {
            continue;
        };
        for inst in &geom_inst.insts {
            if (aabb.mins[0] > 1000000.0) {
                return continue;
            }
            let Ok(mesh) =
                PlantMesh::des_mesh_file(&format!("assets/meshes/{}.mesh", inst.geo_hash))
            else {
                continue;
            };
            let Some(mut tri_mesh) = mesh.get_tri_mesh_with_flag(
                (geom_inst.world_trans * inst.transform).compute_matrix(),
                TriMeshFlags::ORIENTED,
            ) else {
                continue;
            };
            if tri_mesh.contains_point(&Isometry::identity(), &pt) {
                // dbg!(refno);
                return Ok(Some((*refno).into()));
            }
        }
    }

    Ok(None)
}
