use crate::RefnoEnum;
use crate::room::room::GLOBAL_AABB_TREE;
use crate::tool::math_tool;
use crate::tool::math_tool::{
    cal_quat_by_zdir_with_xref, dquat_to_pdms_ori_xyz_str, to_pdms_dvec_str, to_pdms_vec_str,
};
use crate::{
    NamedAttrMap, RefU64, SUL_DB,
    accel_tree::acceleration_tree::QueryRay,
    consts::HAS_PLIN_TYPES,
    get_named_attmap,
    pdms_data::{PlinParam, PlinParamData},
    prim_geo::spine::{Spine3D, SpineCurveType, SweepPath3D},
    shape::pdms_shape::LEN_TOL,
    tool::{
        direction_parse::parse_expr_to_dir,
        math_tool::{quat_to_pdms_ori_str, quat_to_pdms_ori_xyz_str},
    },
};
use anyhow::anyhow;
use approx::abs_diff_eq;
use async_recursion::async_recursion;
use bevy_transform::prelude::*;
use cached::proc_macro::cached;
use futures::future::{BoxFuture, FutureExt};
use glam::{DMat3, DMat4, DQuat, DVec3, Mat3, Mat4, Quat, Vec3};
use parry3d::bounding_volume::Aabb;
use parry3d::query::Ray;
use serde::{Deserialize, Serialize};
use serde_with::DisplayFromStr;
use serde_with::serde_as;
use std::{collections::HashSet, f32::consts::E, time::Instant};
use surrealdb::Surreal;
use surrealdb::engine::any::Any;

pub fn cal_ori_by_z_axis_ref_x(v: DVec3) -> DQuat {
    let mut ref_dir = if v.normalize().dot(DVec3::Z).abs() > 0.999 {
        DVec3::Y
    } else {
        DVec3::Z
    };
    let y_dir = v.cross(ref_dir).normalize();
    let x_dir = y_dir.cross(v).normalize();

    let rotation = DQuat::from_mat3(&DMat3::from_cols(x_dir, y_dir, v));
    rotation
}

pub fn cal_spine_ori_by_z_axis_ref_x(v: DVec3, neg: bool) -> DQuat {
    let mut ref_dir = if v.normalize().dot(DVec3::Z).abs() > 0.999 {
        DVec3::Y
    } else if v.normalize().dot(DVec3::Z).abs() < 0.001 {
        DVec3::Y
    } else {
        DVec3::Z
    };
    if neg {
        ref_dir = -ref_dir;
    }

    let y_dir = v.cross(ref_dir).normalize();
    let x_dir = y_dir.cross(v).normalize();

    let rotation = DQuat::from_mat3(&DMat3::from_cols(x_dir, y_dir, v));
    rotation
}

pub fn cal_ori_by_opdir(v: DVec3) -> DQuat {
    let ref_dir = if v.normalize().dot(DVec3::Z).abs() > 0.999 {
        DVec3::NEG_Y * v.z.signum()
    } else {
        DVec3::Z
    };
    let y_dir = v.cross(ref_dir).normalize();
    let x_dir = y_dir.cross(v).normalize();

    let rotation = DQuat::from_mat3(&DMat3::from_cols(x_dir, y_dir, v));
    rotation
}

///通过ydir 计算方位 , 跟z轴这个参考轴有关系
pub fn cal_ori_by_ydir(mut y_ref_axis: DVec3, z_dir: DVec3) -> DQuat {
    if y_ref_axis.dot(z_dir).abs() > 0.99 {
        y_ref_axis = DVec3::Z;
    }
    let ref_dir = y_ref_axis.cross(z_dir).normalize();
    let y_dir = z_dir.cross(ref_dir).normalize();
    let x_dir = y_dir.cross(z_dir).normalize();

    // dbg!(to_pdms_dvec_str(&ref_dir, true));
    // dbg!(to_pdms_dvec_str(&y_dir, true));
    // dbg!(to_pdms_dvec_str(&x_dir, true));

    let rotation = DQuat::from_mat3(&DMat3::from_cols(x_dir, y_dir, z_dir));
    rotation
}

#[test]
fn test_cal_ydir_ori() {
    let z_dir = parse_expr_to_dir("-X").unwrap();
    let y_ref_axis = parse_expr_to_dir("X 30 Y").unwrap();

    let rot = cal_ori_by_ydir(y_ref_axis, z_dir);
    assert_eq!(dquat_to_pdms_ori_xyz_str(&rot, true), "Y is Y and Z is -X");

    let z_dir = parse_expr_to_dir("-X").unwrap();
    let y_ref_axis = parse_expr_to_dir("Z 30 XY").unwrap();

    let rot = cal_ori_by_ydir(y_ref_axis, z_dir);
    assert_eq!(dquat_to_pdms_ori_xyz_str(&rot, true), "Y is Z and Z is -X");
}

