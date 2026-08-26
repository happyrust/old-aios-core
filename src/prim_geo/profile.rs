use std::collections::HashMap;
use std::default;
use std::f32::consts::{FRAC_PI_2, PI};

use crate::parsed_data::geo_params_data::{CateGeoParam, PdmsGeoParam};
use crate::parsed_data::{CateGeomsInfo, CateProfileParam};
use crate::pdms_types::*;
use crate::prim_geo::category::CateBrepShape;
use crate::prim_geo::spine::{Line3D, Spine3D, SpineCurveType, SweepPath3D};
use crate::prim_geo::{CateBrepShapeMap, SweepSolid};
use crate::shape::pdms_shape::BrepShapeTrait;
use crate::tool::dir_tool::parse_ori_str_to_quat;
use crate::tool::math_tool::{
    dquat_to_pdms_ori_xyz_str, quat_to_pdms_ori_str, to_pdms_ori_str, to_pdms_vec_str,
};
use crate::{RefU64, get_world_transform};
use anyhow::anyhow;
use bevy_transform::prelude::Transform;
use dashmap::{DashMap, DashSet};
use glam::{DMat4, DQuat, DVec3, Mat3, Quat, Vec3};
use std::vec::Vec;

fn spine_segment_mitres(
    path: &SweepPath3D,
    source: (Option<DVec3>, Option<DVec3>),
    local: (Option<DVec3>, Option<DVec3>),
) -> (Option<DVec3>, Option<DVec3>) {
    // Arc3D retains source-space center/start/axis and its instance frame is
    // applied after tessellation, so its cut planes must remain source-space as
    // well. Linear segments are trimmed before their placement and keep the
    // historical spine-local plane convention.
    if matches!(path, SweepPath3D::SpineArc(_)) {
        source
    } else {
        local
    }
}

