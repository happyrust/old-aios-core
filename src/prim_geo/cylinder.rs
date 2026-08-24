use crate::parsed_data::geo_params_data::PdmsGeoParam;
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
use crate::shape::pdms_shape::{BrepShapeTrait, PlantMesh, RsVec3, TRI_TOL, VerifiedShape};
use crate::types::attmap::AttrMap;

use crate::NamedAttrMap;
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

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimLCylinder(self.clone()))
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        CYLINDER_GEO_HASH
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(Self::default())
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
        }
    }
}

impl SCylinder {
    /// Core3D `CSG_BasicSLC::getPrimGeom`（3.1 `0x107272D0`）在把 XTSH/YTSH/XBSH/YBSH
    /// 喂给 `gm_CreateSlopeEndedCylinder` 之前，对每个剪切角做**且只做一次**折叠：
    /// `> 90°` 减 180，然后 `< −90°` 加 180。不是取模：折完仍出界的输入（如 271° → 91°）
    /// 由 libgm `GM_SlopeEndCyl::validate`（`0x10030300`，严格 (−90, 90)）响亮拒绝，
    /// 本仓对应 `check_valid` 返回 false。折叠对已在 (−90, 90] 内的值是恒等，所以
    /// 消费方重复调用是安全的。specs/009 T054，证据（gen-model 仓）
    /// `docs/evidence/2026-08-24-ida-occ-retire-audit.md`。
    #[inline]
    pub fn fold_shear_angle_deg(deg: f32) -> f32 {
        let deg = if deg > 90.0 { deg - 180.0 } else { deg };
        if deg < -90.0 { deg + 180.0 } else { deg }
    }

    /// 折叠后的 `(btm, top)` 剪切角。本 crate 内该折叠只有这一处入口：身份哈希、
    /// 落库规范值与几何后端都必须消费它，不得各折各的（T054）。
    #[inline]
    pub fn folded_shear_angles(&self) -> ([f32; 2], [f32; 2]) {
        (
            self.btm_shear_angles.map(Self::fold_shear_angle_deg),
            self.top_shear_angles.map(Self::fold_shear_angle_deg),
        )
    }

    /// 四个剪切角折叠后的规范副本。哈希与落库值取同一个规范值
    /// （2026-08-13 双键 `param` 的教训）。
    #[inline]
    pub fn folded(&self) -> Self {
        let (btm, top) = self.folded_shear_angles();
        let mut c = self.clone();
        c.btm_shear_angles = btm;
        c.top_shear_angles = top;
        c
    }

    #[inline]
    pub fn is_sscl(&self) -> bool {
        // 按折叠后的角判定：180° 这类输入折完是 0，几何上就是直柱，
        // 走 SSCL 全参数哈希只会平白拆散复用。
        let ([bx, by], [tx, ty]) = self.folded_shear_angles();
        bx.abs() > f32::EPSILON
            || by.abs() > f32::EPSILON
            || tx.abs() > f32::EPSILON
            || ty.abs() > f32::EPSILON
    }
}

impl VerifiedShape for SCylinder {
    #[inline]
    fn check_valid(&self) -> bool {
        // 剪切角折叠后必须严格落在 (−90°, 90°)——libgm `GM_SlopeEndCyl::validate`
        // （`0x10030300`）的口径；折完仍出界（Core3D 只折一次，271° → 91°）响亮拒绝。
        // `< 90.0` 的写法同时把 NaN 拒在门外。specs/009 T054。
        let ([bx, by], [tx, ty]) = self.folded_shear_angles();
        let angles_ok = [bx, by, tx, ty].iter().all(|a| a.abs() < 90.0);
        self.pdia > f32::EPSILON && self.phei.abs() > f32::EPSILON && angles_ok
    }
}

impl BrepShapeTrait for SCylinder {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
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

