use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::prim_geo::facet_caliber::{FacetCaliber, cylinder_caliber};
use bevy_ecs::prelude::*;
use bevy_transform::prelude::Transform;
use glam::{DMat4, DVec3, Mat4, Vec3};
use nom::Parser;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::f64::consts::FRAC_PI_2;
use std::hash::Hash;
use std::hash::Hasher;
use std::sync::Arc;

use crate::prim_geo::basic::*;
use crate::prim_geo::helper::cal_ref_axis;
#[cfg(feature = "truck")]
use crate::shape::pdms_shape::BrepMathTrait;
use crate::shape::pdms_shape::{BrepShapeTrait, PlantMesh, RsVec3, TRI_TOL, VerifiedShape};
use crate::types::attmap::AttrMap;

use crate::NamedAttrMap;
#[cfg(feature = "occ")]
use opencascade::primitives::*;
#[cfg(feature = "occ")]
use opencascade::workplane::Workplane;
#[cfg(feature = "truck")]
use truck_modeling::*;

///元件库里的LCylinder
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
pub struct LCylinder {
    pub paxi_expr: String,
    pub paxi_pt: Vec3,
    //A Axis point
    pub paxi_dir: Vec3, //A Axis Direction

    pub pbdi: f32,
    pub ptdi: f32,
    //diameter
    pub pdia: f32,
    pub negative: bool,
    #[serde(default)]
    pub mesh_caliber: FacetCaliber,
}

impl Default for LCylinder {
    fn default() -> Self {
        LCylinder {
            paxi_expr: "Z".to_string(),
            paxi_pt: Default::default(),
            paxi_dir: Vec3::Z,
            pbdi: -0.5,
            ptdi: 0.5,
            pdia: 1.0,
            negative: false,
            mesh_caliber: FacetCaliber::default(),
        }
    }
}

impl LCylinder {
    pub fn facet_caliber(&self) -> FacetCaliber {
        if self.mesh_caliber.is_explicit() {
            self.mesh_caliber
        } else {
            cylinder_caliber((self.pdia * 0.5) as f64)
        }
    }
}

impl VerifiedShape for LCylinder {
    fn check_valid(&self) -> bool {
        self.pdia > f32::EPSILON && (self.pbdi - self.ptdi).abs() > f32::EPSILON
    }
}

pub fn gen_unit_cylinder() -> PlantMesh {
    let segments = 1;
    let resolution = 36;
    let height = 1.0;
    let radius = 0.5;
    let num_rings = segments + 1;
    let num_vertices = resolution * 2 + num_rings * (resolution + 1);
    let num_faces = resolution * (num_rings - 2);
    let num_indices = (2 * num_faces + 2 * (resolution - 1) * 2) * 3;
    let mut vertices: Vec<Vec3> = Vec::with_capacity(num_vertices as usize);
    let mut normals: Vec<Vec3> = Vec::with_capacity(num_vertices as usize);
    // let mut uvs = Vec::with_capacity(num_vertices as usize);
    let mut indices = Vec::with_capacity(num_indices as usize);

    let step_theta = std::f32::consts::TAU / resolution as f32;
    let step_z = height / segments as f32;

    // rings

    for ring in 0..num_rings {
        let z = 0.0 + ring as f32 * step_z;

        for segment in 0..=resolution {
            let theta = segment as f32 * step_theta;
            let (sin, cos) = theta.sin_cos();

            vertices.push([radius * cos, radius * sin, z].into());
            normals.push([cos, sin, 0.0].into());
            // uvs.push([
            //     segment as f32 / resolution as f32,
            //     ring as f32 / segments as f32,
            // ]);
        }
    }

    // barrel skin

    for i in 0..segments {
        let ring = i * (resolution + 1);
        let next_ring = (i + 1) * (resolution + 1);

        for j in 0..resolution {
            indices.extend_from_slice(&[
                ring + j + 1,
                next_ring + j,
                ring + j,
                ring + j + 1,
                next_ring + j + 1,
                next_ring + j,
            ]);
        }
    }

    // caps

    let mut build_cap = |top: bool| {
        let offset = vertices.len() as u32;
        let (z, normal_z, winding) = if top {
            (height, 1., (1, 0))
        } else {
            (0.0, -1., (0, 1))
        };

        for i in 0..resolution {
            let theta = i as f32 * step_theta;
            let (sin, cos) = theta.sin_cos();

            vertices.push([cos * radius, sin * radius, z].into());
            normals.push([0.0, 0.0, normal_z].into());
        }

        for i in 1..(resolution - 1) {
            indices.extend_from_slice(&[offset, offset + i + winding.1, offset + i + winding.0]);
        }
    };

    // top

    build_cap(true);
    build_cap(false);

    PlantMesh {
        vertices,
        normals,
        indices,
        wire_vertices: vec![],
        aabb: None,
    }
}

