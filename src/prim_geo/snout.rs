use crate::NamedAttrMap;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::prim_geo::libgm_discretise::{FACET_TOL_MM, snout_segments};
use crate::shape::pdms_shape::{BrepShapeTrait, VerifiedShape};
use crate::tool::float_tool::{f32_round_3, hash_f32};
use crate::types::attmap::AttrMap;
use bevy_ecs::prelude::*;
use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::f32::EPSILON;
use std::hash::Hash;
use std::hash::Hasher;

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
    pub poff: f32, //offset

    pub btm_on_top: bool,

    /// 绕轴段数，**只有同心（`poff == 0`）的单位行带**：`gen_unit_shape()` 按两端真实
    /// 半径的大者算好写进来（`GM_Snout::calcFacets` `0x1009EA30`）。原件与偏心 Snout
    /// 上恒为 `None`——偏心那支的键是整个结构的 bincode 序列化，这个字段为 `None` 时
    /// 按 serde 规则**不写出**，字节与加字段之前逐位相同，键因此一位不动（T041 B6）。
    /// 读取一律走 [`Self::segment_count`]。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segments: Option<i32>,
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
            btm_on_top: false,
            segments: None,
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
        let half_off = self.pbax_dir.normalize() * (self.poff / 2.0);
        (
            a_dir * self.pbdi + self.paax_pt - half_off,
            a_dir * self.ptdi + self.paax_pt + half_off,
        )
    }

    /// 偏心 Snout 不复用：整个结构按真实尺寸落库，键是它的 bincode 序列化。
    #[inline]
    pub fn is_eccentric(&self) -> bool {
        self.poff.abs() > EPSILON
    }

    /// 绕轴段数：单位行读携带值，原件按**两端真实半径的大者**现算
    /// （`GM_Snout::calcFacets` 喂的就是大者，取错哪一端侧壁都跟相邻圆柱对不上）。
    /// 哈希与落库的单位参数都从这里取（T041 A3）。
    #[inline]
    pub fn segment_count(&self) -> i32 {
        self.segments.unwrap_or_else(|| {
            snout_segments(
                (self.pbdm / 2.0) as f64,
                (self.ptdm / 2.0) as f64,
                FACET_TOL_MM,
            )
        })
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

    fn hash_unit_mesh_params(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        //对于有偏移的，直接不复用，后面看情况再考虑复用
        if self.is_eccentric() {
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
        pheight.hash(&mut hasher);
        // 锥度比是尺度无关量，单靠它 pbdm=100（24 段）与 pbdm=600（56 段）会同键；
        // 段数按两端真实半径的大者算进来（T041 A 组）。
        self.segment_count().hash(&mut hasher);
        "snout".hash(&mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        if self.is_eccentric() {
            Box::new(self.clone())
        } else {
            let segments = Some(self.segment_count());
            if self.ptdm < 0.001 {
                Box::new(Self {
                    ptdi: 0.5,
                    pbdi: -0.5,
                    ptdm: 0.0,
                    pbdm: 1.0,
                    segments,
                    ..Default::default()
                })
            } else if self.pbdm < 0.001 {
                Box::new(Self {
                    ptdi: 0.5,
                    pbdi: -0.5,
                    ptdm: 1.0,
                    pbdm: 0.0,
                    segments,
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
                    segments,
                    ..Default::default()
                })
            }
        }
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        let pheight = (self.ptdi - self.pbdi).abs();
        //有偏心的时候，不缩放
        if self.poff.abs() > f32::EPSILON {
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

    /// T041：同一锥度比下，段数等价类不同就分行（pbdm=100 → 24 段，600 → 56 段），
    /// 同等价类仍共享（pbdm=1 / 2 都撞 8 段下限）；单位行携带段数并重新哈希到同一个键。
    #[test]
    fn a_concentric_snout_key_carries_its_larger_end_segment_class() {
        let snout = |pbdm: f32| LSnout {
            ptdm: pbdm * 0.5,
            pbdm,
            ..Default::default()
        };
        assert_eq!(snout(100.0).segment_count(), 24);
        assert_eq!(snout(600.0).segment_count(), 56);
        assert_ne!(
            snout(100.0).hash_unit_mesh_params(),
            snout(600.0).hash_unit_mesh_params()
        );
        assert_eq!(
            snout(1.0).hash_unit_mesh_params(),
            snout(2.0).hash_unit_mesh_params()
        );

        let unit = snout(600.0).gen_unit_shape();
        assert_eq!(
            unit.hash_unit_mesh_params(),
            snout(600.0).hash_unit_mesh_params(),
            "单位行重新哈希必须回到同一个键"
        );
        let unit = unit.downcast::<LSnout>().unwrap();
        assert_eq!((unit.segments, unit.pbdm, unit.ptdm), (Some(56), 1.0, 0.5));
    }

    /// T041 B6：偏心 Snout 的键是整个结构的 bincode 字节；`segments` 为 `None` 时不写出，
    /// 序列化字节与加字段之前逐位相同（gen-model `t041_b6` 钉的是具体值，这里钉机制）。
    #[test]
    fn an_eccentric_snout_serialises_without_the_segments_field() {
        let eccentric = LSnout {
            ptdi: 57.6,
            pbdi: -57.6,
            ptdm: 84.42,
            pbdm: 66.33,
            poff: 12.06,
            ..Default::default()
        };
        assert!(eccentric.is_eccentric());
        let json = serde_json::to_string(&eccentric).unwrap();
        assert!(!json.contains("segments"), "{json}");

        // 偏心那支的单位形状就是原件：不带段数，字节不变。
        let unit = eccentric.gen_unit_shape().downcast::<LSnout>().unwrap();
        assert_eq!(unit.segments, None);
        assert_eq!(
            bincode::serialize(&*unit).unwrap(),
            bincode::serialize(&eccentric).unwrap()
        );
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