pub fn cal_spine_ori(v: DVec3, y_ref_dir: DVec3) -> DQuat {
    let x_dir = y_ref_dir.cross(v).normalize();
    let y_dir = v.cross(x_dir).normalize();

    let rotation = DQuat::from_mat3(&DMat3::from_cols(x_dir, y_dir, v));
    rotation
}

pub fn cal_ori_by_z_axis_ref_y(v: DVec3) -> DQuat {
    let mut ref_dir = if v.normalize().dot(DVec3::Z).abs() > 0.999 {
        DVec3::Y
    } else {
        DVec3::Z
    };

    let x_dir = ref_dir.cross(v).normalize();
    let y_dir = v.cross(x_dir).normalize();

    let rotation = DQuat::from_mat3(&DMat3::from_cols(x_dir, y_dir, v));
    rotation
}

pub fn cal_ori_by_extru_axis(v: DVec3, neg: bool) -> DQuat {
    let mut y_ref_dir = if v.normalize().dot(DVec3::Z).abs() > 0.999 {
        DVec3::X
    } else {
        DVec3::Z
    };
    if neg {
        y_ref_dir = -y_ref_dir;
    }

    let x_dir = y_ref_dir.cross(v).normalize();
    let y_dir = v.cross(x_dir).normalize();
    // dbg!((y_ref_dir, x_dir, y_dir, v));
    let rotation = DQuat::from_mat3(&DMat3::from_cols(x_dir, y_dir, v));
    rotation
}

///根据CUTP 和 轴方向，来计算JOINT的方位
pub fn cal_cutp_ori(axis_dir: DVec3, cutp: DVec3) -> DQuat {
    // let cutp = parse_expr_to_dir("Y 36.85 -X").unwrap();
    // let axis_dir = parse_expr_to_dir("Y 36.85 -X").unwrap();
    let mut y_axis = cutp.cross(axis_dir).normalize();
    let d = cutp.dot(axis_dir).abs();
    // dbg!(d);
    if d > 0.99 {
        y_axis = DVec3::Z;
    }
    let x_axis = axis_dir;
    let z_axis = x_axis.cross(y_axis).normalize();
    // let ref_axis = axis_dir.cross(y_axis).normalize();
    // let z_axis = y_axis.cross(ref_axis).normalize();
    // let x_axis = y_axis.cross(z_axis).normalize();
    // dbg!(z_axis);
    // dbg!(to_pdms_dvec_str(&z_axis, true));
    // // dbg!(to_pdms_dvec_str(&ref_axis, true));
    // dbg!(to_pdms_dvec_str(&y_axis, true));
    // dbg!(to_pdms_dvec_str(&x_axis, true));
    DQuat::from_mat3(&DMat3::from_cols(
        x_axis.into(),
        y_axis.into(),
        z_axis.into(),
    ))
}

pub async fn get_spline_pts(refno: RefnoEnum) -> anyhow::Result<Vec<DVec3>> {
    if let Some(ctx) = super::staging::active_staging_reads() {
        return get_spline_pts_on(ctx.db(), refno).await;
    }
    get_spline_pts_on(&SUL_DB, refno).await
}

/// [`get_spline_pts`] 的显式句柄版。
pub async fn get_spline_pts_on(db: &Surreal<Any>, refno: RefnoEnum) -> anyhow::Result<Vec<DVec3>> {
    let mut response = db.query(
        format!("select value (select in.refno.POS as pos, order_num from <-pe_owner[where in.noun='SPINE'].in<-pe_owner order by order_num).pos from only {}", refno.to_pe_key())).await?;
    let pts: Vec<DVec3> = response.take(0)?;
    Ok(pts)
}

pub async fn get_spline_line_dir(refno: RefnoEnum) -> anyhow::Result<DVec3> {
    let pts = get_spline_pts(refno).await?;
    if pts.len() == 2 {
        return Ok((pts[1] - pts[0]).normalize());
    }
    Err(anyhow!("没有找到两个点"))
}

/// 暂存读上下文在场时不经进程缓存直接算（内部读已被上下文路由到暂存库）；
/// 否则走持久层缓存版，行为与历史一致。
pub async fn get_world_transform(refno: RefnoEnum) -> anyhow::Result<Option<Transform>> {
    if super::staging::active_staging_reads().is_some() {
        return get_world_mat4_impl(refno, false)
            .await
            .map(|m| m.map(|x| Transform::from_matrix(x.as_mat4())));
    }
    get_world_transform_cached(refno).await
}