impl BrepShapeTrait for LCylinder {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    #[cfg(feature = "truck")]
    fn gen_brep_shell(&self) -> Option<truck_modeling::Shell> {
        if !self.check_valid() {
            return None;
        }

        let dir = self.paxi_dir.normalize();
        let r = self.pdia / 2.0;
        let c_pt = dir * self.pbdi + self.paxi_pt;
        let center = c_pt.point3();
        let ref_axis = cal_ref_axis(&dir);
        let pt0 = c_pt + ref_axis * r;
        let mut ext_len = self.ptdi - self.pbdi;
        let mut ext_dir = dir.vector3();
        if ext_len < 0.0 {
            ext_dir = -ext_dir;
            ext_len = -ext_len;
        }
        let v = builder::vertex(pt0.point3());
        let w = builder::rsweep(&v, center, ext_dir, Rad(7.0));
        let f = builder::try_attach_plane(&[w]).unwrap();
        let mut s = builder::tsweep(&f, ext_dir * ext_len as f64).into_boundaries();
        s.pop()
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimLCylinder(self.clone()))
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        "cylinder".hash(&mut hasher);
        self.facet_caliber().hash(&mut hasher);
        hasher.finish()
    }

    /// 如果是常规的基本体生成，直接跳过, 复用已经生成好的
    #[cfg(feature = "occ")]
    fn gen_occ_shape(&self) -> anyhow::Result<OccSharedShape> {
        if !self.check_valid() {
            return Err(anyhow::anyhow!("Not valid LCylinder"));
        }

        Ok(CYLINDER_SHAPE.clone())
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(Self {
            mesh_caliber: self.facet_caliber(),
            ..Self::default()
        })
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        Vec3::new(self.pdia, self.pdia, (self.pbdi - self.ptdi).abs())
    }

    ///直接通过基本体的参数，生成模型
    fn gen_csg_mesh(&self) -> Option<PlantMesh> {
        Some(gen_unit_cylinder())
    }

    fn need_use_csg(&self) -> bool {
        false
    }
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
pub struct SCylinder {
    pub paxi_expr: String,
    pub paxi_pt: Vec3,
    pub paxi_dir: Vec3,
    //dist to bottom
    pub phei: f32,
    // height
    pub pdia: f32,
    //diameter
    pub btm_shear_angles: [f32; 2],
    // x shear
    pub top_shear_angles: [f32; 2],
    // y shear
    pub negative: bool,
    pub center_in_mid: bool,
    #[serde(default)]
    pub mesh_caliber: FacetCaliber,
}

impl Default for SCylinder {
    fn default() -> Self {
        Self {
            paxi_expr: "Z".to_string(),
            paxi_dir: Vec3::Z,
            paxi_pt: Default::default(),
            phei: 1.0,
            pdia: 1.0,
            btm_shear_angles: [0.0f32; 2],
            top_shear_angles: [0.0f32; 2],
            negative: false,
            center_in_mid: false,
            mesh_caliber: FacetCaliber::default(),
        }
    }
}

impl SCylinder {
    pub fn facet_caliber(&self) -> FacetCaliber {
        if self.mesh_caliber.is_explicit() {
            self.mesh_caliber
        } else {
            cylinder_caliber((self.pdia * 0.5) as f64)
        }
    }

    #[inline]
    pub fn is_sscl(&self) -> bool {
        self.btm_shear_angles[0].abs() > f32::EPSILON
            || self.btm_shear_angles[1].abs() > f32::EPSILON
            || self.top_shear_angles[0].abs() > f32::EPSILON
            || self.top_shear_angles[1].abs() > f32::EPSILON
    }
}

