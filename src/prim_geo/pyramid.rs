use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::types::attmap::AttrMap;
use glam::{DVec3, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use crate::NamedAttrMap;
#[cfg(feature = "occ")]
use crate::prim_geo::basic::OccSharedShape;
use crate::shape::pdms_shape::{BrepShapeTrait, VerifiedShape};
use bevy_ecs::prelude::*;
#[cfg(feature = "occ")]
use opencascade::primitives::*;

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

pub struct Pyramid {
    pub pbax_pt: Vec3,
    pub pbax_dir: Vec3, //B Axis Direction

    pub pcax_pt: Vec3,
    pub pcax_dir: Vec3, //C Axis Direction

    pub paax_pt: Vec3,
    pub paax_dir: Vec3, //A Axis Direction

    pub pbtp: f32,
    //x top
    pub pctp: f32, //y top

    pub pbbt: f32,
    // x bottom
    pub pcbt: f32, // y bottom

    pub ptdi: f32,
    //dist to top
    pub pbdi: f32, //dist to bottom

    pub pbof: f32,
    // x offset
    pub pcof: f32, // y offset
}

impl Default for Pyramid {
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

impl VerifiedShape for Pyramid {
    fn check_valid(&self) -> bool {
        // dbg!(self);
        let size_flag =
            self.pbtp * self.pctp >= f32::EPSILON || self.pbbt * self.pcbt >= f32::EPSILON;
        if !size_flag {
            return false;
        }
        (self.pbtp >= 0.0 && self.pctp >= 0.0 && self.pbbt >= 0.0 && self.pcbt >= 0.0)
            && ((self.pbtp + self.pctp) > f32::EPSILON || (self.pbbt + self.pcbt) > f32::EPSILON)
    }
}

impl BrepShapeTrait for Pyramid {
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
        let ox = 0.5 * self.pbof as f64;
        let oy = 0.5 * self.pcof as f64;
        let h2 = 0.5 * (self.ptdi - self.pbdi) as f64;

        let mut polys = vec![];
        let mut verts = vec![];

        let mut pts = vec![
            DVec3::new(-tx + ox, -ty + oy, h2),
            DVec3::new(tx + ox, -ty + oy, h2),
            DVec3::new(tx + ox, ty + oy, h2),
            DVec3::new(-tx + ox, ty + oy, h2),
        ];
        // if tx * ty < f64::EPSILON {
        //     verts.push(Vertex::new(DVec3::new(ox, oy, h2)));
        // } else {
        //     polys.push(Wire::from_ordered_points(pts)?);
        // }
        if tx + ty < 0.001 {
            verts.push(Vertex::new(DVec3::new(ox, oy, h2)));
        }
        //todo use line to generate
        // else if tx < 0.001 {
        //     let mut pts = vec![
        //         DVec3::new(tx + ox, -ty + oy, h2),
        //         DVec3::new(tx + ox, ty + oy, h2),
        //     ];
        //     dbg!(&pts);
        //     polys.push(Wire::from_ordered_points(pts)?);
        // } else if ty < 0.001 {
        //     let mut pts = vec![
        //         DVec3::new(-tx + ox, ty + oy, h2),
        //         DVec3::new(tx + ox, ty + oy, h2),
        //     ];
        //     dbg!(&pts);
        //     polys.push(Wire::from_ordered_points(pts)?);
        // }
        else {
            let mut pts = vec![
                DVec3::new(-tx + ox, -ty + oy, h2),
                DVec3::new(tx + ox, -ty + oy, h2),
                DVec3::new(tx + ox, ty + oy, h2),
                DVec3::new(-tx + ox, ty + oy, h2),
            ];
            polys.push(Wire::from_ordered_points(pts)?);
        }

        // dbg!(bx, by);
        if bx + by < 0.001 {
            verts.push(Vertex::new(DVec3::new(-ox, -oy, -h2)));
        }
        // else if bx < 0.001 {
        //     let mut pts = vec![
        //         DVec3::new(bx - ox, -by - oy, -h2),
        //         DVec3::new(bx - ox, by - oy, -h2),
        //     ];
        //     dbg!(&pts);
        //     polys.push(Wire::from_ordered_points(pts)?);
        // } else if by < 0.001 {
        //     let mut pts = vec![
        //         DVec3::new(-bx - ox, by - oy, -h2),
        //         DVec3::new(bx - ox, by - oy, -h2),
        //     ];
        //     dbg!(&pts);
        //     polys.push(Wire::from_ordered_points(pts)?);
        // }
        else {
            let mut pts = vec![
                DVec3::new(-bx - ox, -by - oy, -h2),
                DVec3::new(bx - ox, -by - oy, -h2),
                DVec3::new(bx - ox, by - oy, -h2),
                DVec3::new(-bx - ox, by - oy, -h2),
            ];
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
        "Pyramid".hash(&mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimPyramid(self.clone()))
    }
}

impl From<&AttrMap> for Pyramid {
    fn from(m: &AttrMap) -> Self {
        let xbot = m.get_f32_or_default("XBOT");
        let ybot = m.get_f32_or_default("YBOT");

        let xtop = m.get_f32_or_default("XTOP");
        let ytop = m.get_f32_or_default("YTOP");

        let xoff = m.get_f32_or_default("XOFF");
        let yoff = m.get_f32_or_default("YOFF");

        let height = m.get_f32_or_default("HEIG");

        Pyramid {
            pbax_pt: Default::default(),
            pbax_dir: Vec3::X,
            pcax_pt: Default::default(),
            pcax_dir: Vec3::Y,
            paax_pt: Default::default(),
            paax_dir: Vec3::Z,
            pbtp: xtop,
            pctp: ytop,
            pbbt: xbot,
            pcbt: ybot,
            ptdi: height / 2.0,
            pbdi: -height / 2.0,
            pbof: xoff,
            pcof: yoff,
        }
    }
}

impl From<AttrMap> for Pyramid {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}

impl From<&NamedAttrMap> for Pyramid {
    fn from(m: &NamedAttrMap) -> Self {
        let xbot = m.get_f32_or_default("XBOT");
        let ybot = m.get_f32_or_default("YBOT");

        let xtop = m.get_f32_or_default("XTOP");
        let ytop = m.get_f32_or_default("YTOP");

        let xoff = m.get_f32_or_default("XOFF");
        let yoff = m.get_f32_or_default("YOFF");

        let height = m.get_f32_or_default("HEIG");

        Pyramid {
            pbax_pt: Default::default(),
            pbax_dir: Vec3::X,
            pcax_pt: Default::default(),
            pcax_dir: Vec3::Y,
            paax_pt: Default::default(),
            paax_dir: Vec3::Z,
            pbtp: xtop,
            pctp: ytop,
            pbbt: xbot,
            pcbt: ybot,
            ptdi: height / 2.0,
            pbdi: -height / 2.0,
            pbof: xoff,
            pcof: yoff,
        }
    }
}

impl From<NamedAttrMap> for Pyramid {
    fn from(m: NamedAttrMap) -> Self {
        (&m).into()
    }
}
