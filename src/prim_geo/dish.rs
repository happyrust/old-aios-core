use std::collections::hash_map::DefaultHasher;
use std::f32::consts::PI;

use crate::NamedAttrMap;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::prim_geo::helper::cal_ref_axis;
use crate::shape::pdms_shape::{BrepShapeTrait, PlantMesh, RsVec3, TRI_TOL, VerifiedShape};
use crate::tool::float_tool::hash_f32;
use crate::types::attmap::AttrMap;
use anyhow::anyhow;
use bevy_ecs::prelude::*;
use glam::DVec3;
use glam::Vec3;
use nalgebra::ComplexField;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::thread::sleep;

//可不可以用来表达 sphere
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
pub struct Dish {
    pub paax_expr: String,
    pub paax_pt: Vec3,
    //Axis point
    pub paax_dir: Vec3, //Axis Direction

    pub pdis: f32,
    pub pheig: f32,
    pub pdia: f32,
    //diameter
    //r = √[(a^2 - b^2) / a^2]
    #[serde(default)]
    pub prad: f32,
}

impl Default for Dish {
    fn default() -> Self {
        Self {
            paax_expr: "Z".to_string(),
            paax_pt: Default::default(),
            paax_dir: Vec3::Z,
            pdis: 0.0,
            pheig: 1.0,
            pdia: 2.0,
            prad: 0.0,
        }
    }
}

impl VerifiedShape for Dish {
    fn check_valid(&self) -> bool {
        self.pdia > f32::EPSILON && self.pheig > f32::EPSILON
    }
}

/// dish的实现 shape trait
impl BrepShapeTrait for Dish {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    ///获得关键点
    fn key_points(&self) -> Vec<RsVec3> {
        let r = self.pdia / 2.0;
        let mut h = self.pheig;
        //是个椭圆, 先暂时按圆来处理，然后再拉伸
        if self.prad > 0.0 {
            h = r;
        }
        let radius = (r * r + h * h) / (2.0f32 * h);
        if radius < f32::EPSILON {
            return vec![];
        }
        let sinval = (r / radius).max(-1.0f32).min(1.0f32);
        let mut theta = (sinval).asin();
        if r < h {
            theta = PI - theta;
        }

        let rot_axis = self.paax_dir.normalize();
        let c = rot_axis * self.pdis + self.paax_pt;
        let ref_axis = cal_ref_axis(&rot_axis);
        let p0 = rot_axis * h + c;
        let center = p0 - radius * rot_axis;
        vec![center.into()]
    }

    fn tol(&self) -> f32 {
        0.001 * self.pdia.max(1.0)
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        let r = self.pdia / 2.0;
        let h = self.pheig;
        let radius = (r * r + h * h) / (2.0f32 * h);
        let sinval = (r / radius).max(-1.0f32).min(1.0f32);
        let mut theta = (sinval).asin();
        if radius < f32::EPSILON {
            return 0;
        }
        let mut beta = (h / radius / 2.0).atan();
        if r < h {
            theta = PI - theta;
            beta = PI + beta;
        }
        let mut hasher = DefaultHasher::new();

        hash_f32(theta, &mut hasher);
        hash_f32(self.prad, &mut hasher);
        hash_f32(beta, &mut hasher);
        "dish".hash(&mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        //按比例跳到单位直径圆下
        let dia = self.pdia;
        let h = self.pheig / dia;
        let prad = self.prad / dia;
        Box::new(Self {
            pheig: h,
            pdia: 1.0,
            prad,
            ..Default::default()
        })
    }

    fn get_scaled_vec3(&self) -> Vec3 {
        if self.prad > 0.0 {
            Vec3::new(
                self.pdia,
                self.pdia,
                (self.pheig / (self.pdia / 2.0)) * self.pdia,
            )
        } else {
            Vec3::new(self.pdia, self.pdia, self.pdia)
        }
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimDish(self.clone()))
    }
}

impl From<&AttrMap> for Dish {
    fn from(m: &AttrMap) -> Self {
        Self {
            paax_expr: "Z".to_string(),
            paax_pt: Default::default(),
            paax_dir: Vec3::Z,
            pdis: 0.0,
            pheig: m.get_f32_or_default("HEIG"),
            pdia: m.get_f32_or_default("DIAM"),
            prad: m.get_f32_or_default("RADI"),
        }
    }
}

impl From<AttrMap> for Dish {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}

impl From<&NamedAttrMap> for Dish {
    fn from(m: &NamedAttrMap) -> Self {
        Self {
            paax_expr: "Z".to_string(),
            paax_pt: Default::default(),
            paax_dir: Vec3::Z,
            pdis: 0.0,
            pheig: m.get_f32_or_default("HEIG"),
            pdia: m.get_f32_or_default("DIAM"),
            prad: m.get_f32_or_default("RADI"),
        }
    }
}

impl From<NamedAttrMap> for Dish {
    fn from(m: NamedAttrMap) -> Self {
        (&m).into()
    }
}