#[cached(name = "GET_WORLD_TRANSFORM", result = true)]
pub(crate) async fn get_world_transform_cached(
    refno: RefnoEnum,
) -> anyhow::Result<Option<Transform>> {
    get_world_mat4(refno, false)
        .await
        .map(|m| m.map(|x| Transform::from_matrix(x.as_mat4())))
}

//获得世界坐标系
///使用cache，需要从db manager里移除出来
///获得世界坐标系, 需要缓存数据，如果已经存在数据了，直接获取
///
/// ADR-017 读路由：暂存读上下文在场时绕开进程缓存直接计算——缓存键没有「世界」
/// 维度，暂存世界的矩阵一旦写进全局缓存就会泄漏给持久层读者。内部的
/// `get_ancestor_attmaps` / `query_ancestor_refnos` / `get_spline_pts` 各自带
/// 上下文分流，计算体本身无需知道自己在哪个世界。
pub async fn get_world_mat4(refno: RefnoEnum, is_local: bool) -> anyhow::Result<Option<DMat4>> {
    if super::staging::active_staging_reads().is_some() {
        return get_world_mat4_impl(refno, is_local).await;
    }
    get_world_mat4_cached(refno, is_local).await
}

#[cached(name = "GET_WORLD_MAT4", result = true)]
pub(crate) async fn get_world_mat4_cached(
    refno: RefnoEnum,
    is_local: bool,
) -> anyhow::Result<Option<DMat4>> {
    get_world_mat4_impl(refno, is_local).await
}

