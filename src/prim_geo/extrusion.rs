#[cfg(all(feature = "gen_model", feature = "manifold"))]
use crate::csg::manifold::*;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::prim_geo::wire::*;
use crate::shape::pdms_shape::*;
use crate::tool::float_tool::{f32_round_3, hash_f32, hash_vec3};
use anyhow::anyhow;
use bevy_ecs::prelude::*;
use glam::{DVec3, Vec2, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

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
pub struct Extrusion {
    //xy 为坐标，z为倒角切半径
    pub verts: Vec<Vec<Vec3>>,
    pub height: f32,
    pub cur_type: CurveType,
}

impl Default for Extrusion {
    fn default() -> Self {
        Self {
            verts: vec![],
            height: 100.0,
            cur_type: CurveType::Fill,
        }
    }
}

impl VerifiedShape for Extrusion {
    fn check_valid(&self) -> bool {
        self.height > std::f32::EPSILON
    }
}

impl BrepShapeTrait for Extrusion {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    ///限制参数大小，主要是对负实体的不合理进行限制
    fn apply_limit_by_size(&mut self, l: f32) {
        self.height = self.height.min(l);
        dbg!(&self.height);
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.verts.iter().flatten().for_each(|v| {
            hash_vec3::<DefaultHasher>(v, &mut hasher);
        });
        "Extrusion".hash(&mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        let unit = Self {
            verts: self.verts.clone(),
            height: 100.0, //开放一点大小,不然三角化出来的不对
            cur_type: self.cur_type.clone(),
            ..Default::default()
        };
        Box::new(unit)
    }

    //沿着指定方向拉伸 pbax_dir
    fn get_scaled_vec3(&self) -> Vec3 {
        Vec3::new(1.0, 1.0, self.height as f32 / 100.0)
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
        Some(PdmsGeoParam::PrimExtrusion(self.clone()))
    }

    ///使用manifold生成拉身体的mesh
    fn need_use_csg(&self) -> bool {
        false
    }
}
