//! libgm-compatible primitive tessellation caliber.
//!
//! These rules are identity, not display-quality settings: reusable unit meshes must carry
//! the segment counts computed from the original physical dimensions.

use serde::{Deserialize, Serialize};

pub const FACET_TOL_MM: f64 = 0.5;
pub const MAX_SEGMENTS: i32 = 1000;

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct FacetCaliber {
    pub circumferential: u32,
    pub meridional: u32,
    pub secondary_meridional: u32,
}

impl FacetCaliber {
    pub fn circumferential(segments: i32) -> Self {
        Self {
            circumferential: segments.max(0) as u32,
            meridional: 0,
            secondary_meridional: 0,
        }
    }

    pub fn with_meridians(around: i32, meridional: i32, secondary: i32) -> Self {
        Self {
            circumferential: around.max(0) as u32,
            meridional: meridional.max(0) as u32,
            secondary_meridional: secondary.max(0) as u32,
        }
    }

    pub const fn is_explicit(self) -> bool {
        self.circumferential > 0
    }
}

pub fn chord_tol_is_usable(chord_tol: f64) -> bool {
    chord_tol.is_finite() && chord_tol > 0.0
}

pub fn circle_segments(radius: f64, chord_tol: f64) -> i32 {
    circle_segments_uncapped(radius, chord_tol).min(MAX_SEGMENTS)
}

