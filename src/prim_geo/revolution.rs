#[cfg(all(feature = "gen_model", feature = "manifold"))]
use crate::csg::manifold::*;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
#[cfg(feature = "occ")]
use crate::prim_geo::basic::OccSharedShape;
use crate::prim_geo::wire::*;
use crate::shape::pdms_shape::{BrepShapeTrait, PlantMesh, RsVec3, TRI_TOL, VerifiedShape};
use crate::tool::float_tool::{f32_round_3, hash_f32, hash_vec3};
use approx::AbsDiffEq;
use approx::abs_diff_eq;
use glam::{Vec2, Vec3};
#[cfg(feature = "occ")]
use opencascade::angle::ToAngle;
#[cfg(feature = "occ")]
use opencascade::primitives::*;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
#[derive(
    Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,
)]
pub struct Revolution {
    pub verts: Vec<Vec<Vec3>>,
    pub angle: f32,
    pub rot_dir: Vec3,
    pub rot_pt: Vec3,
}

impl Default for Revolution {
    fn default() -> Self {
        Self {
            verts: vec![],
            angle: 90.0,
            rot_dir: Vec3::X,   //默认绕X轴旋转
            rot_pt: Vec3::ZERO, //默认旋转点
        }
    }
}

impl VerifiedShape for Revolution {
    fn check_valid(&self) -> bool {
        //add some other restrictions
        true
        // self.angle.abs() > std::f32::EPSILON
    }
}

impl BrepShapeTrait for Revolution {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    ///revolve 有些问题，暂时用manifold来代替
    ///如果是沿自己的一条边旋转，需要弄清楚为啥三角化出来的不对
    #[cfg(feature = "occ")]
    fn gen_occ_shape(&self) -> anyhow::Result<OccSharedShape> {
        let wires = gen_occ_wires(&self.verts)?;
        let angle = if abs_diff_eq!(self.angle, 360.0, epsilon = 0.01)
            || self.angle > 360.0
            || self.angle == 0.0
        {
            360.0
        } else {
            self.angle as f64
        };
        // dbg!(angle);
        let r = Face::from_wires(&wires)?.revolve(
            self.rot_pt.as_dvec3(),
            self.rot_dir.as_dvec3(),
            Some(angle.degrees()),
        );
        return Ok(OccSharedShape::new(r.into_shape()));
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.verts.iter().flatten().for_each(|v| {
            hash_vec3::<DefaultHasher>(v, &mut hasher);
        });
        "Revolution".hash(&mut hasher);
        hash_f32(self.angle, &mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    #[inline]
    fn tol(&self) -> f32 {
        use parry2d::bounding_volume::Aabb;
        let pts = self
            .verts
            .iter()
            .flatten()
            .map(|x| nalgebra::Point2::from(nalgebra::Vector2::from(x.truncate())))
            .collect::<Vec<_>>();
        let profile_aabb = Aabb::from_points(&pts);
        0.001 * profile_aabb.bounding_sphere().radius.max(1.0)
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimRevolution(self.clone()))
    }

    ///使用manifold生成拉身体的mesh
    #[inline]
    fn need_use_csg(&self) -> bool {
        false
    }
}
