use crate::parsed_data::{CateProfileParam, SProfileData, SannData};
use crate::prim_geo::spine::*;
use crate::prim_geo::wire;
#[cfg(feature = "truck")]
use crate::shape::pdms_shape::{convert_to_cg_matrix4, BrepMathTrait};
use crate::shape::pdms_shape::{BrepShapeTrait, VerifiedShape, ANGLE_RAD_F64_TOL};
use crate::tool::math_tool::{quat_to_pdms_ori_str, to_pdms_ori_str};
use anyhow::anyhow;
use approx::{abs_diff_eq, abs_diff_ne};
use bevy_ecs::prelude::*;
use cavalier_contours::core::math::bulge_from_angle;
use cavalier_contours::polyline::{seg_midpoint, PlineSource, PlineSourceMut, Polyline};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use glam::*;

use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::prim_geo::wire::polyline_to_debug_json_str;
#[cfg(feature = "truck")]
use truck_base::cgmath64::*;

///含有两边方向的，扫描体
#[derive(
    Component,
    Debug,
    Clone,
    Serialize,
    Deserialize,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct SweepSolid {
    pub profile: CateProfileParam,
    pub drns: Option<DVec3>,
    pub drne: Option<DVec3>,
    pub bangle: f32,
    pub plax: Vec3,
    pub extrude_dir: DVec3,
    pub height: f32,
    pub path: SweepPath3D,
    pub lmirror: bool,
}

/// Core3D `setMitrePlanes` parallel check: `|DRN · tangent| < 1e-6`.
pub const MITRE_PARALLEL_EPS: f64 = 1e-6;

/// `do_solid_segments` 走哪条内核实体分支。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolidSegmentKind {
    Extrusion,
    Revolution,
    RuledSolid,
}

impl SweepSolid {
    /// Core3D `DB_Gensec::setMitrePlanes`：由端面法向与该段切向推导工作斜切平面。
    /// 不改元素上的 `DRNS`/`DRNE`。`None` 表示垂直或平行，斜切被抑制。
    /// 起点方切外法向 = −tangent，终点 = +tangent。零长切向且有 DRN 时闭合失败（保留工作平面）。
    pub fn set_mitre_planes(drn: Option<DVec3>, tangent: DVec3, is_start: bool) -> Option<DVec3> {
        let drn = drn?;
        let tan_len = tangent.length();
        if tan_len <= MITRE_PARALLEL_EPS {
            return Some(drn);
        }
        let tangent = tangent / tan_len;
        let drn_len = drn.length();
        if drn_len <= MITRE_PARALLEL_EPS {
            return Some(drn);
        }
        let drn_unit = drn / drn_len;
        let expected = if is_start { -tangent } else { tangent };
        if (drn_unit - expected).length() <= MITRE_PARALLEL_EPS {
            return None;
        }
        if drn_unit.dot(tangent).abs() <= MITRE_PARALLEL_EPS {
            return None;
        }
        Some(drn)
    }

    /// Core3D `setImpliedBangs`：BANG 是绕规范挤出轴（+Z）的实例旋转，不烤进截面。
    pub fn set_implied_bangs(bangle_deg: f32) -> Quat {
        Quat::from_axis_angle(Vec3::Z, bangle_deg.to_radians())
    }

    /// Core3D `setSpineSegmentTransforms`：长度 / PLAX / 镜像 / 路径切向 → 实例变换（不含 BANG）。
    pub fn set_spine_segment_transforms(
        length: f32,
        plax: Vec3,
        lmirror: bool,
        travel_tangent: DVec3,
    ) -> bevy_transform::prelude::Transform {
        let mut scale = Vec3::new(1.0, 1.0, length / 10.0);
        if lmirror {
            scale.x = -1.0;
        }
        let plax = if plax.length_squared() > 1e-12 {
            plax.normalize()
        } else {
            Vec3::Y
        };
        let plax_rot = Quat::from_rotation_arc(Vec3::Y, plax);
        let dir = travel_tangent.as_vec3();
        let dir = if dir.length_squared() > 1e-12 {
            dir.normalize()
        } else {
            Vec3::Z
        };
        let dir_rot = Quat::from_rotation_arc(Vec3::Z, dir);
        bevy_transform::prelude::Transform {
            rotation: dir_rot * plax_rot,
            scale,
            translation: Vec3::ZERO,
        }
    }

    /// 目录截面即单位几何（hash 只键截面时的输入）。
    pub fn set_cat_data(profile: CateProfileParam) -> Self {
        Self {
            profile,
            ..Default::default()
        }
    }