impl VerifiedShape for SCylinder {
    #[inline]
    fn check_valid(&self) -> bool {
        self.pdia > f32::EPSILON && self.phei.abs() > f32::EPSILON
    }
}

impl BrepShapeTrait for SCylinder {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    #[cfg(feature = "truck")]
    fn gen_brep_shell(&self) -> Option<truck_modeling::Shell> {
        let dir = self.paxi_dir.normalize();
        let dir = Vec3::Z;
        let r = self.pdia / 2.0;
        let c_pt = Vec3::ZERO;
        let center = c_pt.point3();
        let ref_axis = cal_ref_axis(&dir);
        let pt0 = c_pt + ref_axis * r;
        let ext_len = self.phei as f64;
        let ext_dir = dir.vector3();
        let mut reverse_dir = false;
        if ext_len < 0.0 {
            reverse_dir = true;
        }
        // dbg!(ext_dir);
        let v = builder::vertex(pt0.point3());
        let origin_w = builder::rsweep(&v, center, ext_dir, Rad(7.0));

        //还是要和extrude 区分出来
        let scale_x = 1.0 / self.btm_shear_angles[0].to_radians().cos() as f64;
        let scale_y = 1.0 / self.btm_shear_angles[1].to_radians().cos() as f64;
        // dbg!(&self.btm_shear_angles);
        let transform_btm =
            Matrix4::from_angle_y(-Rad(self.btm_shear_angles[0].to_radians() as f64))
                * Matrix4::from_angle_x(Rad(self.btm_shear_angles[1].to_radians() as f64))
                * Matrix4::from_nonuniform_scale(scale_x, scale_y, 1.0);

        // dbg!(&self.top_shear_angles);
        let scale_x = 1.0 / self.top_shear_angles[0].to_radians().cos() as f64;
        let scale_y = 1.0 / self.top_shear_angles[1].to_radians().cos() as f64;
        let transform_top = Matrix4::from_translation(ext_dir * ext_len as f64)
            * Matrix4::from_angle_y(-Rad(self.top_shear_angles[0].to_radians() as f64))
            * Matrix4::from_angle_x(Rad(self.top_shear_angles[1].to_radians() as f64))
            * Matrix4::from_nonuniform_scale(scale_x, scale_y, 1.0);

        let mut w_s = builder::transformed(&origin_w, transform_btm);
        let mut w_e = builder::transformed(&origin_w, transform_top);
        if let Ok(mut f) = builder::try_attach_plane(&[w_s.clone()]) {
            let mut f_e = builder::try_attach_plane(&[w_e.clone()]).unwrap().inverse();
            // dbg!(reverse_dir);
            if !reverse_dir {
                f = f.inverse();
                f_e = f_e.inverse();
            }
            let h_w_s = w_s.split_off(w_s.len() / 2);
            let h_w_e = w_e.split_off(w_e.len() / 2);
            let face1 = builder::homotopy(w_s.front().unwrap(), &w_e.front().unwrap());
            let face2 = builder::homotopy(h_w_s.front().unwrap(), &h_w_e.front().unwrap());
            let shell = vec![f, f_e, face1, face2].into();
            return Some(shell);
        }
        None
    }

    ///获得关键点
    fn key_points(&self) -> Vec<RsVec3> {
        if self.is_sscl() {
            vec![Vec3::ZERO.into(), (Vec3::Z * self.phei.abs()).into()]
        } else {
            vec![Vec3::ZERO.into(), (Vec3::Z * 1.0).into()]
        }
    }

    ///引用限制大小
    fn apply_limit_by_size(&mut self, l: f32) {
        self.phei = self.phei.min(l);
        // dbg!(self.phei);
        self.pdia = self.pdia.min(l);
    }

