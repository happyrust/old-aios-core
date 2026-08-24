use crate::shape::pdms_shape::{BrepShapeTrait, VerifiedShape};
use bevy_ecs::prelude::*;
use glam::Vec3;
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
pub struct Facet {
    pub polygons: Vec<Polygon>,
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
pub struct Polygon {
    pub contours: Vec<Contour>,
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
pub struct Contour {
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
}

impl VerifiedShape for Facet {
    fn check_valid(&self) -> bool {
        true
    }
}

impl BrepShapeTrait for Facet {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        let bytes = bincode::serialize(self).unwrap();
        let mut hasher = DefaultHasher::default();
        bytes.hash(&mut hasher);
        hasher.finish()
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        Vec3::ONE
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }
}

// impl Facet {
//     fn to2d(
//         pts: &[[f32; 3]],
//         normal: [f32; 3],
//         coord_sys: &mut [Vec3; 3],
//     ) -> Vec<lyon::math::Point> {
//         let mut polygon2d = Vec::new();
//         let x_n: Vec3;
//         let y_n: Vec3;
//         let v0: Vec3;
//         if coord_sys[1].length_squared() < f32::EPSILON {
//             v0 = Vec3::from_slice(&pts[0]);
//             let v1 = Vec3::from_slice(&pts[1]);
//             let loc_x = (v1 - v0).normalize();
//             let n = Vec3::from_slice(&normal).normalize();
//
//             let loc_y = n.cross(loc_x);
//             x_n = loc_x.normalize();
//             y_n = loc_y.normalize();
//
//             coord_sys[0] = v0;
//             coord_sys[1] = x_n;
//             coord_sys[2] = y_n;
//         } else {
//             v0 = coord_sys[0];
//             x_n = coord_sys[1];
//             y_n = coord_sys[2];
//         }
//
//         for idx in 0..pts.len() {
//             let to_p = Vec3::from_slice(&pts[idx]) - v0;
//             polygon2d.push(lyon::math::Point::new(
//                 to_p.dot(x_n) as f32,
//                 to_p.dot(y_n) as f32,
//             ));
//         }
//         polygon2d
//     }
// }