    /// Core3D `do_solid_segments`：无斜切直线 → 挤出；真斜切 → 放样；圆弧 → 回转。
    pub fn do_solid_segments(&self) -> SolidSegmentKind {
        match &self.path {
            SweepPath3D::SpineArc(_) => SolidSegmentKind::Revolution,
            SweepPath3D::Line(_) if self.is_sloped() => SolidSegmentKind::RuledSolid,
            SweepPath3D::Line(_) => SolidSegmentKind::Extrusion,
        }
    }

    pub fn travel_tangent(&self, is_start: bool) -> DVec3 {
        match &self.path {
            SweepPath3D::Line(line) => line.get_dir(true).as_dvec3(),
            SweepPath3D::SpineArc(arc) => {
                let radial = (arc.start_pt - arc.center).as_dvec3();
                let axis = if arc.axis.length_squared() > 1e-12 {
                    arc.axis.as_dvec3().normalize()
                } else {
                    DVec3::Z
                };
                let mut tangent = axis.cross(radial);
                if arc.clock_wise {
                    tangent = -tangent;
                }
                if !is_start {
                    let ang = if arc.clock_wise {
                        -(arc.angle as f64)
                    } else {
                        arc.angle as f64
                    };
                    tangent = DQuat::from_axis_angle(axis, ang) * tangent;
                }
                tangent
            }
        }
    }

    pub fn working_mitre_plane(&self, is_start: bool) -> Option<DVec3> {
        let drn = if is_start { self.drns } else { self.drne };
        Self::set_mitre_planes(drn, self.travel_tangent(is_start), is_start)
    }

    #[inline]
    pub fn is_sloped(&self) -> bool {
        self.is_drns_sloped() || self.is_drne_sloped()
    }

    #[inline]
    pub fn is_drns_sloped(&self) -> bool {
        self.working_mitre_plane(true).is_some()
    }

    #[inline]
    pub fn is_drne_sloped(&self) -> bool {
        self.working_mitre_plane(false).is_some()
    }

    //获得drns/drne的面的旋转矩阵
    pub fn get_face_mat4(&self, is_start: bool) -> DMat4 {
        let Some(dir) = self.working_mitre_plane(is_start) else {
            return DMat4::IDENTITY;
        };
        let mut angle_x = (dir.x / dir.z).atan();
        let mut angle_y = -(dir.y / dir.z).atan();
        //这里这个角度限制，应该用 h/2 / l 去计算，这里暂时给45°
        if angle_x.abs() - 0.01 >= FRAC_PI_2 || angle_y.abs() - 0.01 >= FRAC_PI_2 {
            return DMat4::IDENTITY;
        }
        let scale = DVec3::new(1.0 / angle_x.cos().abs(), 1.0 / angle_y.cos().abs(), 1.0);
        // dbg!((dir, angle_x.to_degrees(), angle_y.to_degrees(), scale));
        let rot =
            DQuat::from_axis_angle(DVec3::Y, angle_x) * DQuat::from_axis_angle(DVec3::X, angle_y);
        DMat4::from_scale_rotation_translation(scale, rot, DVec3::ZERO)
    }

