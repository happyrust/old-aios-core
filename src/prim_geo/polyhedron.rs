use crate::parsed_data::geo_params_data::PdmsGeoParam;
#[cfg(feature = "occ")]
use crate::prim_geo::basic::OccSharedShape;
use crate::shape::pdms_shape::{BrepShapeTrait, PlantMesh, RsVec3, TRI_TOL, VerifiedShape};
use anyhow::anyhow;
use bevy_ecs::prelude::*;
use glam::Vec3;
use itertools::Itertools;
use nalgebra::Point;
#[cfg(feature = "occ")]
use opencascade::primitives::{Face, Shell, Wire};
#[cfg(feature = "opencascade")]
use opencascade::{Axis, Edge, OCCShape, Vertex, Wire};
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
pub struct Polyhedron {
    pub polygons: Vec<Polygon>,
    pub mesh: Option<PlantMesh>,
    pub is_polyhe: bool,
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
    pub loops: Vec<Vec<Vec3>>,
}

impl Polygon {
}

impl VerifiedShape for Polyhedron {
    fn check_valid(&self) -> bool {
        !self.polygons.is_empty()
    }
}

impl BrepShapeTrait for Polyhedron {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    #[inline]
    fn tol(&self) -> f32 {
        // let pts: Vec<parry3d::math::Point<f32>> = self
        //     .polygons
        //     .iter()
        //     .map(|x| x.verts.iter().map(|y| y.clone().into()))
        //     .flatten()
        //     .collect();
        // let profile_aabb = parry3d::bounding_volume::Aabb::from_points(&pts);
        // 0.01 * profile_aabb.bounding_sphere().radius.max(1.0)
        0.01
    }

    #[cfg(feature = "occ")]
    fn gen_occ_shape(&self) -> anyhow::Result<OccSharedShape> {
        let mut faces = vec![];
        for polygon in &self.polygons {
            // if polygon.loops.len() >= 1 {
            //     continue;
            // }
            let mut wires = vec![];
            for verts in &polygon.loops {
                if let Ok(wire) = Wire::from_ordered_points(verts.iter().map(|x| x.as_dvec3())) {
                    //需要检查是否能生成 face
                    if let Ok(_) = Face::try_from_wire(&wire) {
                        wires.push(wire);
                    }
                } else {
                    // println!("Failed to create wire from points: {:?}", polygon);
                }
            }
            if wires.is_empty() {
                continue;
            }
            if let Ok(face) = Face::from_wires(&wires) {
                faces.push(face);
            } else {
                // println!("Failed to create face from wire: {:?}", polygon);
            }
        }
        let shell = Shell::from_faces(faces)?;
        Ok(OccSharedShape::new(shell.into()))
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        let bytes = bincode::serialize(self).unwrap();
        let mut hasher = DefaultHasher::default();
        bytes.hash(&mut hasher);
        "Polyhedron".hash(&mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimPolyhedron(self.clone()))
    }
}
