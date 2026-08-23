use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::prim_geo::facet_caliber::{FacetCaliber, snout_caliber};
#[cfg(feature = "truck")]
use crate::shape::pdms_shape::BrepMathTrait;
use crate::shape::pdms_shape::{BrepShapeTrait, VerifiedShape};
use crate::tool::float_tool::{f32_round_3, hash_f32};
use crate::types::attmap::AttrMap;
use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::f32::EPSILON;
use std::hash::Hash;
use std::hash::Hasher;
#[cfg(feature = "truck")]
use truck_meshalgo::prelude::*;
#[cfg(feature = "truck")]
use truck_modeling::Shell;
#[cfg(feature = "truck")]
use truck_modeling::builder::*;

use crate::NamedAttrMap;
#[cfg(feature = "occ")]
use crate::prim_geo::basic::OccSharedShape;
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
pub struct LSnout {
    pub paax_expr: String,
    pub paax_pt: Vec3,  //A Axis point
    pub paax_dir: Vec3, //A Axis Direction

    pub pbax_expr: String,
    pub pbax_pt: Vec3,  //B Axis point
    pub pbax_dir: Vec3, //B Axis Direction

    pub ptdi: f32, //dist to top
    pub pbdi: f32, //dist to bottom
    pub ptdm: f32, //top diameter
    pub pbdm: f32, //bottom diameter
    pub poff: f32, // local X offset (legacy field)
    #[serde(default)]
    pub poff_y: f32, // local Y offset
    #[serde(default)]
    pub mesh_caliber: FacetCaliber,

    pub btm_on_top: bool,
}

impl Default for LSnout {
    fn default() -> Self {
        Self {
            paax_expr: "Z".to_string(),
            paax_pt: Default::default(),
            paax_dir: Vec3::Z,

            pbax_expr: "X".to_string(),
            pbax_pt: Default::default(),
            pbax_dir: Vec3::X,

            ptdi: 0.5,
            pbdi: -0.5,
            ptdm: 1.0,
            pbdm: 1.0,
            poff: 0.0,
            poff_y: 0.0,
            mesh_caliber: FacetCaliber::default(),
            btm_on_top: false,
        }
    }
}

impl VerifiedShape for LSnout {
    #[inline]
    fn check_valid(&self) -> bool {
        //height 必须 >0， 小于0 的情况直接用变换矩阵
        (self.ptdm >= 0.0 && self.pbdm >= 0.0 && (self.ptdm + self.pbdm) > 0.0)
            && (self.ptdi - self.pbdi) > f32::EPSILON
    }
}

impl LSnout {
    pub fn facet_caliber(&self) -> FacetCaliber {
        if self.mesh_caliber.is_explicit() {
            self.mesh_caliber
        } else {
            snout_caliber((self.pbdm * 0.5) as f64, (self.ptdm * 0.5) as f64)
        }
    }

    /// Centres of the bottom and top circles, in the primitive's local frame.
    ///
    /// The eccentric offset `poff` is **split between the two ends**: the bottom moves
    /// by `-poff/2` along the B axis and the top by `+poff/2`, so their separation is
    /// still the full `poff` but the solid stays centred. That is libgm's convention,
    /// not a symmetry we chose: `GM_Snout::calcFacetsWithoutSurfaces` (libgm 3.1
    /// `0x1009EA30`) emits `r*cos(t) - xShift/2` for the bottom ring and
    /// `r*cos(t) + xShift/2` for the top, and the support function in `calcRange`
    /// (`0x1009E900`) is `(xShift*dx + yShift*dy + height*dz)/2`. `GM_Pyramid` matches.
    ///
    /// Before 2026-08-24 both backends piled the whole offset onto the top ring, which
    /// displaced every eccentric reducer by `poff/2` relative to E3D. Because the two
    /// backends agreed with each other, no dual-backend comparison could see it.
    pub fn end_centers(&self) -> (Vec3, Vec3) {
        let a_dir = self.paax_dir.normalize();
        let b_dir = self.pbax_dir.normalize();
        let c_dir = a_dir.cross(b_dir).normalize_or_zero();
        let half_off = (b_dir * self.poff + c_dir * self.poff_y) * 0.5;
        (
            a_dir * self.pbdi + self.paax_pt - half_off,
            a_dir * self.ptdi + self.paax_pt + half_off,
        )
    }
}

//#[typetag::serde]
impl BrepShapeTrait for LSnout {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    #[inline]
    fn tol(&self) -> f32 {
        //以最小的圆精度为准
        0.005 * ((self.pbdm + self.ptdm) / 2.0).max(1.0)
    }