    /// 生成sann的线框
    #[cfg(feature = "truck")]
    fn gen_sann_wire(
        &self,
        origin: Vec2,
        sann: &SannData,
        is_btm: bool,
        r1: f32,
        r2: f32,
    ) -> Option<truck_modeling::Wire> {
        #[cfg(feature = "truck")]
        use truck_modeling::{builder, Surface, Wire};

        let (r1, r2) = if is_btm {
            (r1, r2)
        } else {
            (r2 + sann.drad - sann.dwid - sann.pwidth, r2 + sann.drad)
        };
        // dbg!((r1, r2));
        let z_axis = Vec3::Z;
        let angle = sann.pangle.to_radians();
        // dbg!(angle);
        let mut offset_pt = Vec3::ZERO;
        let mut rot_mat = Mat3::IDENTITY;
        let mut beta_rot = Quat::IDENTITY;
        let mut r_translation = Vector3::new(0.0, 0.0, 0.0);
        offset_pt.x = -sann.plin_pos.x;
        offset_pt.y = -sann.plin_pos.y;
        match &self.path {
            SweepPath3D::SpineArc(d) => {
                let y_axis = d.pref_axis;
                let mut z_axis = self.plax;
                r_translation.x = d.radius as f64;
                if d.clock_wise {
                    z_axis = -z_axis;
                }
                if self.lmirror {
                    z_axis = -z_axis;
                }
                let x_axis = y_axis.cross(z_axis).normalize();
                rot_mat = Mat3::from_cols(x_axis, y_axis, z_axis);
                beta_rot = Quat::from_axis_angle(z_axis, self.bangle.to_radians());
                rot_mat = Mat3::from_quat(Quat::from_rotation_arc(self.plax, Vec3::Z));
            }

            SweepPath3D::Line(d) => {
                rot_mat = Mat3::from_quat(Quat::from_rotation_arc(self.plax, Vec3::Y));
                if d.is_spine {
                    dbg!(self.bangle.to_radians());
                    beta_rot = Quat::from_axis_angle(Vec3::Z, self.bangle.to_radians());
                }
            }
        }
        let p1 = Vec3::new(r1, 0.0, 0.0);
        let p2 = Vec3::new(r2, 0.0, 0.0);
        let p3 = Vec3::new(r2 * angle.cos(), r2 * angle.sin(), 0.0);
        let p4 = Vec3::new(r1 * angle.cos(), r1 * angle.sin(), 0.0);

        let v1 = builder::vertex(p1.point3());
        let v2 = builder::vertex(p2.point3());
        let v3 = builder::vertex(p3.point3());
        let v4 = builder::vertex(p4.point3());
        let center_pt = Point3::new(0.0, 0.0, 0.0);
        let wire = Wire::from(vec![
            builder::line(&v1, &v2),
            builder::circle_arc_with_center(
                center_pt,
                &v2,
                &v3,
                z_axis.vector3(),
                Rad(angle as f64),
            ),
            builder::line(&v3, &v4),
            builder::circle_arc_with_center(
                center_pt,
                &v4,
                &v1,
                -z_axis.vector3(),
                Rad(angle as f64),
            ),
        ]);
        let offset = offset_pt + Vec3::new(origin.x, origin.y, 0.0);
        let translation = Matrix4::from_translation(offset.vector3());
        let r_trans_mat = Matrix4::from_translation(r_translation);
        let m = &rot_mat;
        let local_mat = Matrix4::from_cols(
            m.x_axis.vector4(),
            m.y_axis.vector4(),
            m.z_axis.vector4(),
            Vector4::new(0.0, 0.0, 0.0, 1.0),
        );
        let m = Mat3::from_quat(beta_rot);
        let beta_mat = Matrix4::from_cols(
            m.x_axis.vector4(),
            m.y_axis.vector4(),
            m.z_axis.vector4(),
            Vector4::new(0.0, 0.0, 0.0, 1.0),
        );
        let mut result_wire =
            builder::transformed(&wire, r_trans_mat * beta_mat * local_mat * translation);
        let face = builder::try_attach_plane(&[result_wire.clone()]).ok()?;
        if let Surface::Plane(plane) = face.surface() {
            let _s = self.plax.y as f64;
            // if is_btm && plane.normal().dot(self.extrude_dir.vector3()) > 0.0 {
            //     result_wire.invert();
            // }
        }
        Some(result_wire)
    }

