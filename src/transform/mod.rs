//! Transform module for calculating local and world transforms for entities
//!
//! This module provides functions to calculate transforms (local positions and orientations)
//! for different entity types. It extracts functionality from the `get_world_transform` method
//! to calculate only the local transform of each node relative to its parent, which can then
//! be combined to get the world transform without recalculating from the root each time.

use crate::rs_surreal::spatial::{
    SectionEnd, cal_cutp_ori, cal_ori_by_opdir, cal_ori_by_ydir, cal_ori_by_z_axis_ref_x,
    cal_ori_by_z_axis_ref_y, cal_spine_ori_by_z_axis_ref_x, cal_zdis_pkdi_in_section_by_spine,
    get_spline_pts, query_pline,
};
use crate::{
    NamedAttrMap, RefnoEnum, SUL_DB, get_named_attmap,
    pdms_data::{PlinParam, PlinParamData},
    tool::{direction_parse::parse_expr_to_dir, math_tool::*},
};
use anyhow::anyhow;
use bevy_transform::prelude::*;
use cached::proc_macro::cached;
use glam::{DMat3, DMat4, DQuat, DVec3};

/// Calculate the local transform for an entity relative to its parent
///
/// # Arguments
/// * `refno` - Reference number of the entity
/// * `parent_refno` - Reference number of the parent entity
///
/// # Returns
/// * `Ok(Some(Transform))` - The local transform if calculation succeeds
/// * `Ok(None)` - If the transform cannot be calculated
/// * `Err` - If an error occurs during calculation
#[cached(result = true)]
pub async fn get_local_transform(
    refno: RefnoEnum,
    parent_refno: RefnoEnum,
) -> anyhow::Result<Option<Transform>> {
    get_local_mat4(refno, parent_refno)
        .await
        .map(|m| m.map(|x| Transform::from_matrix(x.as_mat4())))
}

