use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::prim_geo::basic::*;
use crate::shape::pdms_shape::{BrepShapeTrait, PlantMesh, RsVec3, VerifiedShape};
use glam::Vec3;
use hexasphere::shapes::IcoSphere;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
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
}

impl Default for Sphere {
    fn default() -> Self {
        Sphere {
            center: Default::default(),
            radius: 1.0,
        }
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

    fn hash_unit_mesh_params(&self) -> u64 {
        SPHERE_GEO_HASH //代表SPHERE
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(Sphere::default())
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
        }
    }
}

impl From<&NamedAttrMap> for Sphere {
    fn from(m: &NamedAttrMap) -> Self {
        Self {
            center: Default::default(),
            radius: m.get_f32("RADI").unwrap_or_default(),
        }
    }
}
