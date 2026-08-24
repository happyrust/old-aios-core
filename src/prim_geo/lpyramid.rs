use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
#[cfg(feature = "occ")]
use crate::prim_geo::basic::OccSharedShape;
use crate::shape::pdms_shape::{BrepShapeTrait, VerifiedShape};
use bevy_ecs::prelude::*;
use glam::{DVec3, Vec3};
#[cfg(feature = "occ")]
use opencascade::primitives::*;
use serde::{Deserialize, Serialize};
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
pub struct LPyramid {
    pub pbax_pt: Vec3,
    pub pbax_dir: Vec3, //B Axis Direction

    pub pcax_pt: Vec3,
    pub pcax_dir: Vec3, //C Axis Direction

    pub paax_pt: Vec3,
    pub paax_dir: Vec3, //A Axis Direction

    pub pbtp: f32,
    pub pctp: f32,
    //y top
    pub pbbt: f32,
    pub pcbt: f32, // y bottom

    pub ptdi: f32,
    pub pbdi: f32,
    pub pbof: f32,
    // x offset
    pub pcof: f32, // y offset
}

impl Default for LPyramid {
    fn default() -> Self {
        Self {
            pbax_pt: Default::default(),
            pbax_dir: Vec3::X,
            pcax_pt: Default::default(),
            pcax_dir: Vec3::Y,
            paax_pt: Default::default(),
            paax_dir: Vec3::Z,
            pbtp: 1.0,
            pctp: 1.0,
            pbbt: 1.0,
            pcbt: 1.0,
            ptdi: 1.0,
            pbdi: 0.0,
            pbof: 0.0,
            pcof: 0.0,
        }
    }
}

impl VerifiedShape for LPyramid {
    fn check_valid(&self) -> bool {
        let size_flag =
            self.pbtp * self.pctp >= f32::EPSILON || self.pbbt * self.pcbt >= f32::EPSILON;
        if !size_flag {
            return false;
        }
        (self.pbtp >= 0.0 && self.pctp >= 0.0 && self.pbbt >= 0.0 && self.pcbt >= 0.0)
            && ((self.pbtp + self.pctp) > f32::EPSILON || (self.pbbt + self.pcbt) > f32::EPSILON)
    }
}

//#[typetag::serde]
impl BrepShapeTrait for LPyramid {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    //涵盖的情况，需要考虑，上边只有一条边，和退化成点的情况
    #[cfg(feature = "occ")]
    fn gen_occ_shape(&self) -> anyhow::Result<OccSharedShape> {
        //todo 以防止出现有单个点的情况，暂时用这个模拟
        let tx = (self.pbtp / 2.0).max(0.001) as f64;
        let ty = (self.pctp / 2.0).max(0.001) as f64;
        let bx = (self.pbbt / 2.0).max(0.001) as f64;
        let by = (self.pcbt / 2.0).max(0.001) as f64;
        //这里需要按照实际的变换方位来计算
        let ox = self.pbof as f64 * self.pbax_dir.as_dvec3();
        let oy = self.pcof as f64 * self.pcax_dir.as_dvec3();
        // dbg!((ox, oy));
        let offset_3d = ox + oy;
        // let offset_3d = DVec3::new(offset.x as _, offset.y as _, 0.0);
        // dbg!(offset_3d);
        let h2 = 0.5 * (self.ptdi - self.pbdi) as f64;

        let mut polys = vec![];
        let mut verts = vec![];

        let pts = vec![
            DVec3::new(-tx, -ty, h2) + offset_3d,
            DVec3::new(tx, -ty, h2) + offset_3d,
            DVec3::new(tx, ty, h2) + offset_3d,
            DVec3::new(-tx, ty, h2) + offset_3d,
        ];
        if tx + ty < f64::EPSILON {
            verts.push(Vertex::new(DVec3::new(offset_3d.x, offset_3d.y, h2)));
        } else {
            polys.push(Wire::from_ordered_points(pts)?);
        }

        let pts = vec![
            DVec3::new(-bx, -by, -h2),
            DVec3::new(bx, -by, -h2),
            DVec3::new(bx, by, -h2),
            DVec3::new(-bx, by, -h2),
        ];
        if bx + by < f64::EPSILON {
            verts.push(Vertex::new(DVec3::new(-offset_3d.x, -offset_3d.y, -h2)));
        } else {
            polys.push(Wire::from_ordered_points(pts)?);
        }

        Ok(OccSharedShape::new(
            Solid::loft_with_points(polys.iter(), verts.iter())?.into_shape(),
        ))
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        let bytes = bincode::serialize(self).unwrap();
        let mut hasher = DefaultHasher::default();
        bytes.hash(&mut hasher);
        "LPyramid".hash(&mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimLPyramid(self.clone()))
    }
}
