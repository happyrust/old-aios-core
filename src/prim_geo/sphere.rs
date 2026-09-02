use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::prim_geo::libgm_discretise::{FACET_TOL_MM, cylinder_segments};
use crate::shape::pdms_shape::{BrepShapeTrait, PlantMesh, RsVec3, VerifiedShape};
use glam::Vec3;
use hexasphere::shapes::IcoSphere;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::f64::consts::PI;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::NamedAttrMap;
use crate::types::attmap::AttrMap;
use bevy_ecs::prelude::*;

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
//
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
    /// 绕轴段数 `n`，**只有单位行带**（`gen_unit_shape()` 按真实半径算好写进来；
    /// 原件上是 `None`）。经向带数恒为 `n/2`（`GM_Sphere::calcFacetsWithoutSurfaces`
    /// `0x100A20F0`），不是独立自由度，所以只带这一个数（T041 B5）。
    /// 读取一律走 [`Self::segment_count`]。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segments: Option<i32>,
}

impl Default for Sphere {
    fn default() -> Self {
        Sphere {
            center: Default::default(),
            radius: 1.0,
            segments: None,
        }
    }
}

impl Sphere {
    /// 绕轴段数：单位行读携带值，原件按真实半径现算（`GM_Sphere` 与圆柱同一条规则，
    /// 喂自己的半径）。哈希与落库的单位参数都从这里取（T041 A3）。
    #[inline]
    pub fn segment_count(&self) -> i32 {
        self.segments
            .unwrap_or_else(|| cylinder_segments(self.radius as f64, FACET_TOL_MM))
    }
}

impl VerifiedShape for Sphere {
    #[inline]
    fn check_valid(&self) -> bool {
        self.radius > f32::EPSILON
    }
}

impl BrepShapeTrait for Sphere {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    //由于geom kernel还不支持fixed point ，暂时不用这个shell去生成mesh
    ///获得关键点
    fn key_points(&self) -> Vec<RsVec3> {
        vec![self.center.into()]
    }

    /// 球的身份键只有绕轴段数一个自由度（T041 B5）：半径在实例变换里，`stacks` 恒为
    /// `n/2` 不进键。同段数等价类的球共享一行，跨等价类分行。
    fn hash_unit_mesh_params(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.segment_count().hash(&mut hasher);
        "sphere".hash(&mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(Sphere {
            segments: Some(self.segment_count()),
            ..Default::default()
        })
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        Vec3::splat(self.radius)
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimSphere(self.clone()))
    }

    ///直接通过基本体的参数，生成模型
    fn gen_csg_mesh(&self) -> Option<PlantMesh> {
        let generated = IcoSphere::new(32, |point| {
            let inclination = point.y.acos();
            let azimuth = point.z.atan2(point.x);

            let norm_inclination = inclination / std::f32::consts::PI;
            let norm_azimuth = 0.5 - (azimuth / std::f32::consts::TAU);

            [norm_azimuth, norm_inclination]
        });

        let raw_points = generated.raw_points();

        let points = raw_points
            .iter()
            .map(|&p| Vec3::from(p * self.radius))
            .collect::<Vec<Vec3>>();

        let normals = raw_points
            .iter()
            .copied()
            .map(Into::into)
            .collect::<Vec<Vec3>>();

        let mut indices = Vec::with_capacity(generated.indices_per_main_triangle() * 20);
        for i in 0..20 {
            generated.get_indices(i, &mut indices);
        }

        //球也需要提供wireframe的绘制
        return Some(PlantMesh {
            indices,
            vertices: points,
            normals,
            wire_vertices: vec![],
            aabb: None,
        });
    }

    fn need_use_csg(&self) -> bool {
        true
    }
}

impl From<&AttrMap> for Sphere {
    fn from(m: &AttrMap) -> Self {
        Self {
            center: Default::default(),
            radius: m.get_f32("RADI").unwrap_or_default(),
            segments: None,
        }
    }
}

impl From<&NamedAttrMap> for Sphere {
    fn from(m: &NamedAttrMap) -> Self {
        Self {
            center: Default::default(),
            radius: m.get_f32("RADI").unwrap_or_default(),
            segments: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// T041 B5：球的键只混绕轴段数一个数。R=100（32 段）与 R=295（56 段）分行，
    /// R=100 与 R=101 同为 32 段共享一行；单位行携带 `n` 并重新哈希到同一个键。
    #[test]
    fn the_sphere_key_is_its_single_segment_class() {
        let sphere = |radius: f32| Sphere {
            radius,
            ..Default::default()
        };
        assert_eq!(sphere(100.0).segment_count(), 32);
        assert_eq!(sphere(101.0).segment_count(), 32);
        assert_eq!(sphere(295.0).segment_count(), 56);
        assert_ne!(
            sphere(100.0).hash_unit_mesh_params(),
            sphere(295.0).hash_unit_mesh_params()
        );
        assert_eq!(
            sphere(100.0).hash_unit_mesh_params(),
            sphere(101.0).hash_unit_mesh_params()
        );

        let unit = sphere(295.0).gen_unit_shape();
        assert_eq!(
            unit.hash_unit_mesh_params(),
            sphere(295.0).hash_unit_mesh_params()
        );
        let unit = unit.downcast::<Sphere>().unwrap();
        assert_eq!((unit.segments, unit.radius), (Some(56), 1.0));
    }
}
