use bevy_ecs::prelude::*;
use bevy_transform::prelude::Transform;
use glam::{DVec2, DVec3, Quat, Vec3};
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
#[cfg(feature = "truck")]
use truck_modeling::Shell;
#[cfg(feature = "truck")]
use truck_modeling::builder::*;

use crate::NamedAttrMap;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::prim_geo::facet_caliber::{FacetCaliber, circular_torus_caliber};
use crate::prim_geo::helper::RotateInfo;
#[cfg(feature = "truck")]
use crate::shape::pdms_shape::BrepMathTrait;
use crate::shape::pdms_shape::{BrepShapeTrait, PlantMesh, RsVec3, TRI_TOL, VerifiedShape};
use crate::tool::float_tool::hash_f32;
use crate::types::attmap::AttrMap;
use serde::{Deserialize, Serialize};

#[cfg(feature = "occ")]
use crate::prim_geo::basic::OccSharedShape;
#[cfg(feature = "occ")]
use opencascade::angle::ToAngle;
#[cfg(feature = "occ")]
use opencascade::primitives::IntoShape;
#[cfg(feature = "occ")]
use opencascade::primitives::{Shape, Wire};
#[cfg(feature = "occ")]
use opencascade::workplane::Workplane;

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

    #[cfg(feature = "truck")]
    fn gen_brep_shell(&self) -> Option<Shell> {
        if let Some(torus_info) = RotateInfo::cal_rotate_info(
            self.paax_dir,
            self.paax_pt,
            self.pbax_dir,
            self.pbax_pt,
            self.pdia / 2.0,
        ) {
            let circle_origin = self.paax_pt.point3();
            let pt_0 = self.paax_pt + torus_info.rot_axis * self.pdia / 2.0;
            let v = builder::vertex(pt_0.point3());
            let rot_axis = torus_info.rot_axis.vector3();
            let w = builder::rsweep(
                &v,
                circle_origin,
                -self.paax_dir.normalize().vector3(),
                Rad(7.0),
            );
            if let Ok(disk) = builder::try_attach_plane(&vec![w]) {
                let center = torus_info.center.point3();
                let mut solid = builder::rsweep(
                    &disk,
                    center,
                    rot_axis,
                    Rad(torus_info.angle.to_radians() as f64),
                )
                .into_boundaries();
                return solid.pop();
            }
        }
        None
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
    #[serde(default)]
    pub mesh_caliber: FacetCaliber,
}

impl Default for CTorus {
    fn default() -> Self {
        Self {
            rins: 0.5,
            rout: 1.0,
            angle: 90.0,
            mesh_caliber: FacetCaliber::default(),
        }
    }
}

impl CTorus {
    pub fn facet_caliber(&self) -> FacetCaliber {
        if self.mesh_caliber.is_explicit() {
            self.mesh_caliber
        } else {
            circular_torus_caliber(self.rins as f64, self.rout as f64, self.angle as f64)
        }
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

    #[cfg(feature = "truck")]
    fn gen_brep_shell(&self) -> Option<Shell> {
        let radius = ((self.rout - self.rins) / 2.0) as f64;
        if radius <= 0.0 {
            return None;
        }
        let circle_origin = Point3::new(self.rins as f64 + radius, 0.0, 0.0);
        let v = builder::vertex(Point3::new(self.rout as f64, 0.0, 0.0));
        let w = builder::rsweep(&v, circle_origin, Vector3::new(0.0, 1.0, 0.0), Rad(7.0));
        if let Ok(disk) = builder::try_attach_plane(&vec![w]) {
            let mut solid = builder::rsweep(
                &disk,
                Point3::new(0.0, 0.0, 0.0),
                Vector3::new(0.0, 0.0, 1.0),
                Rad(self.angle.to_radians() as f64),
            )
            .into_boundaries();
            return solid.pop();
        }
        None
    }

    #[cfg(feature = "occ")]
    fn gen_occ_shape(&self) -> anyhow::Result<OccSharedShape> {
        let r1 = (self.rins + self.rout) as f64 / 2.0;
        let r2 = (self.rout - self.rins) as f64 / 2.0;

        let center = DVec2::new(r1, 0.0);
        let face = Workplane::xz()
            .translated(center.extend(0.0))
            .circle(0.0, 0.0, r2)
            .unwrap()
            .to_face();
        let r = face.revolve(DVec3::ZERO, DVec3::Z, Some(self.angle.degrees()));
        return Ok(OccSharedShape::new(r.into_shape()));
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        hash_f32(self.rins / self.rout, &mut hasher);
        hash_f32(self.angle, &mut hasher);
        self.facet_caliber().hash(&mut hasher);
        "ctorus".hash(&mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        let rins = self.rins / self.rout;
        let unit = Self {
            rins,
            rout: 1.0,
            angle: self.angle,
            mesh_caliber: self.facet_caliber(),
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
        let r_i = m.get_f32_or_default("RINS");
        let r_o = m.get_f32_or_default("ROUT");
        let angle = m.get_f32_or_default("ANGL");
        CTorus {
            rins: r_i,
            rout: r_o,
            angle,
            mesh_caliber: FacetCaliber::default(),
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
        let r_i = m.get_f32_or_default("RINS");
        let r_o = m.get_f32_or_default("ROUT");
        let angle = m.get_f32_or_default("ANGL");
        CTorus {
            rins: r_i,
            rout: r_o,
            angle,
            mesh_caliber: FacetCaliber::default(),
        }
    }
}

impl From<NamedAttrMap> for CTorus {
    fn from(m: NamedAttrMap) -> Self {
        (&m).into()
    }
}