async fn get_world_mat4_impl(refno: RefnoEnum, is_local: bool) -> anyhow::Result<Option<DMat4>> {
    #[cfg(feature = "profile")]
    let start_ancestors = std::time::Instant::now();
    let mut ancestors: Vec<NamedAttrMap> = super::get_ancestor_attmaps(refno).await?;
    #[cfg(feature = "profile")]
    let elapsed_ancestors = start_ancestors.elapsed();
    #[cfg(feature = "profile")]
    println!("get_ancestor_attmaps took {:?}", elapsed_ancestors);

    #[cfg(feature = "profile")]
    let start_refnos = std::time::Instant::now();
    let ancestor_refnos = crate::query_ancestor_refnos(refno).await?;
    #[cfg(feature = "profile")]
    let elapsed_refnos = start_refnos.elapsed();
    #[cfg(feature = "profile")]
    println!("query_ancestor_refnos took {:?}", elapsed_refnos);
    if ancestor_refnos.len() <= 1 {
        return Ok(Some(DMat4::IDENTITY));
    }
    ancestors.reverse();
    let mut rotation = DQuat::IDENTITY;
    let mut translation = DVec3::ZERO;
    let mut prev_mat4 = DMat4::IDENTITY;
    let mut mat4 = DMat4::IDENTITY;

    let mut owner = refno;
    for (index, atts) in ancestors.windows(2).enumerate() {
        let o_att = &atts[0];
        let att = &atts[1];
        let cur_refno = att.get_refno_or_default();
        let cur_type = att.get_type_str();
        // dbg!(cur_type);
        let owner_type = o_att.get_type_str();
        owner = att.get_owner();
        prev_mat4 = mat4;

        let mut pos = att.get_position().unwrap_or_default().as_dvec3();
        // dbg!(pos);
        let mut quat = DQuat::IDENTITY;
        let mut is_world_quat = false;
        let mut bangle = att.get_f32("BANG").unwrap_or_default() as f64;
        let mut apply_bang = att.contains_key("BANG") && bangle != 0.0;
        //只有GENSEC需要隐藏自己的方位
        if cur_type == "GENSEC" {
            apply_bang = false;
        }
        //土建特殊情况的一些处理
        let owner_is_gensec = owner_type == "GENSEC";
        let mut pos_extru_dir: Option<DVec3> = None;
        if owner_is_gensec {
            //找到spine，获取spine的两个顶点
            if let Ok(pts) = get_spline_pts(owner).await {
                if pts.len() == 2 {
                    pos_extru_dir = Some((pts[1] - pts[0]).normalize());
                }
            }
        } else if let Some(end) = att.get_dpose()
            && let Some(start) = att.get_dposs()
        {
            pos_extru_dir = Some((end - start).normalize());
            // dbg!(pos_extru_dir);
        }
        let is_sjoi = cur_type == "SJOI";
        let has_cut_dir = att.contains_key("CUTP");
        let cut_dir = att.get_dvec3("CUTP").unwrap_or(DVec3::Z);
        if is_sjoi {
            let cut_len = att.get_f64("CUTB").unwrap_or_default();
            // dbg!(&cut_dir);
            //先判断是否有cref
            //如果CUTP 没有z分量，则不考虑这些
            if let Some(c_ref) = att.get_foreign_refno("CREF")
                && let Ok(c_att) = get_named_attmap(c_ref).await
            {
                let jline = c_att.get_str("JLIN").map(|x| x.trim()).unwrap_or("NA");
                // dbg!(jline);
                if let Ok(Some(param)) = query_pline(c_ref, jline.into()).await {
                    let jlin_pos = param.pt;
                    let jlin_plax = param.plax;
                    // dbg!((&jlin_pos, &jlin_plax));
                    let c_t = Box::pin(get_world_transform(c_ref))
                        .await?
                        .unwrap_or_default();
                    let o_t = Box::pin(get_world_transform(o_att.get_owner()))
                        .await?
                        .unwrap_or_default();
                    let jlin_offset = c_t.rotation.as_dquat() * jlin_pos;
                    // dbg!(jlin_offset);
                    let c_axis = c_t.rotation.as_dquat() * DVec3::Z;
                    // dbg!(c_axis);
                    let c_wpos = c_t.translation.as_dvec3() + jlin_offset;
                    // dbg!(c_wpos);
                    // 是沿着附属的梁的轴方向再平移
                    let z_axis = o_t.rotation.as_dquat() * DVec3::Z;
                    // dbg!(z_axis);
                    // 取cref 对应构件的PLIN的位置
                    //如果垂直了，CUTP就是失效，不用考虑加冗余
                    let same_plane = c_axis.dot(cut_dir).abs() > 0.001;
                    if same_plane {
                        // dbg!(o_t.translation);
                        let delta = (c_wpos - o_t.translation.as_dvec3()).dot(z_axis);
                        // dbg!(delta);
                        translation = o_t.translation.as_dvec3() + delta * z_axis;
                        // dbg!(translation);
                        //如果 jlin_axis 和 z_axis 垂直
                        let perpendicular = z_axis.dot(c_axis).abs() < 0.001;
                        if !perpendicular {
                            translation += z_axis * cut_len;
                            // dbg!(translation);
                        }
                    }
                }
            } else {
            }
        }
        if att.contains_key("ZDIS") {
            if cur_type == "ENDATU" {
                //需要判断是第几个ENDATU
                let endatu_index: Option<u32> =
                    crate::get_index_by_noun_in_parent(owner, cur_refno, Some("ENDATU"))
                        .await
                        .unwrap();
                let section_end = if endatu_index == Some(0) {
                    Some(SectionEnd::START)
                } else if endatu_index == Some(1) {
                    Some(SectionEnd::END)
                } else {
                    None
                };
                // dbg!(&section_end);
                if let Some(result) = cal_zdis_pkdi_in_section_by_spine(
                    owner,
                    0.0,
                    att.get_f32("ZDIS").unwrap_or_default(),
                    section_end,
                )
                .await?
                {
                    pos += result.1;
                    quat = result.0;
                    // dbg!(math_tool::dquat_to_pdms_ori_xyz_str(&quat, true));
                    translation = translation + rotation * pos;
                    rotation = quat;
                    mat4 = DMat4::from_rotation_translation(rotation, translation);
                    continue;
                }
            } else {
                let zdist = att.get_f32("ZDIS").unwrap_or_default();
                let pkdi = att.get_f32("PKDI").unwrap_or_default();
                //zdis 起点应该是从poss 开始，所以这里需要加上这个偏移
                if let Some((tmp_quat, tmp_pos)) =
                    cal_zdis_pkdi_in_section_by_spine(owner, pkdi, zdist, None).await?
                {
                    // pos = result.1;
                    quat = tmp_quat;
                    // dbg!(math_tool::dquat_to_pdms_ori_xyz_str(&quat, true));
                    // dbg!(tmp_pos);
                    pos = tmp_pos;
                    // translation = translation + rotation * tmp_pos;
                    // dbg!(translation);
                    is_world_quat = true;
                    // rotation = quat;
                    // mat4 = DMat4::from_rotation_translation(rotation, translation);
                    // continue;
                } else {
                    translation += rotation * DVec3::Z * zdist as f64;
                    // dbg!(translation);
                }
            }
        }
        if att.contains_key("NPOS") {
            let npos = att.get_vec3("NPOS").unwrap_or_default();
            // dbg!(npos);
            pos += npos.as_dvec3();
            // dbg!(pos);
        }

        let quat_v = att.get_rotation();
        let has_local_ori = quat_v.is_some();
        let mut need_bangle = false;
        //特殊处理的类型
        if (!owner_is_gensec && has_local_ori) || (owner_is_gensec && cur_type == "TMPL") {
            quat = quat_v.unwrap_or_default();
        } else {
            if let Some(z_axis) = pos_extru_dir {
                need_bangle = true;
                if owner_is_gensec {
                    //todo 待测试特殊情况
                    if !is_world_quat {
                        if !z_axis.is_normalized() {
                            return Ok(None);
                        }
                        quat = cal_spine_ori_by_z_axis_ref_x(z_axis, true);
                    }
                } else {
                    if !z_axis.is_normalized() {
                        return Ok(None);
                    }
                    //跳过是owner sctn或者 WALL 的计算
                    quat = cal_ori_by_z_axis_ref_y(z_axis);
                    // dbg!(math_tool::dquat_to_pdms_ori_xyz_str(&quat, false));
                }
            }
        }

        //如果posl有，就不起用CUTB，相当于CUTB是一个手动对齐
        //直接在世界坐标系下求坐标，跳过局部求解
        //有 cref 的时候，需要保持方向和 cref 一致
        let ydir_axis = att.get_dvec3("YDIR");
        let pos_line = att.get_str("POSL").map(|x| x.trim()).unwrap_or_default();
        let delta_vec = att.get_dvec3("DELP").unwrap_or_default();
        let mut has_opdir = false;
        if let Some(opdir) = att.get_dvec3("OPDI").map(|x| x.normalize()) {
            quat = cal_ori_by_opdir(opdir);
            has_opdir = true;
            // dbg!(dquat_to_pdms_ori_xyz_str(&quat, true));
            if pos_line.is_empty() {
                pos += delta_vec;
            }
        }

        //todo fix 处理 posl的计算
        if !pos_line.is_empty() {
            // dbg!(&cur_type);
            //plin里的位置偏移
            let mut plin_pos = DVec3::ZERO;
            let mut pline_plax = DVec3::X;
            // POSL 的处理, 获得父节点的形集, 自身的形集处理，已经在profile里处理过
            let mut is_lmirror = false;
            let ancestor_refnos = crate::query_filter_ancestors(owner, &HAS_PLIN_TYPES).await?;
            if let Some(plin_owner) = ancestor_refnos.into_iter().next() {
                let target_own_att = crate::get_named_attmap(plin_owner)
                    .await
                    .unwrap_or_default();
                is_lmirror = target_own_att.get_bool("LMIRR").unwrap_or_default();
                let own_pos_line = target_own_att.get_str("JUSL").unwrap_or("NA");
                let own_pos_line = if own_pos_line.is_empty() {
                    "NA"
                } else {
                    own_pos_line
                };

                if let Ok(Some(param)) = crate::query_pline(plin_owner, pos_line.into()).await {
                    plin_pos = param.pt;
                    pline_plax = param.plax;
                    #[cfg(feature = "debug_spatial")]
                    {
                        dbg!(plin_owner);
                        dbg!(pos_line);
                        dbg!(&param);
                    }
                }
                if let Ok(Some(own_param)) =
                    crate::query_pline(plin_owner, own_pos_line.into()).await
                {
                    plin_pos -= own_param.pt;
                    #[cfg(feature = "debug_spatial")]
                    {
                        dbg!(own_pos_line);
                        dbg!(&own_param);
                    }
                }
                #[cfg(feature = "debug_spatial")]
                {
                    dbg!(&plin_pos);
                }
            }
            let z_axis = if is_lmirror { -pline_plax } else { pline_plax };
            let plin_pos = if is_lmirror { -plin_pos } else { plin_pos };
            let mut new_quat = {
                if cur_type == "FITT" {
                    //受到bang的影响，需要变换
                    //绕着z轴旋转
                    let y_axis = DQuat::from_axis_angle(z_axis, bangle.to_radians()) * DVec3::Z;
                    let x_axis = y_axis.cross(z_axis).normalize();
                    // dbg!((x_axis, y_axis, z_axis));
                    DQuat::from_mat3(&DMat3::from_cols(x_axis, y_axis, z_axis))
                } else if cur_type == "SCOJ" {
                    cal_ori_by_z_axis_ref_x(z_axis) * quat
                } else {
                    cal_ori_by_z_axis_ref_y(z_axis) * quat
                }
            };
            // dbg!(dquat_to_pdms_ori_xyz_str(&new_quat, true));
            //处理有YDIR的情况
            if let Some(v) = ydir_axis {
                new_quat = cal_ori_by_ydir(v.normalize(), z_axis);
            }
            if apply_bang {
                new_quat = new_quat * DQuat::from_rotation_z(bangle.to_radians());
            }
            // dbg!(dquat_to_pdms_ori_xyz_str(&new_quat, true));
            let offset = rotation * (pos + plin_pos) + rotation * new_quat * delta_vec;
            #[cfg(feature = "debug_spatial")]
            {
                dbg!(&pos);
                dbg!(&plin_pos);
                dbg!(&delta_vec);
                dbg!(offset);
            }
            translation += offset;
            rotation = rotation * new_quat;
            // dbg!(dquat_to_pdms_ori_xyz_str(&rotation, true));
        } else {
            if let Some(v) = ydir_axis {
                let z_axis = DVec3::X;
                // dbg!((v, z_axis));
                quat = cal_ori_by_ydir(v.normalize(), z_axis);
                // dbg!(dquat_to_pdms_ori_xyz_str(&quat, true));
            }
            if apply_bang {
                quat = quat * DQuat::from_rotation_z(bangle.to_radians());
            }
            if has_cut_dir && !has_opdir && !has_local_ori {
                // dbg!(cut_dir);
                let mat3 = DMat3::from_quat(rotation);
                // dbg!((mat3.z_axis, cut_dir));
                quat = cal_cutp_ori(mat3.z_axis, cut_dir);
                is_world_quat = true;
            }
            translation = translation + rotation * pos;
            if is_world_quat {
                rotation = quat;
            } else {
                rotation = rotation * quat;
            }
        }

        mat4 = DMat4::from_rotation_translation(rotation, translation);
    }

    if rotation.is_nan() || translation.is_nan() {
        return Ok(None);
    }

    if is_local {
        mat4 = prev_mat4.inverse() * mat4;
    }

    Ok(Some(mat4))
}

