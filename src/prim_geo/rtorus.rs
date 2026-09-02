use crate::NamedAttrMap;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::prim_geo::helper::*;
use crate::prim_geo::libgm_discretise::{FACET_TOL_MM, torus_ring_segments};
use crate::shape::pdms_shape::*;
use crate::tool::float_tool::hash_f32;
use crate::types::attmap::AttrMap;
use bevy_ecs::prelude::*;
use bevy_transform::prelude::Transform;
use glam::{DVec3, Mat3, Quat, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use std::sync::Arc;
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
pub struct SRTorus {
    pub paax_expr: String,
    pub paax_pt: Vec3,
    //A Axis point
    pub paax_dir: Vec3, //A Axis Direction

    pub pbax_expr: String,
    pub pbax_pt: Vec3,
    //B Axis point
    pub pbax_dir: Vec3, //B Axis Direction

    pub pheig: f32,
    pub pdia: f32,
}

impl Default for SRTorus {
    fn default() -> Self {
        Self {
            paax_expr: "X".to_string(),
            paax_pt: Vec3::new(5.0, 0.0, 0.0),
            paax_dir: Vec3::X,

            pbax_expr: "Y".to_string(),
            pbax_pt: Vec3::new(0.0, 5.0, 0.0),
            pbax_dir: Vec3::Y,
            pheig: 1.0,
            pdia: 1.0,
        }
    }
}

#[derive(Default)]
struct TorusInfo {
    pub center: Vec3,
    pub angle: f32,
    pub rot_axis: Vec3,
    pub radius: f32,
}

impl SRTorus {
    pub fn convert_to_rtorus(&self) -> Option<(RTorus, Transform)> {
        if let Some(torus_info) = RotateInfo::cal_rotate_info(
            self.paax_dir,
            self.paax_pt,
            self.pbax_dir,
            self.pbax_pt,
            self.pdia / 2.0,
        ) {
            // dbg!(&torus_info);
            let mut rtorus = RTorus::default();
            rtorus.angle = torus_info.angle;
            rtorus.height = self.pheig;
            rtorus.rins = torus_info.radius - self.pdia / 2.0;
            rtorus.rout = torus_info.radius + self.pdia / 2.0;
            let z_axis = torus_info.rot_axis.normalize();
            let x_axis = (self.pbax_pt - torus_info.center).normalize();
            let y_axis = z_axis.cross(x_axis).normalize();
            let translation = torus_info.center;
            let mat = Transform {
                rotation: Quat::from_mat3(&Mat3::from_cols(x_axis, y_axis, z_axis)),
                translation,
                ..Default::default()
            };
            return Some((rtorus, mat));
        }

        None
    }
}

impl VerifiedShape for SRTorus {
    fn check_valid(&self) -> bool {
        self.pheig > 0.0 && self.pdia > 0.0
    }
}

//#[typetag::serde]
impl BrepShapeTrait for SRTorus {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    #[inline]
    fn tol(&self) -> f32 {
        0.01 * self.pdia.min(self.pheig).max(1.0)
    }
}

impl From<AttrMap> for SRTorus {
    fn from(_: AttrMap) -> Self {
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
pub struct RTorus {
    //内圆半径
    pub rins: f32,
    //外圆半径
    pub rout: f32,
    pub height: f32,
    pub angle: f32, //旋转角度
    /// 环向段数，**只有单位行带**（`gen_unit_shape()` 按真实外半径算好写进来；原件上是
    /// `None`）。矩形截面没有管向曲率，所以只有这**一元**——别照抄圆环面再加一个不存在
    /// 的轴（T041 B4）。读取一律走 [`Self::ring_segment_count`]。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ring_segments: Option<i32>,
}

impl Default for RTorus {
    fn default() -> Self {
        Self {
            rins: 0.5,
            rout: 1.0,
            height: 1.0,
            angle: 90.0,
            ring_segments: None,
        }
    }
}

impl RTorus {
    /// 环向段数：单位行读携带值，原件按真实外半径与扫角现算（`GM_RectTorus`
    /// `0x100962F0` 喂外半径走部分回转）。哈希与落库的单位参数都从这里取（T041 A3）。
    #[inline]
    pub fn ring_segment_count(&self) -> i32 {
        self.ring_segments.unwrap_or_else(|| {
            torus_ring_segments(self.rout as f64, FACET_TOL_MM, self.angle as f64)
        })
    }
}

impl VerifiedShape for RTorus {
    #[inline]
    fn check_valid(&self) -> bool {
        // `rins >= 0.0`：libgm `GM_RectTorus::validate`（3.1 `0x10030780`）接受
        // rIns = 0（判据是 rIns ≥ −1e-6，负值才报 −87），内缘贴轴的矩形环面是
        // E3D 里合法可建的形状；旧写法 `rins > 0.0` 比 libgm 还严，把它拒掉了。
        // specs/009 T055。
        self.rout > 0.0
            && self.rins >= 0.0
            && self.angle.abs() > 0.0
            && (self.rout - self.rins) > f32::EPSILON
            && self.height > f32::EPSILON
    }
}

impl BrepShapeTrait for RTorus {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        hash_f32(self.rins / self.rout, &mut hasher);
        hash_f32(self.angle, &mut hasher);
        // 只有环向这一元进键；`height` 走 `get_scaled_vec3` 的 z，本来就不进（T041 B4）。
        self.ring_segment_count().hash(&mut hasher);
        "rtorus".hash(&mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        let rins = self.rins / self.rout;
        let unit = Self {
            rins,
            rout: 1.0,
            height: 1.0,
            angle: self.angle,
            ring_segments: Some(self.ring_segment_count()),
        };
        Box::new(unit)
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        Vec3::new(self.rout, self.rout, self.height)
    }

    #[inline]
    fn tol(&self) -> f32 {
        let d = ((self.rout - self.rins) / 2.0 + self.height) / 2.0;
        0.01 * d.max(1.0)
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimRTorus(self.clone()))
    }
}

impl From<&AttrMap> for RTorus {
    fn from(m: &AttrMap) -> Self {
        // Core3D `CSG_BasicRTO::getPrimGeom`（3.1 `0x10727140`）在属性读取处就
        // `fmax(RINS, 0.0)`：负内半径夹成 0 照常建体，不是拒绝。specs/009 T055，
        // 证据（gen-model 仓）docs/evidence/2026-08-24-ida-occ-retire-audit.md。
        let rins = m.get_f32("RINS").unwrap().max(0.0);
        let rout = m.get_f32("ROUT").unwrap();
        let height = m.get_f32("HEIG").unwrap();
        let angle = m.get_f32("ANGL").unwrap();
        RTorus {
            rins,
            rout,
            height,
            angle,
            ring_segments: None,
        }
    }
}

impl From<AttrMap> for RTorus {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}

impl From<&NamedAttrMap> for RTorus {
    fn from(m: &NamedAttrMap) -> Self {
        // 与 `From<&AttrMap>` 同一条 Core3D 夹取（T055），两条入口不得只夹一条。
        let rins = m.get_f32_or_default("RINS").max(0.0);
        let rout = m.get_f32_or_default("ROUT");
        let height = m.get_f32_or_default("HEIG");
        let angle = m.get_f32_or_default("ANGL");
        RTorus {
            rins,
            rout,
            height,
            angle,
            ring_segments: None,
        }
    }
}

impl From<NamedAttrMap> for RTorus {
    fn from(m: NamedAttrMap) -> Self {
        (&m).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::attval::AttrVal;

    fn attrs(rins: f64, rout: f64, heig: f64, angle: f64) -> AttrMap {
        let mut m = AttrMap::default();
        m.insert_by_att_name("RINS", AttrVal::DoubleType(rins));
        m.insert_by_att_name("ROUT", AttrVal::DoubleType(rout));
        m.insert_by_att_name("HEIG", AttrVal::DoubleType(heig));
        m.insert_by_att_name("ANGL", AttrVal::DoubleType(angle));
        m
    }

    /// T055：Core3D（`0x10727140`）对 RINS 做 `fmax(RINS, 0.0)`。夹取回退或
    /// `check_valid` 回到 `rins > 0.0` 的旧写法，这里都会红。
    #[test]
    fn a_negative_inner_radius_is_clamped_to_zero_like_core3d() {
        let t = RTorus::from(&attrs(-5.0, 10.0, 4.0, 90.0));
        assert_eq!(t.rins, 0.0, "负 RINS 必须夹成 0，不是照收");
        assert!(
            t.check_valid(),
            "rIns=0 在 libgm `GM_RectTorus::validate`（0x10030780）里合法，本仓不得比它严"
        );

        let explicit_zero = RTorus::from(&attrs(0.0, 10.0, 4.0, 90.0));
        assert_eq!(
            t.hash_unit_mesh_params(),
            explicit_zero.hash_unit_mesh_params(),
            "夹成 0 的负 RINS 与显式 0 必须共享一行单位几何"
        );
    }

    /// 夹取只对负值生效：RINS ≥ 0 的既有路径行为一位不变。
    #[test]
    fn a_non_negative_inner_radius_is_untouched() {
        let t = RTorus::from(&attrs(3.0, 10.0, 4.0, 90.0));
        assert_eq!(t.rins, 3.0);
        assert!(t.check_valid());
    }

    /// T041 B4：矩形环面的键只多环向**一元**——`rout = 250`（90° 下 13 段）与
    /// `rout = 1000`（25 段）分行，只差 `height` 的两件仍同一行。
    #[test]
    fn a_rectangular_torus_key_carries_only_the_ring_class() {
        let torus = |rout: f32, height: f32| RTorus {
            rins: rout * 0.5,
            rout,
            height,
            angle: 90.0,
            ..Default::default()
        };
        assert_eq!(torus(250.0, 40.0).ring_segment_count(), 13);
        assert_eq!(torus(1000.0, 40.0).ring_segment_count(), 25);
        assert_ne!(
            torus(250.0, 40.0).hash_unit_mesh_params(),
            torus(1000.0, 40.0).hash_unit_mesh_params()
        );
        assert_eq!(
            torus(250.0, 40.0).hash_unit_mesh_params(),
            torus(250.0, 900.0).hash_unit_mesh_params(),
            "height 不进键"
        );

        let unit = torus(1000.0, 40.0).gen_unit_shape();
        assert_eq!(
            unit.hash_unit_mesh_params(),
            torus(1000.0, 40.0).hash_unit_mesh_params()
        );
        let unit = unit.downcast::<RTorus>().unwrap();
        assert_eq!((unit.ring_segments, unit.rout, unit.height), (Some(25), 1.0, 1.0));
    }
}
