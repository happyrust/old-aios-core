use std::collections::hash_map::DefaultHasher;
use std::f32::consts::PI;

use crate::NamedAttrMap;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::prim_geo::helper::cal_ref_axis;
use crate::prim_geo::libgm_discretise::{
    FACET_TOL_MM, elliptical_dish_facets, spherical_dish_facets,
};
use crate::shape::pdms_shape::{BrepShapeTrait, PlantMesh, RsVec3, TRI_TOL, VerifiedShape};
use crate::tool::float_tool::{f32_round_3, hash_f32};
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
    /// 段数元组，**只有单位行带**（`gen_unit_shape()` 按真实尺寸算好写进来；原件上是
    /// `None`）。两个分支元数不同——球碟 `(around, meridional)`、椭圆碟
    /// `(around, hub, knuckle)`——所以带的是一个枚举而不是摊平的数组：椭圆碟
    /// `a=1000, h=5` 的 `(100, 2, 2)` 与球碟 `a=1000, h=35` 的 `(100, 2)` 前两位逐位相同，
    /// 摊平就撞键（T041 B2）。读取一律走 [`Self::segment_counts`]。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segments: Option<DishSegments>,
}

/// 碟的离散段数，按分支分开（元数不同）。
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub enum DishSegments {
    /// 球碟（`prad ≤ 0`）：`GM_SDish::calcFacets`（`0x10099CF0`）——绕轴 + 经向。
    Spherical { around: i32, meridional: i32 },
    /// 椭圆碟（`prad > 0`，实为托里球形封头）：`GM_EDish::calcFacetsWithoutSurfaces`
    /// （`0x10054AB0`）——绕轴 + 球冠经向 + 拐角经向。
    Elliptical {
        around: i32,
        hub: i32,
        knuckle: i32,
    },
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
            segments: None,
        }
    }
}

impl Dish {
    /// `RADI` 只是开关：`> 0` 走托里球形封头（PDMS 叫「椭圆碟」），否则是球碟
    /// （Core3D `CSG_BasicDIS::getPrimGeom` `0x10726D10` 读了它也只用来判分支）。
    #[inline]
    pub fn is_elliptical(&self) -> bool {
        self.prad > 0.0
    }

    /// 键里与落库里那个 `prad`：**按直径归一并按三位小数量化**。
    ///
    /// 键的其余分量（`theta` / `beta`）只由 `pheig/(pdia/2)` 决定，是尺度无关量；raw
    /// `prad` 曾是键里唯一一个带真实长度的分量，于是同比例不同尺寸的两件碟被判成两行
    /// （多占一行），而 raw `prad` 相同、直径不同的两件反而同键、各落一份不同的单位几何
    /// （少落一行，后写的顶掉先写的，`geo_hash` 一路都对得上）。`hash_f32` 本身按
    /// `f32_round_3` 量化，落库值必须取**同一个**量化值，否则 T002 那个「同一个 id 两份
    /// param」在碟上原样复现（gen-model specs/009 T053 第 (3) 条 / T041 `b1b` `b1c`）。
    #[inline]
    pub fn unit_prad(&self) -> f32 {
        f32_round_3(self.prad / self.pdia)
    }