pub fn circle_segments_uncapped(radius: f64, chord_tol: f64) -> i32 {
    if !(radius > 0.0) {
        return 1;
    }
    let x = (1.0 - (chord_tol / radius).abs()).max(0.0);
    let mut step_deg = x.acos().to_degrees() * 2.0;
    if !(step_deg > 0.0) || step_deg > 45.0 {
        step_deg = 45.0;
    }
    let n = (360.0 / step_deg).ceil() as i32;
    (n + 3) & !3
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PartRev {
    pub segments: i32,
    pub is_full: bool,
    pub start_deg: f64,
    pub end_deg: f64,
}

pub fn part_rev_segments(radius: f64, chord_tol: f64, start_deg: f64, end_deg: f64) -> PartRev {
    let mut start = start_deg;
    let mut end = end_deg;
    if start.is_finite() && end.is_finite() {
        if start >= end {
            end += 360.0 * (((start - end) / 360.0).floor() + 1.0);
        }
        if end > start + 360.0 {
            end -= 360.0 * ((end - start - 360.0) / 360.0).ceil();
        }
    }
    let n_full = circle_segments(radius, chord_tol);
    let sweep = end - start;
    if sweep.abs() <= 1e-6 || (sweep - 360.0).abs() <= 1e-6 {
        return PartRev {
            segments: n_full,
            is_full: true,
            start_deg: 0.0,
            end_deg: 360.0,
        };
    }
    PartRev {
        segments: (f64::from(n_full) * sweep / 360.0).ceil().max(2.0) as i32,
        is_full: false,
        start_deg: start,
        end_deg: end,
    }
}

pub fn cylinder_caliber(radius: f64) -> FacetCaliber {
    FacetCaliber::circumferential(circle_segments(radius, FACET_TOL_MM))
}

pub fn sphere_caliber(radius: f64) -> FacetCaliber {
    let around = circle_segments(radius, FACET_TOL_MM);
    FacetCaliber::with_meridians(around, (around / 2).max(2), 0)
}

pub fn snout_caliber(r_bottom: f64, r_top: f64) -> FacetCaliber {
    FacetCaliber::circumferential(circle_segments(r_bottom.max(r_top), FACET_TOL_MM))
}

pub fn circular_torus_caliber(r_inside: f64, r_outside: f64, sweep_deg: f64) -> FacetCaliber {
    FacetCaliber::with_meridians(
        part_rev_segments(r_outside, FACET_TOL_MM, 0.0, sweep_deg).segments,
        circle_segments((r_outside - r_inside) * 0.5, FACET_TOL_MM),
        0,
    )
}

pub fn rectangular_torus_caliber(r_outside: f64, sweep_deg: f64) -> FacetCaliber {
    FacetCaliber::circumferential(
        part_rev_segments(r_outside, FACET_TOL_MM, 0.0, sweep_deg).segments,
    )
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SphericalDishFacets {
    pub sphere_radius: f64,
    pub polar_angle: f64,
    pub around: i32,
    pub meridional: i32,
}

pub fn spherical_dish_facets(base_radius: f64, height: f64) -> Option<SphericalDishFacets> {
    if !(base_radius > 0.0) || !(height > 0.0) {
        return None;
    }
    let sphere_radius = (base_radius * base_radius / height + height) * 0.5;
    if !(sphere_radius > 0.0) {
        return None;
    }
    let ratio = height / sphere_radius;
    let polar_angle = if ratio.abs() <= 1e-6 {
        (2.0 * ratio).sqrt()
    } else {
        (1.0 - ratio).clamp(-1.0, 1.0).acos()
    };
    let around = circle_segments(
        if height >= base_radius {
            sphere_radius
        } else {
            base_radius
        },
        FACET_TOL_MM,
    );
    let step = std::f64::consts::TAU / f64::from(around.max(1));
    let meridional = (polar_angle / step).ceil().max(1.0) as i32;
    Some(SphericalDishFacets {
        sphere_radius,
        polar_angle,
        around,
        meridional,
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EllipticalDishFacets {
    pub knuckle_radius: f64,
    pub hub_radius: f64,
    pub transition_angle: f64,
    pub around: i32,
    pub hub: i32,
    pub knuckle: i32,
}

pub fn elliptical_dish_facets(base_radius: f64, height: f64) -> Option<EllipticalDishFacets> {
    let (a, h) = (base_radius, height);
    if !(a > 0.0) || !(h > 0.0) {
        return None;
    }
    let s = (a * a + h * h).sqrt();
    let knuckle_radius = h / ((a - h) / s + 1.0);
    if !(knuckle_radius > 0.0) {
        return None;
    }
    let (hub_radius, transition_angle) = if (a - h).abs() <= 1e-6 {
        (knuckle_radius, std::f64::consts::FRAC_PI_4)
    } else {
        let hub_radius = (a * a + h * h - 2.0 * a * knuckle_radius) / (2.0 * (h - knuckle_radius));
        let den = hub_radius - knuckle_radius;
        if den == 0.0 || !hub_radius.is_finite() {
            return None;
        }
        let q = (h - knuckle_radius) / den;
        let angle = if q.abs() > 1e-6 {
            (1.0 - q).clamp(-1.0, 1.0).acos()
        } else {
            (2.0 * q).max(0.0).sqrt()
        };
        (hub_radius, angle)
    };
    if !(hub_radius > 0.0) || !transition_angle.is_finite() {
        return None;
    }
    let around = circle_segments(a, FACET_TOL_MM);
    let theta = transition_angle.to_degrees();
    let mut hub = part_rev_segments(hub_radius, FACET_TOL_MM, 0.0, theta).segments;
    let mut knuckle = part_rev_segments(knuckle_radius, FACET_TOL_MM, theta, 90.0).segments;
    if 2 * (hub + knuckle) > MAX_SEGMENTS {
        if 4 * knuckle > MAX_SEGMENTS {
            knuckle = 250;
        }
        if 4 * hub > MAX_SEGMENTS {
            hub = 250;
        }
    }
    Some(EllipticalDishFacets {
        knuckle_radius,
        hub_radius,
        transition_angle,
        around,
        hub,
        knuckle,
    })
}

pub fn dish_caliber(diameter: f64, height: f64, elliptical: bool) -> Option<FacetCaliber> {
    if elliptical {
        elliptical_dish_facets(diameter * 0.5, height)
            .map(|f| FacetCaliber::with_meridians(f.around, f.hub, f.knuckle))
    } else {
        spherical_dish_facets(diameter * 0.5, height)
            .map(|f| FacetCaliber::with_meridians(f.around, f.meridional, 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_radius_changes_identity_caliber() {
        assert_ne!(cylinder_caliber(0.5), cylinder_caliber(295.0));
        assert_eq!(cylinder_caliber(295.0).circumferential % 4, 0);
    }


    #[test]
    fn cylinder_identity_is_shared_only_at_the_same_caliber() {
        use crate::prim_geo::{LCylinder, SCylinder};
        use crate::shape::pdms_shape::BrepShapeTrait;

        let lc = LCylinder {
            pdia: 200.0,
            ..Default::default()
        };
        let sc = SCylinder {
            pdia: 200.0,
            ..Default::default()
        };
        let larger = LCylinder {
            pdia: 2000.0,
            ..Default::default()
        };
        assert_eq!(lc.hash_unit_mesh_params(), sc.hash_unit_mesh_params());
        assert_ne!(lc.hash_unit_mesh_params(), larger.hash_unit_mesh_params());
        assert_eq!(
            lc.hash_unit_mesh_params(),
            lc.gen_unit_shape().hash_unit_mesh_params()
        );
    }

    #[test]
    fn dish_hash_and_unit_shape_use_the_same_normalized_parameters() {
        use crate::prim_geo::Dish;
        use crate::shape::pdms_shape::BrepShapeTrait;

        for dish in [
            Dish {
                pdia: 1000.0,
                pheig: 250.0,
                prad: 0.0,
                ..Default::default()
            },
            Dish {
                pdia: 1000.0,
                pheig: 250.0,
                prad: 120.0,
                ..Default::default()
            },
        ] {
            assert_eq!(
                dish.hash_unit_mesh_params(),
                dish.gen_unit_shape().hash_unit_mesh_params()
            );
        }
    }

    #[test]
    fn dish_carries_all_required_directions() {
        let spherical = dish_caliber(1000.0, 250.0, false).unwrap();
        let elliptical = dish_caliber(1000.0, 250.0, true).unwrap();
        assert!(spherical.circumferential > 0 && spherical.meridional > 0);
        assert!(
            elliptical.circumferential > 0
                && elliptical.meridional > 0
                && elliptical.secondary_meridional > 0
        );
    }
}
