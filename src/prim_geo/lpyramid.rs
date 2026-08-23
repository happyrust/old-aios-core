use std::collections::hash_map::DefaultHasher;
#[cfg(feature = "truck")]
use truck_modeling::builder::*;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use glam::{DVec3, Vec3};
use serde::{Deserialize, Serialize};
#[cfg(feature = "truck")]
use truck_meshalgo::prelude::*;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::shape::pdms_shape::{BrepShapeTrait, VerifiedShape};
use bevy_ecs::prelude::*;
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
        let size_flag = self.pbtp * self.pctp >= f32::EPSILON || self.pbbt * self.pcbt >= f32::EPSILON;
        if !size_flag {
            return false;
        }
        (self.pbtp >= 0.0 && self.pctp >= 0.0 && self.pbbt >= 0.0 && self.pcbt >= 0.0) &&
            ((self.pbtp + self.pctp) > f32::EPSILON || (self.pbbt + self.pcbt) > f32::EPSILON)
    }
}

//#[typetag::serde]
impl BrepShapeTrait for LPyramid {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    //涵盖的情况，需要考虑，上边只有一条边，和退化成点的情况
    #[cfg(feature = "truck")]
    fn gen_brep_shell(&self) -> Option<truck_modeling::Shell> {

        //todo 需要解决这里的homotopy问题，能兼容point 和 line的情况
        let tx = (self.pbtp as f64 / 2.0).max(0.001);
        let ty = (self.pctp as f64 / 2.0).max(0.001);
        let bx = (self.pbbt as f64 / 2.0).max(0.001);
        let by = (self.pcbt as f64 / 2.0).max(0.001);
        let ox = self.pbof as f64 * Vector2::new(self.pbax_dir.x as _, self.pbax_dir.y as _);
        let oy = self.pcof as f64 * Vector2::new(self.pcax_dir.x as _, self.pcax_dir.y as _);
        let h2 = 0.5 * (self.ptdi - self.pbdi) as f64;

        let offset = ox + oy;
        let offset_3d = Vector3::new(offset.x as _, offset.y as _, 0.0);
        let pts = vec![
            builder::vertex(Point3::new(-tx, -ty, h2) + offset_3d),
            builder::vertex(Point3::new(tx, -ty, h2) + offset_3d),
            builder::vertex(Point3::new(tx, ty, h2) + offset_3d),
            builder::vertex(Point3::new(-tx, ty, h2) + offset_3d),
        ];
        let ets = vec![
            builder::line(&pts[0], &pts[1]),
            builder::line(&pts[1], &pts[2]),
            builder::line(&pts[2], &pts[3]),
            builder::line(&pts[3], &pts[0]),
        ];


        let pts = vec![
            builder::vertex(Point3::new(-bx, -by, -h2)),
            builder::vertex(Point3::new(bx, -by, -h2)),
            builder::vertex(Point3::new(bx, by, -h2)),
            builder::vertex(Point3::new(-bx, by, -h2)),
        ];
        let ebs = vec![
            builder::line(&pts[0], &pts[1]),
            builder::line(&pts[1], &pts[2]),
            builder::line(&pts[2], &pts[3]),
            builder::line(&pts[3], &pts[0]),
        ];

        let mut faces = vec![];
        if let Ok(f) = try_attach_plane(&[Wire::from_iter(&ebs)]) {
            faces.push(f.inverse());
        }
        if let Ok(f) = try_attach_plane(&[Wire::from_iter(&ets)]) {
            faces.push(f);
        }
        let mut shell: Shell = Shell::from(faces);
        shell.push(builder::homotopy(&ebs[0], &ets[0]));
        shell.push(builder::homotopy(&ebs[1], &ets[1]));
        shell.push(builder::homotopy(&ebs[2], &ets[2]));
        shell.push(builder::homotopy(&ebs[3], &ets[3]));

        Some(shell)
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