///查询形集PLIN的值，todo 需要做缓存优化
// #[cached]
/// 根据参考号和JUSL值查询形集PLIN的参数数据
///
/// # Arguments
/// * `refno` - 参考号
/// * `jusl` - JUSL值
///
/// # Returns
/// * `Ok(Some(PlinParamData))` - 查询成功返回PLIN参数数据
/// * `Ok(None)` - 未找到匹配的PLIN数据
/// * `Err` - 查询过程中发生错误
pub async fn query_pline(refno: RefnoEnum, jusl: String) -> anyhow::Result<Option<PlinParamData>> {
    let cat_att = crate::get_cat_attmap(refno).await.unwrap_or_default();
    let psref = cat_att
        .get_foreign_refno("PSTR")
        .unwrap_or(cat_att.get_foreign_refno("PTSS").unwrap_or_default());
    if !psref.is_valid() {
        return Ok(None);
    }
    let c_refnos = crate::get_children_refnos(psref).await.unwrap_or_default();
    // dbg!(&c_refnos);
    for c_refno in c_refnos {
        let a = crate::get_named_attmap(c_refno).await?;
        let Some(p_key) = a.get_as_string("PKEY") else {
            continue;
        };
        let param = PlinParam {
            vxy: [
                a.get_as_string("PX").unwrap_or("0".to_string()),
                a.get_as_string("PY").unwrap_or("0".to_string()),
            ],
            dxy: [
                a.get_as_string("DX").unwrap_or("0".to_string()),
                a.get_as_string("DY").unwrap_or("0".to_string()),
            ],
            plax: a.get_as_string("PLAX").unwrap_or("unset".to_string()),
        };
        let x = super::resolve_expression(&param.vxy[0], refno, false).await?;
        let y = super::resolve_expression(&param.vxy[1], refno, false).await?;
        let dx = super::resolve_expression(&param.dxy[0], refno, false).await?;
        let dy = super::resolve_expression(&param.dxy[1], refno, false).await?;
        let plax = parse_expr_to_dir(&param.plax)
            .unwrap_or(DVec3::Y)
            .normalize();
        let plin_data = PlinParamData {
            pt: DVec3::new(x, y, 0.0) + DVec3::new(dx, dy, 0.0) * plax,
            plax,
        };
        if p_key == jusl {
            return Ok(Some(plin_data));
        }
    }
    Ok(None)
}