    #[cfg(feature = "occ")]
    fn gen_occ_shape(&self) -> anyhow::Result<OccSharedShape> {
        let rt = self.ptdm / 2.0;
        let rb = self.pbdm / 2.0;

        let a_dir = self.paax_dir.normalize();
        let (p0, p1) = self.end_centers();

        let mut circles = vec![];
        let mut verts = vec![];
        if self.pbdm < f32::EPSILON {
            verts.push(Vertex::new(p0.as_dvec3()));
        } else {
            let circle = Wire::circle(rb as _, p0.as_dvec3(), a_dir.as_dvec3());
            circles.push(circle);
        }

        if self.ptdm < f32::EPSILON {
            verts.push(Vertex::new(p1.as_dvec3()));
        } else {
            let circle = Wire::circle(rt as _, p1.as_dvec3(), a_dir.as_dvec3());
            circles.push(circle);
        }

        Ok(OccSharedShape::new(
            Solid::loft_with_points(circles.iter(), verts.iter())?.into(),
        ))
    }

    #[cfg(feature = "truck")]
    fn gen_brep_shell(&self) -> Option<Shell> {
        let rt = (self.ptdm / 2.0).max(0.01);
        let rb = (self.pbdm / 2.0).max(0.01);

        let a_dir = self.paax_dir.normalize();
        let b_dir = self.pbax_dir.normalize();
        let (p0, p1) = self.end_centers();
        let p2 = b_dir * rt + p1;
        let p3 = b_dir * rb + p0;
        let v2 = builder::vertex(p2.point3());
        let v3 = builder::vertex(p3.point3());

        //todo 表达cone的情况
        let mut is_cone = false;
        if self.ptdm * self.pbdm < 0.001 {
            is_cone = true;
        }
        // dbg!(is_cone);

        let rot_axis = a_dir.vector3();
        let mut circle1 = builder::rsweep(&v3, p0.point3(), rot_axis, Rad(7.0));
        let c1 = circle1.clone();

        let mut circle2 = builder::rsweep(&v2, p1.point3(), rot_axis, Rad(7.0));
        let c2 = circle2.clone();

        // dbg!((circle1.len(), circle2.len()));

        let new_wire_1 = circle1.split_off((0.5 * circle1.len() as f32) as usize);
        let new_wire_2 = circle2.split_off((0.5 * circle2.len() as f32) as usize);
        // dbg!((circle1.len(), circle2.len()));
        // dbg!((new_wire_1.len(), new_wire_2.len()));
        let shell1 = builder::try_wire_homotopy(&new_wire_1, &new_wire_2).ok()?;
        let shell2 = builder::try_wire_homotopy(&circle1, &circle2).ok()?;

        if let Ok(disk1) = builder::try_attach_plane(&vec![c1.inverse()]) {
            if let Ok(disk2) = builder::try_attach_plane(&vec![c2]) {
                let mut shell = Shell::from(vec![disk1, disk2]);
                shell.extend(shell1);
                shell.extend(shell2);
                return Some(shell);
            }
        }
        None
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        //对于有偏移的，直接不复用，后面看情况再考虑复用
        if self.poff.abs() > f32::EPSILON || self.poff_y.abs() > f32::EPSILON {
            let bytes = bincode::serialize(self).unwrap();
            let mut hasher = DefaultHasher::default();
            bytes.hash(&mut hasher);
            return hasher.finish();
        }
        let pheight = (self.ptdi - self.pbdi) > 0.0;
        let alpha = if self.pbdm != 0.0 {
            self.ptdm / self.pbdm
        } else {
            0.0
        };
        hash_f32(alpha, &mut hasher);
        self.facet_caliber().hash(&mut hasher);
        pheight.hash(&mut hasher);
        "snout".hash(&mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        if self.poff.abs() > f32::EPSILON || self.poff_y.abs() > f32::EPSILON {
            Box::new(self.clone())
        } else {
            if self.ptdm < 0.001 {
                Box::new(Self {
                    ptdi: 0.5,
                    pbdi: -0.5,
                    ptdm: 0.0,
                    pbdm: 1.0,
                    mesh_caliber: self.facet_caliber(),
                    ..Default::default()
                })
            } else if self.pbdm < 0.001 {
                Box::new(Self {
                    ptdi: 0.5,
                    pbdi: -0.5,
                    ptdm: 1.0,
                    pbdm: 0.0,
                    mesh_caliber: self.facet_caliber(),
                    ..Default::default()
                })
            } else {
                // The reusable mesh id hashes this ratio at three decimal places.
                // Persist the very same canonical value; otherwise equivalent CATA
                // definitions such as 5/9 and their copied rounded form share an id
                // but produce two different `inst_geo.param` rows.
                let ptdm = f32_round_3(self.ptdm / self.pbdm);
                Box::new(Self {
                    ptdi: 0.5,
                    pbdi: -0.5,
                    ptdm,
                    pbdm: 1.0,
                    mesh_caliber: self.facet_caliber(),
                    ..Default::default()
                })
            }
        }
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        let pheight = (self.ptdi - self.pbdi).abs();
        //有偏心的时候，不缩放
        if self.poff.abs() > f32::EPSILON || self.poff_y.abs() > f32::EPSILON {
            Vec3::ONE
        } else {
            if self.pbdm < 0.001 {
                Vec3::new(self.ptdm, self.ptdm, pheight)
            } else {
                Vec3::new(self.pbdm, self.pbdm, pheight)
            }
        }
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimLSnout(self.clone()))
    }
}