    ///计算SPRO的face
    /// start_vec 为起始方向
    #[cfg(feature = "truck")]
    fn cal_spro_wire(&self, profile: &SProfileData) -> Option<truck_modeling::Wire> {
        #[cfg(feature = "truck")]
        use truck_meshalgo::prelude::*;
        #[cfg(feature = "truck")]
        use truck_modeling::{builder, Surface};

        let verts = &profile.verts;
        let len = verts.len();

        let mut offset_pt = Vec3::ZERO;
        let mut rot_mat = Mat3::IDENTITY;
        let mut beta_rot = Quat::IDENTITY;
        let mut r_translation = Vector3::new(0.0, 0.0, 0.0);
        let plin_pos = profile.plin_pos;
        // dbg!(&profile);
        offset_pt.x = -plin_pos.x;
        offset_pt.y = -plin_pos.y;
        match &self.path {
            SweepPath3D::SpineArc(d) => {
                let y_axis = d.pref_axis;
                let mut z_axis = self.plax;
                r_translation.x = d.radius as f64;
                if d.clock_wise {
                    z_axis = -z_axis;
                }
                if self.lmirror {
                    z_axis = -z_axis;
                }
                let x_axis = y_axis.cross(z_axis).normalize();
                //旋转到期望的平面
                rot_mat = Mat3::from_cols(x_axis, y_axis, z_axis);
            }
            SweepPath3D::Line(d) => {
                rot_mat = Mat3::from_quat(Quat::from_rotation_arc(self.plax, Vec3::Y));
                // dbg!(rot_mat);
                // dbg!(to_pdms_ori_str(&rot_mat));
            }
        }

        // dbg!(&offset_pt);
        let mut points = vec![];
        for i in 0..len {
            // let p = Vec3::new(verts[i][0], verts[i][1], 0.0);
            let p = verts[i].extend(0.0);
            points.push(p);
        }
        let wire = wire::gen_wire(&points, &profile.frads).ok()?;
        // dbg!(self.bangle);
        let translation = Matrix4::from_translation(offset_pt.vector3());
        // dbg!(translation);
        let r_trans_mat = Matrix4::from_translation(r_translation);
        let m = &rot_mat;
        let local_mat = Matrix4::from_cols(
            m.x_axis.vector4(),
            m.y_axis.vector4(),
            m.z_axis.vector4(),
            Vector4::new(0.0, 0.0, 0.0, 1.0),
        );
        let m = Mat3::from_quat(beta_rot);
        let final_mat = r_trans_mat * local_mat * translation;
        // dbg!(&wire);
        let mut result_wire = builder::transformed(&wire, final_mat);
        // dbg!(result_wire.vertex_iter().collect::<Vec<_>>());
        let face = builder::try_attach_plane(&[result_wire.clone()]).ok()?;
        if let Surface::Plane(plane) = face.surface() {
            // let _s = self.plax.y as f64;
            // if plane.normal().dot(self.extrude_dir.vector3()) > 0.0 {
            //     result_wire.invert();
            // }
        }
        Some(result_wire)
    }
}

impl Default for SweepSolid {
    fn default() -> Self {
        Self {
            profile: CateProfileParam::UNKOWN,
            drns: None,
            drne: None,
            bangle: 0.0,
            plax: Vec3::Y,
            extrude_dir: DVec3::Z,
            height: 0.0,
            path: SweepPath3D::default(),
            lmirror: false,
        }
    }
}

impl VerifiedShape for SweepSolid {
    fn check_valid(&self) -> bool {
        !self.extrude_dir.is_nan() && self.extrude_dir.length() > 0.0
    }
}