    /// 段数元组：单位行读携带值，原件按真实尺寸现算（两个分支各自的 libgm 规则）。
    /// 尺寸退化算不出母线时回 `None`——键那边随之只剩形状分量，落库值也不带段数，
    /// 生成器到时按 `check_valid` / 规则自己响亮失败。哈希与落库同源（T041 A3）。
    pub fn segment_counts(&self) -> Option<DishSegments> {
        if self.segments.is_some() {
            return self.segments;
        }
        let (a, h) = ((self.pdia / 2.0) as f64, self.pheig as f64);
        if self.is_elliptical() {
            elliptical_dish_facets(a, h, FACET_TOL_MM).map(|f| DishSegments::Elliptical {
                around: f.around,
                hub: f.hub,
                knuckle: f.knuckle,
            })
        } else {
            spherical_dish_facets(a, h, FACET_TOL_MM).map(|f| DishSegments::Spherical {
                around: f.around,
                meridional: f.meridional,
            })
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
        // 归一化后的 prad（与 `gen_unit_shape` 落库的是同一个量化值），不是 raw 长度。
        hash_f32(self.unit_prad(), &mut hasher);
        hash_f32(beta, &mut hasher);
        // 段数枚举整个进键：自带分支与元数，球碟二元组撞不上椭圆碟三元组的前两位。
        self.segment_counts().hash(&mut hasher);
        "dish".hash(&mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        //按比例跳到单位直径圆下
        let dia = self.pdia;
        let h = self.pheig / dia;
        Box::new(Self {
            pheig: h,
            pdia: 1.0,
            prad: self.unit_prad(),
            segments: self.segment_counts(),
            ..Default::default()
        })
    }

    fn get_scaled_vec3(&self) -> Vec3 {
        if self.is_elliptical() {
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
            segments: None,
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
            segments: None,
        }
    }
}

impl From<NamedAttrMap> for Dish {
    fn from(m: NamedAttrMap) -> Self {
        (&m).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dish(dia: f32, height_ratio: f32, prad_ratio: f32) -> Dish {
        Dish {
            pdia: dia,
            pheig: dia * height_ratio,
            prad: dia * prad_ratio,
            ..Default::default()
        }
    }

    /// T041 B1：键里的 `prad` 是按直径归一并量化的值，与落库值同源。比例相同、段数
    /// 相同的两件碟共享一行；raw `prad` 相同而 `prad/pdia` 不同的两件**不得**同键。
    #[test]
    fn the_dish_key_uses_the_normalised_prad_the_unit_row_persists() {
        let a = dish(10.0, 0.25, 0.2);
        let b = dish(12.0, 0.25, 0.2);
        assert_eq!(a.hash_unit_mesh_params(), b.hash_unit_mesh_params());

        let c = dish(12.0, 0.25, 2.0 / 12.0);
        assert_eq!(a.prad, c.prad, "夹具：raw prad 相同");
        assert_ne!(a.hash_unit_mesh_params(), c.hash_unit_mesh_params());

        let unit = a.gen_unit_shape();
        assert_eq!(unit.hash_unit_mesh_params(), a.hash_unit_mesh_params());
        let unit = unit.downcast::<Dish>().unwrap();
        assert_eq!(unit.prad, 0.2);
        assert_eq!(unit.pdia, 1.0);
        assert_eq!(
            unit.segments,
            Some(DishSegments::Elliptical {
                around: 8,
                hub: 2,
                knuckle: 2
            })
        );
    }

    /// T041 B2：椭圆碟 `(100, 2, 2)` 与球碟 `(100, 2)` 前两位逐位相同，枚举带着分支进键，
    /// 两者不得撞成同一个键；球碟的单位行带的是二元组。
    #[test]
    fn the_two_dish_branches_carry_tuples_of_different_arity() {
        let elliptical = dish(2000.0, 5.0 / 2000.0, 0.2);
        let spherical = dish(2000.0, 35.0 / 2000.0, 0.0);
        assert_eq!(
            elliptical.segment_counts(),
            Some(DishSegments::Elliptical {
                around: 100,
                hub: 2,
                knuckle: 2
            })
        );
        assert_eq!(
            spherical.segment_counts(),
            Some(DishSegments::Spherical {
                around: 100,
                meridional: 2
            })
        );
        assert_ne!(
            elliptical.hash_unit_mesh_params(),
            spherical.hash_unit_mesh_params()
        );

        let unit = spherical.gen_unit_shape().downcast::<Dish>().unwrap();
        assert!(!unit.is_elliptical());
        assert_eq!(
            unit.segments,
            Some(DishSegments::Spherical {
                around: 100,
                meridional: 2
            })
        );
    }
}
