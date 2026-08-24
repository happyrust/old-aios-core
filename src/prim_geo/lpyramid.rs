use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::shape::pdms_shape::{BrepShapeTrait, VerifiedShape};
use bevy_ecs::prelude::*;
use glam::{DVec3, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
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
