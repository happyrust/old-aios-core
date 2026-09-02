use crate::NamedAttrMap;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::prim_geo::helper::RotateInfo;
use crate::prim_geo::libgm_discretise::{
    FACET_TOL_MM, circular_torus_tube_segments, torus_ring_segments,
};
use crate::shape::pdms_shape::{BrepShapeTrait, PlantMesh, RsVec3, TRI_TOL, VerifiedShape};
use crate::tool::float_tool::hash_f32;
use crate::types::attmap::AttrMap;
use bevy_ecs::prelude::*;
use bevy_transform::prelude::Transform;
use glam::{DVec2, DVec3, Quat, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

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
pub struct SCTorus {
    pub paax_pt: Vec3,
    //A Axis point
    pub paax_dir: Vec3, //A Axis Direction

    pub pbax_pt: Vec3,
    //B Axis point
    pub pbax_dir: Vec3, //B Axis Direction

    pub pdia: f32,
}

impl SCTorus {
    pub fn convert_to_ctorus(&self) -> Option<(CTorus, Transform)> {
        if let Some(torus_info) = RotateInfo::cal_rotate_info(
            self.paax_dir,
            self.paax_pt,
            self.pbax_dir,
            self.pbax_pt,
            self.pdia / 2.0,
        ) {
            let mut ctorus = CTorus::default();
            ctorus.angle = torus_info.angle;
            ctorus.rins = torus_info.radius - self.pdia / 2.0;
            ctorus.rout = torus_info.radius + self.pdia / 2.0;
            let z_axis = torus_info.rot_axis.normalize();
            let mut x_axis = (self.pbax_pt - torus_info.center).normalize();
            let translation = torus_info.center;
            // dbg!(torus_info.center);
            if x_axis.is_nan() {
                x_axis = -Vec3::Y;
                ctorus.rout = ctorus.rout / 2.0;
            }
            let y_axis = z_axis.cross(x_axis).normalize();
            let mat = Transform {
                rotation: Quat::from_mat3(&bevy_math::Mat3::from_cols(x_axis, y_axis, z_axis)),
                translation,
                ..Default::default()
            };
            if mat.is_nan() {
                return None;
            }
            return Some((ctorus, mat));
        }
        None
    }
}

impl Default for SCTorus {
    fn default() -> Self {
        SCTorus {
            paax_pt: Vec3::new(5.0, 0.0, 0.0),
            paax_dir: Vec3::new(1.0, 0.0, 0.0), //Down

            pbax_pt: Vec3::new(0.0, 5.0, 0.0),
            pbax_dir: Vec3::new(0.0, 1.0, 0.0), //UP
            pdia: 2.0,
        }
    }
}

impl VerifiedShape for SCTorus {
    fn check_valid(&self) -> bool {
        true
    }
}

impl BrepShapeTrait for SCTorus {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    fn key_points(&self) -> Vec<RsVec3> {
        let mut points = BrepShapeTrait::key_points(self);
        points.extend_from_slice(&[self.paax_pt.into(), self.pbax_pt.into()]);
        points
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    fn tol(&self) -> f32 {
        0.01 * self.pdia.max(1.0)
    }
}

impl From<AttrMap> for SCTorus {
    fn from(_m: AttrMap) -> Self {
        Default::default()
    }
}

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
pub struct CTorus {
    pub rins: f32,
    //内圆半径
    pub rout: f32,
    //外圆半径
    pub angle: f32, //旋转角度
    /// 环向 / 管截面两个方向的段数，**只有单位行带**（`gen_unit_shape()` 按真实半径算好
    /// 写进来；原件上是 `None`）。两个数都是 `rout` 的函数，但量化粒度不同——
    /// `rins/rout = 0.5`、360° 下 `rout = 104` 与 `105` 环向同为 36、管截面却是 16 与 20
    /// ——所以键里两个都要有（T041 B3）。读取一律走 [`Self::segment_counts`]。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segments: Option<CircularTorusSegments>,
}

/// 圆环面的两个离散方向：`GM_CircTorus`（`0x10047150`）扫掠方向喂外半径走部分回转，
/// 管截面方向喂 `(rOut − rIns)/2` 走整圆。
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct CircularTorusSegments {
    /// 环向（扫掠方向）段数。
    pub ring: i32,
    /// 管截面段数。
    pub tube: i32,
}

impl Default for CTorus {
    fn default() -> Self {
        Self {
            rins: 0.5,
            rout: 1.0,
            angle: 90.0,
            segments: None,
        }
    }
}

impl CTorus {
    /// 两个方向的段数：单位行读携带值，原件按真实半径现算。哈希与落库的单位参数都
    /// 从这里取（T041 A3）。
    #[inline]
    pub fn segment_counts(&self) -> CircularTorusSegments {
        self.segments.unwrap_or_else(|| CircularTorusSegments {
            ring: torus_ring_segments(self.rout as f64, FACET_TOL_MM, self.angle as f64),
            tube: circular_torus_tube_segments(self.rins as f64, self.rout as f64, FACET_TOL_MM),
        })
    }
}

impl VerifiedShape for CTorus {
    fn check_valid(&self) -> bool {
        self.rout > 0.0
            && self.rins >= 0.0
            && self.angle.abs() > 0.0
            && (self.rout - self.rins) > f32::EPSILON
    }
}

impl BrepShapeTrait for CTorus {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        hash_f32(self.rins / self.rout, &mut hasher);
        hash_f32(self.angle, &mut hasher);
        // 二元组整个进键（带结构，不摊平）：只混环向，环向相同而管截面段数不同的
        // 两件会共用一行（T041 B3）。
        self.segment_counts().hash(&mut hasher);
        "ctorus".hash(&mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        let rins = self.rins / self.rout;
        let unit = Self {
            rins,
            rout: 1.0,
            angle: self.angle,
            segments: Some(self.segment_counts()),
        };
        Box::new(unit)
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        Vec3::splat(self.rout)
    }

    fn tol(&self) -> f32 {
        0.001
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimCTorus(self.clone()))
    }
}

impl From<&AttrMap> for CTorus {
    fn from(m: &AttrMap) -> Self {
        // Core3D `CSG_BasicCTO::getPrimGeom`（3.1 `0x10726BE0`）在属性读取处就
        // `fmax(RINS, 0.0)`：负内半径夹成 0 照常建体（内缘贴轴），不是拒绝。
        // 与它同层夹取；`check_valid` 的 `rins >= 0.0` 保留当保险。
        // specs/009 T055，证据（gen-model 仓）docs/evidence/2026-08-24-ida-occ-retire-audit.md。
        let r_i = m.get_f32_or_default("RINS").max(0.0);
        let r_o = m.get_f32_or_default("ROUT");
        let angle = m.get_f32_or_default("ANGL");
        CTorus {
            rins: r_i,
            rout: r_o,
            angle,
            segments: None,
        }
    }
}

impl From<AttrMap> for CTorus {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}

impl From<&NamedAttrMap> for CTorus {
    fn from(m: &NamedAttrMap) -> Self {
        // 与 `From<&AttrMap>` 同一条 Core3D 夹取（T055），两条入口不得只夹一条。
        let r_i = m.get_f32_or_default("RINS").max(0.0);
        let r_o = m.get_f32_or_default("ROUT");
        let angle = m.get_f32_or_default("ANGL");
        CTorus {
            rins: r_i,
            rout: r_o,
            angle,
            segments: None,
        }
    }
}

impl From<NamedAttrMap> for CTorus {
    fn from(m: NamedAttrMap) -> Self {
        (&m).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::attval::AttrVal;

    fn attrs(rins: f64, rout: f64, angle: f64) -> AttrMap {
        let mut m = AttrMap::default();
        m.insert_by_att_name("RINS", AttrVal::DoubleType(rins));
        m.insert_by_att_name("ROUT", AttrVal::DoubleType(rout));
        m.insert_by_att_name("ANGL", AttrVal::DoubleType(angle));
        m
    }

    /// T055：Core3D（`0x10726BE0`）对 RINS 做 `fmax(RINS, 0.0)`——负内半径夹成 0
    /// 照常建体。回退成原值照收的写法，这里的 `check_valid` 断言就红。
    #[test]
    fn a_negative_inner_radius_is_clamped_to_zero_like_core3d() {
        let t = CTorus::from(&attrs(-5.0, 10.0, 90.0));
        assert_eq!(t.rins, 0.0, "负 RINS 必须夹成 0，不是照收");
        assert!(t.check_valid(), "夹取后的 rins=0 环面必须可建");

        // 夹取把负值折到 0 那一行上：与显式 RINS=0 同一个 geo_hash（E3D 里两者同一个几何）。
        let explicit_zero = CTorus::from(&attrs(0.0, 10.0, 90.0));
        assert_eq!(
            t.hash_unit_mesh_params(),
            explicit_zero.hash_unit_mesh_params(),
            "夹成 0 的负 RINS 与显式 0 必须共享一行单位几何"
        );
    }

    /// 夹取只对负值生效：RINS ≥ 0 的既有路径行为与 `geo_hash` 一位不变。
    #[test]
    fn a_non_negative_inner_radius_is_untouched() {
        let t = CTorus::from(&attrs(3.0, 10.0, 90.0));
        assert_eq!(t.rins, 3.0);
        assert!(t.check_valid());
    }

    /// T041 B3：环向相同（36）而管截面段数不同（16 / 20）的两件圆环面要分行；
    /// 单位行携带二元组并重新哈希到同一个键。
    #[test]
    fn a_circular_torus_key_carries_both_directions() {
        let torus = |rout: f32| CTorus {
            rins: rout * 0.5,
            rout,
            angle: 360.0,
            ..Default::default()
        };
        assert_eq!(
            torus(104.0).segment_counts(),
            CircularTorusSegments { ring: 36, tube: 16 }
        );
        assert_eq!(
            torus(105.0).segment_counts(),
            CircularTorusSegments { ring: 36, tube: 20 }
        );
        assert_ne!(
            torus(104.0).hash_unit_mesh_params(),
            torus(105.0).hash_unit_mesh_params()
        );

        let unit = torus(105.0).gen_unit_shape();
        assert_eq!(
            unit.hash_unit_mesh_params(),
            torus(105.0).hash_unit_mesh_params()
        );
        let unit = unit.downcast::<CTorus>().unwrap();
        assert_eq!(unit.rout, 1.0);
        assert_eq!(unit.segments, Some(CircularTorusSegments { ring: 36, tube: 20 }));
    }
}