    fn hash_unit_mesh_params(&self) -> u64 {
        if self.is_sscl() {
            let mut hasher = DefaultHasher::new();
            // 哈希折叠后的规范副本：135° 与 −45° 是同一个几何，必须同键；
            // 且与 `gen_unit_shape` 落库的是同一个值（2026-08-13 双键教训）。T054。
            let bytes = bincode::serialize(&self.folded()).unwrap();
            bytes.hash(&mut hasher);
            "SSCL".hash(&mut hasher);
            hasher.finish()
        } else {
            CYLINDER_GEO_HASH
        }
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        if self.is_sscl() {
            return Box::new(self.folded());
        }
        Box::new(Self::default())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sscl(btm: [f32; 2], top: [f32; 2]) -> SCylinder {
        SCylinder {
            pdia: 40.0,
            phei: 100.0,
            btm_shear_angles: btm,
            top_shear_angles: top,
            ..Default::default()
        }
    }

    /// T054：折叠表手抄自 Core3D `CSG_BasicSLC::getPrimGeom`（`0x107272D0`）——
    /// 每角一次 `>90 减 180`、再一次 `<−90 加 180`，不是取模。
    /// 271° 折成 91（出界留给 validate/check_valid 拒），改成取模这条就红。
    #[test]
    fn the_shear_fold_matches_core3d() {
        for (raw, folded) in [
            (135.0_f32, -45.0_f32),
            (-135.0, 45.0),
            (91.0, -89.0),
            (-91.0, 89.0),
            (45.0, 45.0),
            (-45.0, -45.0),
            (180.0, 0.0),
            (-180.0, 0.0),
            (90.0, 90.0),
            (-90.0, -90.0),
            (271.0, 91.0),
        ] {
            assert_eq!(
                SCylinder::fold_shear_angle_deg(raw),
                folded,
                "fold({raw}) 必须是 {folded}"
            );
        }
    }

    /// 135° 与 −45° 折叠后是同一个几何：同一个 `geo_hash`，且 `gen_unit_shape`
    /// 落库的是同一个规范 param（哈希与落库值同源，2026-08-13 双键教训）。
    #[test]
    fn a_foldable_pair_shares_one_identity_and_one_canonical_param() {
        let raw = sscl([135.0, 0.0], [0.0, 30.0]);
        let canonical = sscl([-45.0, 0.0], [0.0, 30.0]);

        assert!(raw.is_sscl() && canonical.is_sscl());
        assert!(raw.check_valid() && canonical.check_valid());
        assert_eq!(
            raw.hash_unit_mesh_params(),
            canonical.hash_unit_mesh_params(),
            "折叠对必须同键"
        );

        let raw_param = raw.gen_unit_shape().convert_to_geo_param().unwrap();
        let canonical_param = canonical.gen_unit_shape().convert_to_geo_param().unwrap();
        assert_eq!(
            serde_json::to_string(&raw_param).unwrap(),
            serde_json::to_string(&canonical_param).unwrap(),
            "同键必须落同一份规范 param，不得两个变体并进一个对象"
        );
    }

    /// 折完仍出界的角响亮拒绝：90°（切面与轴平行）与 271°（折成 91°）都不可建；
    /// NaN 也进不来。libgm `GM_SlopeEndCyl::validate` 的口径是严格 (−90, 90)。
    #[test]
    fn an_angle_still_out_of_range_after_the_fold_is_rejected() {
        assert!(!sscl([90.0, 0.0], [0.0, 0.0]).check_valid());
        assert!(!sscl([271.0, 0.0], [0.0, 0.0]).check_valid());
        assert!(!sscl([f32::NAN, 0.0], [0.0, 0.0]).check_valid());
        assert!(sscl([89.9, 0.0], [0.0, 0.0]).check_valid());
        assert!(
            sscl([135.0, 0.0], [0.0, 0.0]).check_valid(),
            "135° 折成 −45°，可建"
        );
    }

    /// 180° 的剪切角折完是 0：几何上就是直柱，必须走单位圆柱的复用身份，
    /// 不得按 SSCL 全参数哈希白白拆散复用。
    #[test]
    fn a_180_degree_shear_folds_back_to_a_plain_cylinder() {
        let c = sscl([180.0, 180.0], [-180.0, 180.0]);
        assert!(!c.is_sscl());
        assert_eq!(c.hash_unit_mesh_params(), CYLINDER_GEO_HASH);
    }
}