#[derive(Debug)]
pub enum SectionEnd {
    START,
    END,
}

/// 计算ZDIS和PKDI, refno 是有这个SPLINE属性或者SCTN这种的参考号
pub async fn cal_zdis_pkdi_in_section_by_spine(
    refno: RefnoEnum,
    pkdi: f32,
    zdis: f32,
    section_end: Option<SectionEnd>,
) -> anyhow::Result<Option<(DQuat, DVec3)>> {
    let mut pos = DVec3::default();
    let mut quat = DQuat::IDENTITY;
    //默认只有一个
    let mut spline_paths = get_spline_path(refno).await?;
    if spline_paths.is_empty() {
        return Ok(None);
    }
    let spine_ydir = spline_paths[0].preferred_dir.as_dvec3();

    let mut sweep_paths = spline_paths[0].generate_paths().0;
    let lens: Vec<f32> = sweep_paths.iter().map(|x| x.length()).collect::<Vec<_>>();
    let total_len: f32 = lens.iter().sum();
    let world_mat4 = Box::pin(get_world_mat4(refno, false))
        .await?
        .unwrap_or_default();
    let (_, w_quat, _) = world_mat4.to_scale_rotation_translation();
    let mut tmp_dist = zdis as f64;
    let mut tmp_porp = pkdi.clamp(0.0, 1.0);
    let start_len = (total_len * tmp_porp) as f64;
    //pkdi 给了一个比例的距离
    tmp_dist += start_len;
    //后续要考虑反方向的情况
    let mut cur_len = 0.0;
    for (i, path) in sweep_paths.into_iter().enumerate() {
        tmp_dist -= cur_len;
        cur_len = lens[i] as f64;
        //在第一段范围内，或者是最后一段，就没有长度的限制
        if tmp_dist > cur_len || i == lens.len() - 1 {
            match path {
                SweepPath3D::Line(l) => {
                    let mut z_dir = get_spline_line_dir(refno)
                        .await
                        .unwrap_or_default()
                        .normalize_or_zero();
                    if z_dir.length() == 0.0 {
                        // z_dir = DVec3::Z;
                        // let mut y_dir = spine_ydir;
                        // if y_dir.normalize().dot(DVec3::Z).abs() > 0.999 {
                        //     y_dir = DVec3::X
                        // };
                        // let x_dir = y_dir.cross(z_dir).normalize();
                        // quat = DQuat::from_mat3(&DMat3::from_cols(x_dir, y_dir, z_dir));
                        quat = w_quat;
                    } else {
                        quat = cal_spine_ori(z_dir, spine_ydir);
                        z_dir = DMat3::from_quat(quat).z_axis;
                        quat = w_quat * quat;
                    }
                    // dbg!(dquat_to_pdms_ori_xyz_str(&quat, true));
                    let spine = &spline_paths[i];
                    match section_end {
                        Some(SectionEnd::START) => {
                            pos = spine.pt0.as_dvec3();
                        }
                        Some(SectionEnd::END) => {
                            pos = spine.pt1.as_dvec3();
                        }
                        _ => {
                            pos += z_dir * tmp_dist + spine.pt0.as_dvec3();
                        }
                    }
                    break;
                }
                SweepPath3D::SpineArc(arc) => {
                    //使用弧长去计算当前的点的位置
                    if arc.radius > LEN_TOL {
                        let arc_center = arc.center.as_dvec3();
                        let arc_radius = arc.radius as f64;
                        let v = (arc.start_pt.as_dvec3() - arc_center).normalize();
                        let mut start_angle = DVec3::X.angle_between(v);
                        if DVec3::X.cross(v).z < 0.0 {
                            start_angle = -start_angle;
                        }
                        let mut theta = (tmp_dist / arc_radius);
                        if arc.clock_wise {
                            theta = -theta;
                        }
                        theta = start_angle + theta;
                        pos = arc_center + arc_radius * DVec3::new(theta.cos(), theta.sin(), 0.0);
                        let y_axis = DVec3::Z;
                        let mut x_axis = (arc_center - pos).normalize();
                        if arc.clock_wise {
                            x_axis = -x_axis;
                        }
                        let z_axis = x_axis.cross(y_axis).normalize();
                        // dbg!((x_axis, y_axis, z_axis));
                        quat = DQuat::from_mat3(&DMat3::from_cols(x_axis, y_axis, z_axis));
                        // dbg!(dquat_to_pdms_ori_xyz_str(&quat));
                        quat = w_quat * quat;
                    }
                }
                _ => {}
            }
        }
    }
    Ok(Some((quat, pos)))
}