impl From<&AttrMap> for LSnout {
    fn from(m: &AttrMap) -> Self {
        let h = m.get_f32("HEIG").unwrap_or_default();
        LSnout {
            ptdi: h / 2.0,
            pbdi: -h / 2.0,
            ptdm: m.get_f32("DTOP").unwrap_or_default(),
            pbdm: m.get_f32("DBOT").unwrap_or_default(),
            poff: m.get_f32("XOFF").unwrap_or_default(),
            poff_y: m.get_f32("YOFF").unwrap_or_default(),
            ..Default::default()
        }
    }
}

impl From<AttrMap> for LSnout {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}

impl From<&NamedAttrMap> for LSnout {
    fn from(m: &NamedAttrMap) -> Self {
        let h = m.get_f32("HEIG").unwrap_or_default();
        LSnout {
            ptdi: h / 2.0,
            pbdi: -h / 2.0,
            ptdm: m.get_f32("DTOP").unwrap_or_default(),
            pbdm: m.get_f32("DBOT").unwrap_or_default(),
            poff: m.get_f32("XOFF").unwrap_or_default(),
            poff_y: m.get_f32("YOFF").unwrap_or_default(),
            ..Default::default()
        }
    }
}

impl From<NamedAttrMap> for LSnout {
    fn from(m: NamedAttrMap) -> Self {
        (&m).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reusable_snout_unit_param_matches_rounded_hash_identity() {
        let from_ratio = LSnout {
            ptdi: 1.0,
            pbdi: 0.0,
            ptdm: 5.0,
            pbdm: 9.0,
            ..Default::default()
        };
        let copied_rounded = LSnout {
            ptdi: 1.0,
            pbdi: 0.0,
            ptdm: 0.555_555_5,
            pbdm: 1.0,
            ..Default::default()
        };

        assert_eq!(
            from_ratio.hash_unit_mesh_params(),
            copied_rounded.hash_unit_mesh_params()
        );
        let left = from_ratio.gen_unit_shape().downcast::<LSnout>().unwrap();
        let right = copied_rounded
            .gen_unit_shape()
            .downcast::<LSnout>()
            .unwrap();
        assert_eq!(left.ptdm, right.ptdm);
        assert_eq!(left.ptdm, 0.556);
    }

    /// The eccentric offset is split between the two ends, not piled onto the top.
    ///
    /// The discriminating half is the assertion on the *bottom* centre: the pre-2026-08-24
    /// code left it exactly on the axis. Volume is unaffected either way (Cavalieri), and
    /// the two ends stay a full `poff` apart, so nothing but the absolute position tells
    /// the two conventions apart -- which is why this needs libgm as the reference rather
    /// than the other backend. See `end_centers` for the addresses.
    #[test]
    fn the_eccentric_offset_is_split_between_the_two_ends() {
        // Dimensions of the one eccentric reducer found in the live library.
        let snout = LSnout {
            ptdi: 57.6,
            pbdi: -57.6,
            ptdm: 84.42,
            pbdm: 66.33,
            poff: 12.06,
            ..Default::default()
        };
        let (bottom, top) = snout.end_centers();

        assert!(
            bottom.abs_diff_eq(Vec3::new(-6.03, 0.0, -57.6), 1e-4),
            "bottom centre {bottom} is not at -poff/2 along B -- offset back on the top only?"
        );
        assert!(
            top.abs_diff_eq(Vec3::new(6.03, 0.0, 57.6), 1e-4),
            "top centre {top} is not at +poff/2 along B"
        );
        assert!(
            (top - bottom).abs_diff_eq(Vec3::new(12.06, 0.0, 115.2), 1e-4),
            "splitting the offset must not halve the eccentricity: {}",
            top - bottom
        );

        // A concentric snout must stay exactly on the axis.
        let straight = LSnout {
            poff: 0.0,
            ..snout.clone()
        };
        let (b0, t0) = straight.end_centers();
        assert!(b0.abs_diff_eq(Vec3::new(0.0, 0.0, -57.6), 1e-4));
        assert!(t0.abs_diff_eq(Vec3::new(0.0, 0.0, 57.6), 1e-4));
    }
}