impl BrepShapeTrait for SweepSolid {
    fn is_reuse_unit(&self) -> bool {
        matches!(&self.path, SweepPath3D::Line(_)) && !self.is_sloped()
    }

    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    #[cfg(feature = "truck")]
    fn gen_brep_shell(&self) -> Option<truck_modeling::Shell> {
        #[cfg(feature = "truck")]
        use truck_base::cgmath64::Point3;
        #[cfg(feature = "truck")]
        use truck_modeling::*;
        let mut profile_wire = None;
        let mut top_profile_wire = None;
        let mut is_sann = false;
        let (profile_wire, _top_profile_wire) = match &self.profile {
            CateProfileParam::SANN(p) => {
                let w = p.pwidth;
                let r = p.pradius;
                let r1 = r - w;
                let r2 = r;
                let origin = p.xy + p.dxy;
                profile_wire = self.gen_sann_wire(origin, p, true, r1, r2);
                top_profile_wire = self.gen_sann_wire(origin, p, false, r1, r2);
                is_sann = true;
                (profile_wire, top_profile_wire)
            }
            CateProfileParam::SPRO(p) => {
                let wire = self.cal_spro_wire(p);
                (wire, None)
            }
            CateProfileParam::SREC(p) => {
                let profile = p.convert_to_spro();
                let wire = self.cal_spro_wire(&profile);
                (wire, None)
            }
            _ => (None, None),
        };
        // if let Some(mut wire) = profile_wire && let Some(mut top_wire) = top_profile_wire {
        if let Some(wire) = profile_wire {
            //check if valid
            if self.drns.is_nan() || self.drne.is_nan() {
                // return Err(anyhow!("drns or drne is nan"));
                println!("drns or drne is nan");
                return None;
            }
            match &self.path {
                SweepPath3D::SpineArc(arc) => {
                    let mut face_s = builder::try_attach_plane(&[wire]).unwrap();
                    if let Surface::Plane(plane) = face_s.surface() {
                        let is_rev_face = (plane.normal().y * arc.axis.z as f64) < 0.0;
                        if is_rev_face {
                            dbg!("Face inveted");
                            face_s.invert();
                        }
                    }
                    let rot_angle = arc.angle;
                    let rot_axis = if arc.clock_wise { -Vec3::Z } else { Vec3::Z };
                    let solid = builder::rsweep(
                        &face_s,
                        Point3::origin(),
                        rot_axis.vector3(),
                        Rad(rot_angle as f64),
                    );
                    let shell: Shell = solid.into_boundaries().pop()?;
                    return Some(shell);
                }
                SweepPath3D::Line(l) => {
                    let mut transform_btm = Mat4::IDENTITY;
                    let mut transform_top = Mat4::IDENTITY;
                    if self.drns.is_normalized() && self.is_drns_sloped() {
                        let x_angle = self.drns.angle_between(Vec3::X).abs();
                        let scale_x = if x_angle < ANGLE_RAD_F64_TOL {
                            1.0
                        } else {
                            1.0 / (x_angle.sin())
                        };
                        let y_angle = self.drns.angle_between(Vec3::Y).abs();
                        let scale_y = if y_angle < ANGLE_RAD_F64_TOL {
                            1.0
                        } else {
                            1.0 / (y_angle.sin())
                        };
                        transform_btm =
                            Mat4::from_quat(glam::Quat::from_rotation_arc(Vec3::Z, self.drns))
                                * Mat4::from_scale(Vec3::new(scale_x, scale_y, 1.0));
                    }
                    if self.drne.is_normalized() && self.is_drne_sloped() {
                        let x_angle = (-self.drne).angle_between(Vec3::X).abs();
                        let scale_x = if x_angle < ANGLE_RAD_F64_TOL {
                            1.0
                        } else {
                            1.0 / (x_angle.sin())
                        };
                        let y_angle = (-self.drne).angle_between(DVec3::Y).abs();
                        let scale_y = if y_angle < ANGLE_RAD_F64_TOL {
                            1.0
                        } else {
                            1.0 / (y_angle.sin())
                        };
                        transform_top =
                            Mat4::from_quat(glam::Quat::from_rotation_arc(Vec3::Z, -self.drne))
                                * Mat4::from_scale(Vec3::new(scale_x, scale_y, 1.0));
                    }
                    transform_top =
                        Mat4::from_translation(Vec3::new(0.0, 0.0, l.length())) * transform_top;

                    let mut faces = vec![];
                    let wire_s = builder::transformed(&wire, convert_to_cg_matrix4(&transform_btm));
                    let wire_e = builder::transformed(&wire, convert_to_cg_matrix4(&transform_top));
                    let edges_cnt = wire_s.len();
                    for i in 0..edges_cnt {
                        let c1 = &wire_s[i];
                        let c2 = &wire_e[i];
                        faces.push(builder::homotopy(c1, c2).inverse());
                    }
                    let face_s = builder::try_attach_plane(&[wire_s]).ok()?;
                    let face_e = builder::try_attach_plane(&[wire_e]).ok()?;
                    faces.push(face_s);
                    faces.push(face_e.inverse());
                    let shell: Shell = faces.into();
                    return Some(shell);
                }
            }
        }
        None
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        //截面暂时用这个最省力的方法
        let mut hasher = DefaultHasher::default();
        let bytes = if self.is_drns_sloped() || self.is_drne_sloped() {
            bincode::serialize(&self).unwrap()
        } else if let SweepPath3D::SpineArc(_) = self.path {
            bincode::serialize(&self).unwrap()
        } else {
            bincode::serialize(&self.profile).unwrap()
        };
        bytes.hash(&mut hasher);
        "loft".hash(&mut hasher);

        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        let mut unit = self.clone();
        if let SweepPath3D::Line(_) = unit.path
            && !self.is_sloped()
        {
            // `hash_unit_mesh_params` deliberately identifies every reusable,
            // non-sloped linear sweep by its profile alone.  The persisted unit
            // parameter must obey the same identity contract: instance-only
            // orientation, end-face and path fields may not leak into the
            // canonical `inst_geo` row for that hash.
            unit.drns = None;
            unit.drne = None;
            unit.bangle = 0.0;
            unit.plax = Vec3::Y;
            unit.extrude_dir = DVec3::Z;
            unit.height = 0.0;
            unit.path = SweepPath3D::Line(Line3D {
                start: Default::default(),
                end: Vec3::Z * 10.0,
                is_spine: false,
            });
            unit.lmirror = false;
        }
        Box::new(unit)
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        if self.is_sloped() {
            return Vec3::ONE;
        }
        match &self.path {
            SweepPath3D::Line(l) => Vec3::new(1.0, 1.0, l.length() / 10.0),
            _ => Vec3::ONE,
        }
    }