pub async fn get_spline_path(refno: RefnoEnum) -> anyhow::Result<Vec<Spine3D>> {
    let type_name = crate::get_type_name(refno).await?;
    // dbg!(&type_name);
    let mut paths = vec![];
    if type_name == "GENSEC" || type_name == "WALL" {
        let children_refs = crate::get_children_refnos(refno).await.unwrap_or_default();
        // dbg!(&children_refs);
        for &x in children_refs.iter() {
            let spine_att = crate::get_named_attmap(x).await?;
            // dbg!(&spine_att);
            if spine_att.get_type_str() != "SPINE" {
                continue;
            }
            let ch_atts = crate::get_children_named_attmaps(x)
                .await
                .unwrap_or_default();
            let len = ch_atts.len();
            if len < 1 {
                continue;
            }

            let mut i = 0;
            while i < ch_atts.len() - 1 {
                let att1 = &ch_atts[i];
                let t1 = att1.get_type_str();
                let att2 = &ch_atts[(i + 1) % len];
                let t2 = att2.get_type_str();
                if t1 == "POINSP" && t2 == "POINSP" {
                    paths.push(Spine3D {
                        refno: att1.get_refno().unwrap(),
                        pt0: att1.get_position().unwrap_or_default(),
                        pt1: att2.get_position().unwrap_or_default(),
                        curve_type: SpineCurveType::LINE,
                        preferred_dir: spine_att.get_vec3("YDIR").unwrap_or(Vec3::Z),
                        ..Default::default()
                    });
                    // dbg!(&paths);
                    i += 1;
                } else if t1 == "POINSP" && t2 == "CURVE" {
                    let att3 = &ch_atts[(i + 2) % len];
                    let pt0 = att1.get_position().unwrap_or_default();
                    let pt1 = att3.get_position().unwrap_or_default();
                    let mid_pt = att2.get_position().unwrap_or_default();
                    let cur_type_str = att2.get_str("CURTYP").unwrap_or("unset");
                    let curve_type = match cur_type_str {
                        "CENT" => SpineCurveType::CENT,
                        "THRU" => SpineCurveType::THRU,
                        _ => SpineCurveType::UNKNOWN,
                    };
                    paths.push(Spine3D {
                        refno: att2.get_refno().unwrap(),
                        pt0,
                        pt1,
                        thru_pt: mid_pt,
                        center_pt: mid_pt,
                        cond_pos: att2.get_vec3("CPOS").unwrap_or_default(),
                        curve_type,
                        preferred_dir: spine_att.get_vec3("YDIR").unwrap_or(Vec3::Z),
                        radius: att2.get_f32("RAD").unwrap_or_default(),
                    });
                    i += 2;
                }
            }
        }
    }

    // dbg!(&paths);

    Ok(paths)
}