/// Calculate the local transformation matrix for an entity relative to its parent
///
/// # Arguments
/// * `refno` - Reference number of the entity
/// * `parent_refno` - Reference number of the parent entity
///
/// # Returns
/// * `Ok(Some(DMat4))` - The local transformation matrix if calculation succeeds
/// * `Ok(None)` - If the transform cannot be calculated
/// * `Err` - If an error occurs during calculation
#[cached(result = true)]
pub async fn get_local_mat4(
    refno: RefnoEnum,
    parent_refno: RefnoEnum,
) -> anyhow::Result<Option<DMat4>> {
    // Get attribute maps for the entity and its parent
    let att = get_named_attmap(refno).await?;
    let parent_att = get_named_attmap(parent_refno).await?;

    let cur_type = att.get_type_str();
    let parent_type = parent_att.get_type_str();

    let mut rotation = DQuat::IDENTITY;
    let mut translation = DVec3::ZERO;

    // Get position from attributes
    let mut pos = att.get_position().unwrap_or_default().as_dvec3();

    // Initialize quaternion and flags
    let mut quat = DQuat::IDENTITY;
    let mut is_world_quat = false;
    let mut bangle = att.get_f32("BANG").unwrap_or_default() as f64;
    let mut apply_bang = att.contains_key("BANG") && bangle != 0.0;

    // Special case for GENSEC
    if cur_type == "GENSEC" {
        apply_bang = false;
    }

    // Handle parent-specific transformations
    let parent_is_gensec = parent_type == "GENSEC";
    let mut pos_extru_dir: Option<DVec3> = None;

    if parent_is_gensec {
        // Find spine and get its vertices
        if let Ok(pts) = get_spline_pts(parent_refno).await {
            if pts.len() == 2 {
                pos_extru_dir = Some((pts[1] - pts[0]).normalize());
            }
        }
    } else if let Some(end) = att.get_dpose()
        && let Some(start) = att.get_dposs()
    {
        pos_extru_dir = Some((end - start).normalize());
    }

    // Handle SJOI type
    let is_sjoi = cur_type == "SJOI";
    let has_cut_dir = att.contains_key("CUTP");
    let cut_dir = att.get_dvec3("CUTP").unwrap_or(DVec3::Z);

    if is_sjoi {
        let cut_len = att.get_f64("CUTB").unwrap_or_default();

        if let Some(c_ref) = att.get_foreign_refno("CREF")
            && let Ok(c_att) = get_named_attmap(c_ref).await
        {
            let jline = c_att.get_str("JLIN").map(|x| x.trim()).unwrap_or("NA");

            if let Ok(Some(param)) = query_pline(c_ref, jline.into()).await {
                let jlin_pos = param.pt;
                let jlin_plax = param.plax;

                // Get the world transform for c_ref and parent_refno to calculate local transform
                let c_world = crate::rs_surreal::get_world_mat4(c_ref, false)
                    .await?
                    .unwrap_or(DMat4::IDENTITY);
                let parent_world = crate::rs_surreal::get_world_mat4(parent_refno, false)
                    .await?
                    .unwrap_or(DMat4::IDENTITY);
                let c_local_mat = parent_world.inverse() * c_world;
                let c_t = Transform::from_matrix(c_local_mat.as_mat4());

                let jlin_offset = c_t.rotation.as_dquat() * jlin_pos;
                let c_axis = c_t.rotation.as_dquat() * DVec3::Z;
                let c_wpos = c_t.translation.as_dvec3() + jlin_offset;

                // Calculate along the beam axis direction
                let z_axis = rotation * DVec3::Z;

                // Check if CUTP is effective
                let same_plane = c_axis.dot(cut_dir).abs() > 0.001;
                if same_plane {
                    let delta = (c_wpos - translation).dot(z_axis);
                    translation = translation + delta * z_axis;

                    // Check if perpendicular
                    let perpendicular = z_axis.dot(c_axis).abs() < 0.001;
                    if !perpendicular {
                        translation += z_axis * cut_len;
                    }
                }
            }
        }
    }

    // Handle ZDIS attribute
    if att.contains_key("ZDIS") {
        if cur_type == "ENDATU" {
            // Determine which ENDATU it is
            let endatu_index: Option<u32> =
                crate::get_index_by_noun_in_parent(parent_refno, refno, Some("ENDATU"))
                    .await
                    .unwrap();

            let section_end = if endatu_index == Some(0) {
                Some(SectionEnd::START)
            } else if endatu_index == Some(1) {
                Some(SectionEnd::END)
            } else {
                None
            };

            if let Some(result) = cal_zdis_pkdi_in_section_by_spine(
                parent_refno,
                0.0,
                att.get_f32("ZDIS").unwrap_or_default(),
                section_end,
            )
            .await?
            {
                pos += result.1;
                quat = result.0;
                translation = translation + rotation * pos;
                rotation = quat;
                return Ok(Some(DMat4::from_rotation_translation(
                    rotation,
                    translation,
                )));
            }
        } else {
            let zdist = att.get_f32("ZDIS").unwrap_or_default();
            let pkdi = att.get_f32("PKDI").unwrap_or_default();

            if let Some((tmp_quat, tmp_pos)) =
                cal_zdis_pkdi_in_section_by_spine(parent_refno, pkdi, zdist, None).await?
            {
                quat = tmp_quat;
                pos = tmp_pos;
                is_world_quat = true;
            } else {
                translation += rotation * DVec3::Z * zdist as f64;
            }
        }
    }

    // Handle NPOS attribute
    if att.contains_key("NPOS") {
        let npos = att.get_vec3("NPOS").unwrap_or_default();
        pos += npos.as_dvec3();
    }

    // Handle rotation
    let quat_v = att.get_rotation();
    let has_local_ori = quat_v.is_some();
    let mut need_bangle = false;

    // Special handling for different types
    if (!parent_is_gensec && has_local_ori) || (parent_is_gensec && cur_type == "TMPL") {
        quat = quat_v.unwrap_or_default();
    } else {
        if let Some(z_axis) = pos_extru_dir {
            need_bangle = true;
            if parent_is_gensec {
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
                quat = cal_ori_by_z_axis_ref_y(z_axis);
            }
        }
    }

    // Handle YDIR, POSL, DELP attributes
    let ydir_axis = att.get_dvec3("YDIR");
    let pos_line = att.get_str("POSL").map(|x| x.trim()).unwrap_or_default();
    let delta_vec = att.get_dvec3("DELP").unwrap_or_default();
    let mut has_opdir = false;

    if let Some(opdir) = att.get_dvec3("OPDI").map(|x| x.normalize()) {
        quat = cal_ori_by_opdir(opdir);
        has_opdir = true;
        if pos_line.is_empty() {
            pos += delta_vec;
        }
    }

    // Handle POSL (position line)
    if !pos_line.is_empty() {
        // Get PLIN position offset
        let mut plin_pos = DVec3::ZERO;
        let mut pline_plax = DVec3::X;
        let mut is_lmirror = false;

        let ancestor_refnos =
            crate::query_filter_ancestors(parent_refno, &crate::consts::HAS_PLIN_TYPES).await?;
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
            }

            if let Ok(Some(own_param)) = crate::query_pline(plin_owner, own_pos_line.into()).await {
                plin_pos -= own_param.pt;
            }
        }

        let z_axis = if is_lmirror { -pline_plax } else { pline_plax };
        let plin_pos = if is_lmirror { -plin_pos } else { plin_pos };

        let mut new_quat = {
            if cur_type == "FITT" {
                // Affected by bang, need transformation
                // Rotate around z-axis
                let y_axis = DQuat::from_axis_angle(z_axis, bangle.to_radians()) * DVec3::Z;
                let x_axis = y_axis.cross(z_axis).normalize();
                DQuat::from_mat3(&DMat3::from_cols(x_axis, y_axis, z_axis))
            } else if cur_type == "SCOJ" {
                cal_ori_by_z_axis_ref_x(z_axis) * quat
            } else {
                cal_ori_by_z_axis_ref_y(z_axis) * quat
            }
        };

        // Handle YDIR
        if let Some(v) = ydir_axis {
            new_quat = cal_ori_by_ydir(v.normalize(), z_axis);
        }

        // Apply BANG if needed
        if apply_bang {
            new_quat = new_quat * DQuat::from_rotation_z(bangle.to_radians());
        }

        let offset = rotation * (pos + plin_pos) + rotation * new_quat * delta_vec;
        translation += offset;
        rotation = rotation * new_quat;
    } else {
        // Handle YDIR without POSL
        if let Some(v) = ydir_axis {
            let z_axis = DVec3::X;
            quat = cal_ori_by_ydir(v.normalize(), z_axis);
        }

        // Apply BANG if needed
        if apply_bang {
            quat = quat * DQuat::from_rotation_z(bangle.to_radians());
        }

        // Handle CUTP
        if has_cut_dir && !has_opdir && !has_local_ori {
            let mat3 = DMat3::from_quat(rotation);
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

    // Create final transformation matrix
    let mat4 = DMat4::from_rotation_translation(rotation, translation);

    // Check for NaN values
    if rotation.is_nan() || translation.is_nan() {
        return Ok(None);
    }

    Ok(Some(mat4))
}

/// Calculate the world transform for an entity by combining local transforms
///
/// This function calculates the world transform by traversing the hierarchy
/// from the entity up to the root, calculating local transforms along the way,
/// and then combining them.
///
/// # Arguments
/// * `refno` - Reference number of the entity
///
/// # Returns
/// * `Ok(Some(Transform))` - The world transform if calculation succeeds
/// * `Ok(None)` - If the transform cannot be calculated
/// * `Err` - If an error occurs during calculation
#[cached(result = true)]
pub async fn get_world_transform(refno: RefnoEnum) -> anyhow::Result<Option<Transform>> {
    get_world_mat4(refno)
        .await
        .map(|m| m.map(|x| Transform::from_matrix(x.as_mat4())))
}

/// Calculate the world transformation matrix for an entity by combining local transforms
///
/// # Arguments
/// * `refno` - Reference number of the entity
///
/// # Returns
/// * `Ok(Some(DMat4))` - The world transformation matrix if calculation succeeds
/// * `Ok(None)` - If the transform cannot be calculated
/// * `Err` - If an error occurs during calculation
#[cached(result = true)]
pub async fn get_world_mat4(refno: RefnoEnum) -> anyhow::Result<Option<DMat4>> {
    // Get ancestors
    let ancestors: Vec<RefnoEnum> = crate::query_ancestor_refnos(refno).await?;

    if ancestors.len() <= 1 {
        return Ok(Some(DMat4::IDENTITY));
    }

    // Start with identity matrix
    let mut world_mat = DMat4::IDENTITY;

    // Traverse ancestors from root to leaf
    for i in 0..ancestors.len() - 1 {
        let parent_refno = ancestors[i];
        let child_refno = ancestors[i + 1];

        // Get local transform
        if let Some(local_mat) = get_local_mat4(child_refno, parent_refno).await? {
            // Combine with world transform
            world_mat = world_mat * local_mat;
        } else {
            return Ok(None);
        }
    }

    Ok(Some(world_mat))
}