pub async fn create_profile_geos(
    refno: RefnoEnum,
    geom_info: &CateGeomsInfo,
    brep_shapes_map: &CateBrepShapeMap,
) -> anyhow::Result<bool> {
    let geos = &geom_info.geometries;
    if geos.len() == 0 {
        return Ok(false);
    }
    let att = crate::get_named_attmap(refno).await?;
    let type_name = att.get_type_str();
    let mut plax = Vec3::Y;
    let mut extrude_dir = DVec3::Z;
    let mat = crate::get_world_mat4(refno, true)
        .await?
        .unwrap_or_default();
    // dbg!(&mat);
    let (_, rot, _) = mat.to_scale_rotation_translation();
    // dbg!(dquat_to_pdms_ori_xyz_str(&rot));
    let inv_quat = rot.inverse();
    // dbg!((refno, att.get_dvec3("DRNS"), att.get_dvec3("DRNE")));
    let mut drns = att
        .get_dvec3("DRNS")
        .map(|x| inv_quat.mul_vec3(x.normalize()));
    let mut drne = att
        .get_dvec3("DRNE")
        .map(|x| inv_quat.mul_vec3(x.normalize()));
    // dbg!((refno, drns, drne));
    let parent_refno = att.get_owner();
    let mut spine_mitres = HashMap::new();
    let mut spine_paths = if type_name == "GENSEC" || type_name == "WALL" {
        let children_refnos = crate::query_filter_children(refno, &["SPINE"])
            .await
            .unwrap_or_default();
        let mut paths = vec![];
        for &spine_refno in children_refnos.iter() {
            let mut spine_att = crate::get_named_attmap(spine_refno).await?;
            if spine_att.is_empty() {
                spine_att = crate::get_implicit_named_attmap(spine_refno, "SPINE").await?;
            }
            let spine_mat = crate::get_world_mat4(spine_refno, true)
                .await?
                .unwrap_or_default();
            let inv_mat = spine_mat.inverse();
            let source_drns = spine_att
                .get_dvec3("DRNS")
                .and_then(|normal| normal.try_normalize());
            let source_drne = spine_att
                .get_dvec3("DRNE")
                .and_then(|normal| normal.try_normalize());
            //如果是墙，会有这两个属性
            drns = spine_att
                .get_dvec3("DRNS")
                .map(|x| inv_mat.transform_vector3(x.normalize()));
            if drns.is_some() && drns.unwrap().is_nan() {
                drns = None;
            }
            drne = spine_att
                .get_dvec3("DRNE")
                .map(|x| inv_mat.transform_vector3(x.normalize()));
            if drne.is_some() && drne.unwrap().is_nan() {
                drne = None;
            }
            // dbg!((drns, drne));
            let ch_atts = crate::get_children_named_attmaps(spine_refno)
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
                    let segment_refno = att1.get_refno().unwrap();
                    spine_mitres.insert(segment_refno, ((source_drns, source_drne), (drns, drne)));
                    paths.push(Spine3D {
                        refno: segment_refno,
                        pt0: att1.get_position().unwrap_or_default(),
                        pt1: att2.get_position().unwrap_or_default(),
                        curve_type: SpineCurveType::LINE,
                        preferred_dir: spine_att.get_vec3("YDIR").unwrap_or(Vec3::Z),
                        ..Default::default()
                    });
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
                    let segment_refno = att2.get_refno().unwrap();
                    spine_mitres.insert(segment_refno, ((source_drns, source_drne), (drns, drne)));
                    paths.push(Spine3D {
                        refno: segment_refno,
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
        paths
    } else {
        vec![]
    };

    if spine_paths.len() == 0 {
        if let Some(poss) = att.get_poss()
            && let Some(pose) = att.get_pose()
        {
            let height = pose.distance(poss);
            //还原成相对坐标系下的拉升方向
            for (i, geom) in geos.iter().enumerate() {
                if let CateGeoParam::Profile(profile) = geom {
                    let Some(profile_refno) = profile.get_refno() else {
                        continue;
                    };
                    plax = profile.get_plax();
                    let bangle = att.get_f32("BANG").unwrap_or_default();

                    // POSS/POSE 是世界系坐标：path 要和上面的 DRNS/DRNE 一样过 inv_quat
                    // 拉回元素局部系（带 SPINE 的分支同口径——path 归一到局部，方向由
                    // 元素的 world_trans 携带）。世界系向量直接落进局部系字段，会让
                    // get_trans()/working_mitre_plane() 把方向再计一次。
                    let path = Line3D {
                        start: Default::default(),
                        end: inv_quat.mul_vec3((pose - poss).as_dvec3()).as_vec3(),
                        is_spine: false,
                    };

                    let solid = SweepSolid {
                        profile: profile.clone(),
                        drns,
                        drne,
                        bangle,
                        plax,
                        extrude_dir,
                        height,
                        path: SweepPath3D::Line(path),
                        lmirror: att.get_bool("LMIRR").unwrap_or_default(),
                    };
                    brep_shapes_map
                        .entry(refno)
                        .or_insert(Vec::new())
                        .push(CateBrepShape {
                            refno: profile_refno,
                            brep_shape: Box::new(solid),
                            transform: Transform::IDENTITY,
                            visible: true,
                            is_tubi: false,
                            shape_err: None,
                            pts: Default::default(),
                            is_ngmr: false,
                            is_instance_negative: false,
                        });
                }
            }
        }
    } else {
        for spine in spine_paths {
            for (i, geom) in geos.iter().enumerate() {
                if let CateGeoParam::Profile(profile) = geom {
                    plax = profile.get_plax();
                    let (paths, mut transform) = spine.generate_paths();
                    let (source_mitres, local_mitres) = spine_mitres
                        .get(&spine.refno)
                        .copied()
                        .unwrap_or(((drns, drne), (drns, drne)));
                    let bangle = att.get_f32("BANG").unwrap_or_default();
                    for path in paths {
                        let (segment_drns, segment_drne) =
                            spine_segment_mitres(&path, source_mitres, local_mitres);
                        let loft = SweepSolid {
                            profile: profile.clone(),
                            drns: segment_drns,
                            drne: segment_drne,
                            bangle,
                            plax,
                            extrude_dir,
                            height: 0.0,
                            path,
                            lmirror: att.get_bool("LMIRR").unwrap_or_default(),
                        };
                        transform.scale = loft.get_scaled_vec3();
                        let hash = profile
                            .get_refno()
                            .unwrap()
                            .hash_with_another_refno(spine.refno);
                        brep_shapes_map
                            .entry(refno)
                            .or_insert(Vec::new())
                            .push(CateBrepShape {
                                //这里需要混合在一起，可能有多个profile 和 多个 spine的点 生成的
                                refno: RefU64(hash).into(),
                                brep_shape: Box::new(loft),
                                transform,
                                visible: true,
                                is_tubi: false,
                                shape_err: None,
                                pts: Default::default(),
                                is_ngmr: false,
                                is_instance_negative: false,
                            });
                    }
                }
            }
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prim_geo::spine::{Arc3D, Line3D};

    #[test]
    fn arc_mitre_planes_keep_the_same_source_frame_as_arc_geometry() {
        let source = (Some(DVec3::X), Some(DVec3::NEG_X));
        let local = (Some(DVec3::Z), Some(DVec3::NEG_Z));
        let arc = SweepPath3D::SpineArc(Arc3D::default());
        let line = SweepPath3D::Line(Line3D::default());

        assert_eq!(spine_segment_mitres(&arc, source, local), source);
        assert_eq!(spine_segment_mitres(&line, source, local), local);
    }
}