    #[inline]
    fn get_trans(&self) -> bevy_transform::prelude::Transform {
        if !self.is_reuse_unit() {
            return bevy_transform::prelude::Transform::IDENTITY;
        }
        let length = match &self.path {
            SweepPath3D::Line(line) => line.length(),
            SweepPath3D::SpineArc(_) => 10.0,
        };
        let spine = Self::set_spine_segment_transforms(
            length,
            self.plax,
            self.lmirror,
            self.travel_tangent(true),
        );
        let bang = Self::set_implied_bangs(self.bangle);
        bevy_transform::prelude::Transform {
            rotation: spine.rotation * bang,
            scale: spine.scale,
            translation: Vec3::ZERO,
        }
    }

    fn tol(&self) -> f32 {
        if let Some(aabb) = self.profile.get_bbox() {
            return 0.01 * aabb.bounding_sphere().radius.max(1.0);
        }
        0.01
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimLoft(self.clone()))
    }
}

fn cal_end_face_rot(current_rot: DQuat, extru_dir: DVec3, face_dir: Option<DVec3>) -> DMat4 {
    let mut mat = DMat4::IDENTITY;
    if let Some(mut fd) = face_dir {
        let dir = current_rot.mul_vec3(extru_dir);
        //求两者之间的夹角，如果是负数，就是反方向
        let angle = dir.angle_between(fd);
        //如果超过90度，就是反方向
        if angle.abs() > std::f32::consts::FRAC_PI_2 as _ {
            fd = -fd;
        }
        // dbg!(angle);
        let dir_x = DVec3::new(dir.x, 0.0, dir.z).normalize();
        let fd_x = DVec3::new(fd.x, 0.0, fd.z).normalize();
        let angle_x = dir_x.angle_between(fd_x);
        let scale_x = 1.0 / angle_x.cos();

        let dir_y = DVec3::new(0.0, dir.y, dir.z).normalize();
        let fd_y = DVec3::new(0.0, fd.y, fd.z).normalize();
        let angle_y = dir_y.angle_between(fd_y);
        let scale_y = 1.0 / angle_y.cos();

        mat = DMat4::from_scale_rotation_translation(
            DVec3::new(scale_x, scale_y, 1.0),
            DQuat::IDENTITY,
            DVec3::ZERO,
        );
    }
    mat
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsed_data::SRectData;

    const MITRE_EPS: f64 = 1e-6;

    fn assert_no_mitre(drn: Option<DVec3>, tangent: DVec3, is_start: bool) {
        let before = drn;
        let plane = SweepSolid::set_mitre_planes(drn, tangent, is_start);
        assert_eq!(
            plane, None,
            "tangent={tangent:?} drn={drn:?} start={is_start}"
        );
        assert_eq!(
            drn, before,
            "set_mitre_planes must not be thought of as mutating the input"
        );
    }

    #[test]
    fn drns_perp_no_mitre() {
        assert_no_mitre(Some(DVec3::NEG_Z), DVec3::Z, true);
        assert_no_mitre(None, DVec3::Z, true);
    }

    #[test]
    fn drne_perp_no_mitre() {
        assert_no_mitre(Some(DVec3::Z), DVec3::Z, false);
        assert_no_mitre(None, DVec3::Z, false);
    }

    #[test]
    fn drns_parallel_suppress_mitre() {
        assert_no_mitre(Some(DVec3::X), DVec3::Z, true);
        assert_no_mitre(Some(DVec3::NEG_Y), DVec3::Z, false);
    }

    #[test]
    fn true_mitre_kept() {
        let drn = DVec3::new(0.3, 0.0, -0.953939).normalize();
        let plane = SweepSolid::set_mitre_planes(Some(drn), DVec3::Z, true);
        assert!(
            plane.is_some(),
            "skewed DRNS against +Z must keep a working plane"
        );
        let plane = plane.unwrap();
        assert!(
            (plane.normalize() - drn).length() < MITRE_EPS,
            "working plane keeps the attribute direction"
        );
    }

    #[test]
    fn perp_to_non_z_tangent() {
        assert_no_mitre(Some(DVec3::NEG_X), DVec3::X, true);
        assert_no_mitre(Some(DVec3::X), DVec3::X, false);
    }

    #[test]
    fn set_mitre_planes_does_not_write_attributes() {
        let mut solid = SweepSolid {
            drns: Some(DVec3::X),
            drne: Some(DVec3::Y),
            path: SweepPath3D::Line(Line3D {
                start: Vec3::ZERO,
                end: Vec3::Z * 4.0,
                is_spine: false,
            }),
            ..Default::default()
        };
        let _ = SweepSolid::set_mitre_planes(solid.drns, DVec3::Z, true);
        let _ = SweepSolid::set_mitre_planes(solid.drne, DVec3::Z, false);
        assert_eq!(solid.drns, Some(DVec3::X));
        assert_eq!(solid.drne, Some(DVec3::Y));
        solid.drns = Some(DVec3::NEG_Z);
        assert_eq!(solid.drns, Some(DVec3::NEG_Z));
    }

    fn reusable_line() -> SweepSolid {
        SweepSolid {
            profile: CateProfileParam::UNKOWN,
            path: SweepPath3D::Line(Line3D {
                start: Vec3::ZERO,
                end: Vec3::Z * 4.0,
                is_spine: true,
            }),
            ..Default::default()
        }
    }

    fn unit(solid: &SweepSolid) -> SweepSolid {
        *solid
            .gen_unit_shape()
            .downcast::<SweepSolid>()
            .expect("SweepSolid unit shape keeps its concrete type")
    }

    fn assert_canonical_envelope(solid: &SweepSolid) {
        let canonical = unit(solid);
        assert_eq!(canonical.drns, None);
        assert_eq!(canonical.drne, None);
        assert_eq!(canonical.bangle, 0.0);
        assert_eq!(canonical.plax, Vec3::Y);
        assert_eq!(canonical.extrude_dir, DVec3::Z);
        assert_eq!(canonical.height, 0.0);
        assert!(!canonical.lmirror);
        let SweepPath3D::Line(line) = canonical.path else {
            panic!("reusable linear unit shape must stay linear");
        };
        assert_eq!(line.start, Vec3::ZERO);
        assert_eq!(line.end, Vec3::Z * 10.0);
        assert!(!line.is_spine);
    }

    #[test]
    fn reusable_linear_aliases_share_hash_and_canonical_unit_shape() {
        let left = reusable_line();
        let mut right = left.clone();
        right.drns = Some(DVec3::NEG_Z);
        right.drne = Some(DVec3::Z);
        right.bangle = 37.0;
        right.plax = Vec3::X;
        right.extrude_dir = DVec3::X;
        right.height = 42.0;
        right.lmirror = true;
        right.path = SweepPath3D::Line(Line3D {
            start: Vec3::ZERO,
            end: Vec3::Z * 11.0,
            is_spine: false,
        });

        assert!(!left.is_sloped());
        assert!(!right.is_sloped());
        assert_eq!(left.hash_unit_mesh_params(), right.hash_unit_mesh_params());
        assert_eq!(
            bincode::serialize(&unit(&left)).unwrap(),
            bincode::serialize(&unit(&right)).unwrap()
        );
        assert_canonical_envelope(&left);
        assert_canonical_envelope(&right);
    }

    #[test]
    fn perp_to_non_z_tangent_shares_canonical_unit() {
        let z_path = reusable_line();
        let mut x_path = reusable_line();
        x_path.path = SweepPath3D::Line(Line3D {
            start: Vec3::ZERO,
            end: Vec3::X * 7.0,
            is_spine: false,
        });
        x_path.drns = Some(DVec3::NEG_X);
        x_path.drne = Some(DVec3::X);

        assert!(
            !x_path.is_sloped(),
            "square-cut against +X must not count as mitre"
        );
        assert_eq!(
            z_path.hash_unit_mesh_params(),
            x_path.hash_unit_mesh_params()
        );
        assert_eq!(
            bincode::serialize(&unit(&z_path)).unwrap(),
            bincode::serialize(&unit(&x_path)).unwrap()
        );
        assert_canonical_envelope(&x_path);
        assert_eq!(x_path.do_solid_segments(), SolidSegmentKind::Extrusion);
    }

    #[test]
    fn parallel_drn_still_reuses_profile_hash() {
        let left = reusable_line();
        let mut parallel = left.clone();
        parallel.drns = Some(DVec3::X);
        parallel.drne = Some(DVec3::NEG_Y);

        assert!(!parallel.is_sloped());
        assert_eq!(
            left.hash_unit_mesh_params(),
            parallel.hash_unit_mesh_params()
        );
        assert_canonical_envelope(&parallel);
        assert_eq!(parallel.do_solid_segments(), SolidSegmentKind::Extrusion);
    }

    #[test]
    fn true_mitre_is_not_canonicalized() {
        let mut solid = reusable_line();
        solid.drns = Some(DVec3::new(0.3, 0.0, -0.953939).normalize());

        assert!(solid.is_sloped());
        assert_eq!(solid.do_solid_segments(), SolidSegmentKind::RuledSolid);
        assert_ne!(
            solid.hash_unit_mesh_params(),
            reusable_line().hash_unit_mesh_params()
        );
        let stored = unit(&solid);
        assert_eq!(stored.drns, solid.drns);
        let SweepPath3D::Line(line) = stored.path else {
            panic!("true mitre stays linear");
        };
        assert_eq!(line.end, Vec3::Z * 4.0);
    }

    #[test]
    fn spine_arc_is_not_canonicalized() {
        let mut solid = reusable_line();
        solid.path = SweepPath3D::SpineArc(Arc3D {
            center: Vec3::ZERO,
            radius: 10.0,
            angle: FRAC_PI_2 as f32,
            start_pt: Vec3::X * 10.0,
            clock_wise: false,
            axis: Vec3::Z,
            pref_axis: Vec3::Y,
        });

        assert_eq!(solid.do_solid_segments(), SolidSegmentKind::Revolution);
        assert_ne!(
            solid.hash_unit_mesh_params(),
            reusable_line().hash_unit_mesh_params()
        );
        let stored = unit(&solid);
        assert!(matches!(stored.path, SweepPath3D::SpineArc(_)));
    }

    #[test]
    fn reusable_linear_hash_still_distinguishes_profiles() {
        let left = reusable_line();
        let mut right = left.clone();
        right.profile = CateProfileParam::SREC(SRectData {
            size: Vec2::new(2.0, 3.0),
            ..Default::default()
        });

        assert_ne!(left.hash_unit_mesh_params(), right.hash_unit_mesh_params());
    }

    #[test]
    fn set_implied_bangs_rotates_instance_not_unit() {
        let a = reusable_line();
        let mut b = a.clone();
        b.bangle = 37.0;

        assert_eq!(a.hash_unit_mesh_params(), b.hash_unit_mesh_params());
        assert_eq!(
            bincode::serialize(&unit(&a)).unwrap(),
            bincode::serialize(&unit(&b)).unwrap()
        );
        assert_eq!(a.get_trans().rotation, Quat::IDENTITY);
        assert_eq!(b.get_trans().rotation, SweepSolid::set_implied_bangs(37.0));
        assert_eq!(unit(&b).bangle, 0.0);
    }

    #[test]
    fn set_spine_segment_transforms_scale_plax_and_mirror() {
        let mut solid = reusable_line();
        solid.path = SweepPath3D::Line(Line3D {
            start: Vec3::ZERO,
            end: Vec3::Z * 20.0,
            is_spine: true,
        });
        assert!(
            (solid.get_trans().scale.z - 2.0).abs() < 1e-5,
            "length 20 against dummy 10 → scale.z = 2"
        );

        solid.lmirror = true;
        assert!(solid.get_trans().scale.x < 0.0);
        assert_eq!(
            solid.hash_unit_mesh_params(),
            reusable_line().hash_unit_mesh_params()
        );

        solid.lmirror = false;
        solid.plax = Vec3::X;
        let trans = solid.get_trans();
        let expected_plax = SweepSolid::set_spine_segment_transforms(20.0, Vec3::X, false, DVec3::Z);
        assert_eq!(trans.rotation, expected_plax.rotation);
        assert_eq!(unit(&solid).plax, Vec3::Y);
        assert_eq!(unit(&solid).extrude_dir, DVec3::Z);
    }

    #[test]
    fn world_path_direction_lives_in_instance_rotation() {
        let mut solid = reusable_line();
        solid.path = SweepPath3D::Line(Line3D {
            start: Vec3::ZERO,
            end: Vec3::X * 10.0,
            is_spine: false,
        });
        solid.drns = Some(DVec3::NEG_X);
        solid.drne = Some(DVec3::X);

        assert!(!solid.is_sloped());
        let trans = solid.get_trans();
        let rotated_z = trans.rotation * Vec3::Z;
        assert!(
            (rotated_z - Vec3::X).length() < 1e-5,
            "instance rotation must send canonical +Z along the path, got {rotated_z:?}"
        );
        assert_eq!(unit(&solid).extrude_dir, DVec3::Z);
        let SweepPath3D::Line(line) = unit(&solid).path else {
            panic!("canonical envelope stays linear");
        };
        assert_eq!(line.start, Vec3::ZERO);
        assert_eq!(line.end, Vec3::Z * 10.0);
        assert!(!line.is_spine);
    }
}