///沿着 dir 方向找到最近的目标构件
pub async fn query_neareast_along_axis(
    refno: RefnoEnum,
    dir: Vec3,
    target_type: &str,
) -> anyhow::Result<Option<(RefnoEnum, f32)>> {
    let pos = get_world_transform(refno)
        .await?
        .unwrap_or_default()
        .translation;
    //不用 room 的方法查询一次，直接用射线去查找
    let ray = Ray::new(pos.into(), dir.into());
    // dbg!(&ray);
    let rtree = GLOBAL_AABB_TREE.read().await;
    let mut filter = HashSet::new();
    filter.insert(target_type.to_string());
    let nearest = rtree
        .query_nearest_by_ray(QueryRay::new(ray, filter, true))
        .await;

    nearest
}

pub async fn query_neareast_by_pos_dir(
    pos: Vec3,
    dir: Vec3,
    target_type: &str,
) -> anyhow::Result<Option<(RefnoEnum, f32)>> {
    // let pos = get_world_transform(refno).await?.unwrap_or_default().translation;
    //不用 room 的方法查询一次，直接用射线去查找
    let ray = Ray::new(pos.into(), dir.into());
    // dbg!(&ray);
    let rtree = GLOBAL_AABB_TREE.read().await;
    let mut filter = HashSet::new();
    filter.insert(target_type.to_string());
    let nearest = rtree
        .query_nearest_by_ray(QueryRay::new(ray, filter, true))
        .await;

    nearest
}

/// 查询指定节点的包围盒，需要遍历子节点的所有包围盒, 如果是含有负实体的，取父节点的包围盒
/// 负实体的邻居节点如果是正实体，可能也要考虑在内
/// 还有种情况就是图形平台的包围盒？是需要去查询所有子节点的包围盒的
pub async fn query_bbox(refno: RefnoEnum) -> anyhow::Result<Option<(RefnoEnum, f32)>> {
    //获得所有子节点的包围盒？
    //还是所有的包围盒的

    Ok(None)
}
