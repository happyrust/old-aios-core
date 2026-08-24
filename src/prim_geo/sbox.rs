use crate::NamedAttrMap;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::prim_geo::basic::*;
use crate::shape::pdms_shape::*;
use crate::types::attmap::AttrMap;
use bevy_ecs::prelude::*;
use glam::Vec3;
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
pub struct SBox {
    pub center: Vec3,
    pub size: Vec3,
}

impl Default for SBox {
    fn default() -> Self {
        SBox {
            center: Default::default(),
            size: Vec3::new(1.0, 1.0, 1.0),
        }
    }
}

impl VerifiedShape for SBox {
    #[inline]
    fn check_valid(&self) -> bool {
        self.size.x > f32::EPSILON && self.size.y > f32::EPSILON && self.size.z > f32::EPSILON
    }
}

//#[typetag::serde]
impl BrepShapeTrait for SBox {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    fn apply_limit_by_size(&mut self, l: f32) {
        self.size.x = self.size.x.min(l);
        self.size.y = self.size.y.min(l);
        self.size.z = self.size.z.min(l);
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        BOX_GEO_HASH
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(Self::default())
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        self.size
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimBox(self.clone()))
    }
}

impl From<&AttrMap> for SBox {
    fn from(m: &AttrMap) -> Self {
        SBox {
            center: Default::default(),
            size: Vec3::new(
                m.get_f32("XLEN").unwrap_or_default(),
                m.get_f32("YLEN").unwrap_or_default(),
                m.get_f32("ZLEN").unwrap_or_default(),
            ),
        }
    }
}

impl From<AttrMap> for SBox {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}

impl From<&NamedAttrMap> for SBox {
    fn from(m: &NamedAttrMap) -> Self {
        SBox {
            center: Default::default(),
            size: Vec3::new(
                m.get_f32("XLEN").unwrap_or_default(),
                m.get_f32("YLEN").unwrap_or_default(),
                m.get_f32("ZLEN").unwrap_or_default(),
            ),
        }
    }
}

impl From<NamedAttrMap> for SBox {
    fn from(m: NamedAttrMap) -> Self {
        (&m).into()
    }
}