    #[cfg(feature = "occ")]
    fn gen_occ_shape(&self) -> anyhow::Result<OccSharedShape> {
        if self.is_sscl() {
            let dir = DVec3::Z;
            let r = self.pdia as f64 / 2.0;
            let ext_len = self.phei as f64;
            let mut circle = Workplane::xy().circle(0.0, 0.0, r)?;

            //还是要和extrude 区分出来
            let scale_x = 1.0 / self.btm_shear_angles[0].to_radians().cos() as f64;
            let scale_y = 1.0 / self.btm_shear_angles[1].to_radians().cos() as f64;
            let scale_mat = DMat4::from_scale(DVec3::new(scale_x, scale_y, 1.0));
            // dbg!(&self.btm_shear_angles);
            let transform_btm =
                DMat4::from_axis_angle(DVec3::Y, -(self.btm_shear_angles[0].to_radians() as f64))
                    * DMat4::from_axis_angle(
                        DVec3::X,
                        (self.btm_shear_angles[1].to_radians() as f64),
                    )
                    * scale_mat;

            // dbg!(&self.top_shear_angles);
            let scale_x = 1.0 / self.top_shear_angles[0].to_radians().cos() as f64;
            let scale_y = 1.0 / self.top_shear_angles[1].to_radians().cos() as f64;
            let scale_mat = DMat4::from_scale(DVec3::new(scale_x, scale_y, 1.0));
            let transform_top = DMat4::from_translation(dir * ext_len as f64)
                * DMat4::from_axis_angle(DVec3::Y, -(self.top_shear_angles[0].to_radians() as f64))
                * DMat4::from_axis_angle(DVec3::X, (self.top_shear_angles[1].to_radians() as f64))
                * scale_mat;
            let btm_circe = circle.transformed_by_gmat(&transform_btm)?;
            let top_circle = circle.transformed_by_gmat(&transform_top)?;

            Ok(OccSharedShape::new(
                Solid::loft([btm_circe, top_circle].iter()).into(),
            ))
        } else {
            Ok(CYLINDER_SHAPE.clone())
        }
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        if self.is_sscl() {
            let mut hasher = DefaultHasher::new();
            let bytes = bincode::serialize(self).unwrap();
            bytes.hash(&mut hasher);
            "SSCL".hash(&mut hasher);
            hasher.finish()
        } else {
            let mut hasher = DefaultHasher::new();
            "cylinder".hash(&mut hasher);
            self.facet_caliber().hash(&mut hasher);
            hasher.finish()
        }
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        if self.is_sscl() {
            return Box::new(self.clone());
        }
        Box::new(Self {
            mesh_caliber: self.facet_caliber(),
            ..Self::default()
        })
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        if self.is_sscl() {
            Vec3::new(1.0, 1.0, 1.0)
        } else {
            Vec3::new(self.pdia, self.pdia, self.phei.abs())
        }
    }

    #[inline]
    fn get_trans(&self) -> Transform {
        Transform {
            rotation: Default::default(),
            translation: if self.center_in_mid {
                Vec3::new(0.0, 0.0, -self.phei / 2.0)
            } else {
                Vec3::ZERO
            },
            scale: self.get_scaled_vec3(),
        }
    }

    #[inline]
    fn tol(&self) -> f32 {
        if self.is_sscl() {
            0.004 * (self.pdia.max(1.0))
        } else {
            TRI_TOL
        }
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimSCylinder(self.clone()))
    }

    ///直接通过基本体的参数，生成模型
    fn gen_csg_mesh(&self) -> Option<PlantMesh> {
        Some(gen_unit_cylinder())
    }

    fn need_use_csg(&self) -> bool {
        // !self.is_sscl()
        false
    }
}

impl From<&AttrMap> for SCylinder {
    fn from(m: &AttrMap) -> Self {
        let phei = m.get_f32_or_default("HEIG");
        let pdia = m.get_f32_or_default("DIAM");
        SCylinder {
            paxi_expr: "Z".to_string(),
            paxi_pt: Default::default(),
            paxi_dir: Vec3::Z,
            phei,
            pdia,
            negative: false,
            center_in_mid: true,
            ..Default::default()
        }
    }
}

impl From<AttrMap> for SCylinder {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}

impl From<&NamedAttrMap> for SCylinder {
    fn from(m: &NamedAttrMap) -> Self {
        let phei = m.get_f32_or_default("HEIG");
        let pdia = m.get_f32_or_default("DIAM");
        SCylinder {
            paxi_expr: "Z".to_string(),
            paxi_pt: Default::default(),
            paxi_dir: Vec3::Z,
            phei,
            pdia,
            negative: false,
            center_in_mid: true,
            ..Default::default()
        }
    }
}

impl From<NamedAttrMap> for SCylinder {
    fn from(m: NamedAttrMap) -> Self {
        (&m).into()
    }
}
