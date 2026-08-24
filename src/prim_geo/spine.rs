use bevy_ecs::component::Component;
use glam::{Mat3, Quat, Vec3};

use serde::{Deserialize, Serialize};

use std::f32::consts::PI;

use crate::tool::float_tool::{f32_round_3, vec3_round_3};
use crate::{RefU64, RefnoEnum};
use bevy_transform::prelude::*;

#[derive(
    Component,
    Default,
    Debug,
    Clone,
    Serialize,
    Deserialize,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub enum SpineCurveType {
    #[default]
    UNKNOWN,
    CENT,
    THRU,
    LINE,
}

#[derive(
    Component,
    Debug,
    Default,
    Clone,
    Serialize,
    Deserialize,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct Spine3D {
    pub refno: RefnoEnum,
    pub pt0: Vec3,
    pub pt1: Vec3,
    pub thru_pt: Vec3,
    pub center_pt: Vec3,
    pub cond_pos: Vec3,
    pub radius: f32,
    pub curve_type: SpineCurveType,
    pub preferred_dir: Vec3,
}

impl Spine3D {
    //获取两端的方向，如果是直线段，就是直线的方向，如果是圆弧，就是切线的方向
    pub fn get_dir(&self, start_or_end: bool) -> Vec3 {
        match self.curve_type {
            SpineCurveType::LINE => {
                if start_or_end {
                    (self.pt1 - self.pt0).normalize_or_zero()
                } else {
                    (self.pt0 - self.pt1).normalize_or_zero()
                }
            }
            SpineCurveType::THRU => {
                let center = circum_center(self.pt0, self.pt1, self.thru_pt);
                let vec0 = self.pt0 - center;
                let vec1 = self.pt1 - center;
                let angle = (PI - vec0.angle_between(vec1)) * 2.0;
                let axis = vec1.cross(vec0).normalize();
                let dir = if start_or_end { vec0 } else { vec1 };
                let rot = Quat::from_rotation_arc(Vec3::Z, axis);
                rot.mul_vec3(dir)
            }
            SpineCurveType::CENT => {
                let center = self.center_pt;
                let vec0 = self.pt0 - center;
                let vec1 = self.pt1 - center;
                let angle = (PI - vec0.angle_between(vec1)) * 2.0;
                let axis = vec1.cross(vec0).normalize();
                let dir = if start_or_end { vec0 } else { vec1 };
                let rot = Quat::from_rotation_arc(Vec3::Z, axis);
                rot.mul_vec3(dir)
            }
            SpineCurveType::UNKNOWN => Vec3::Z,
        }
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
pub enum SweepPath3D {
    Line(Line3D),
    SpineArc(Arc3D),
}

impl Default for SweepPath3D {
    fn default() -> Self {
        Self::Line(Line3D::default())
    }
}

impl SweepPath3D {
    pub fn length(&self) -> f32 {
        match self {
            Self::Line(line) => line.length(),
            Self::SpineArc(arc) => arc.angle.abs() * arc.radius,
        }
    }
}

/// `Arc3D` 结构的定义
#[derive(
    Component,
    Debug,
    Clone,
    Default,
    Serialize,
    Deserialize,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct Arc3D {
    /// 弧的中心点
    pub center: Vec3,

    /// 弧的半径
    pub radius: f32,

    /// 弧的角度 (以弧度表示)
    pub angle: f32,

    /// 弧的起始点
    pub start_pt: Vec3,

    /// 弧的绘制方向，如果顺时针则为 `true`
    pub clock_wise: bool,

    /// 弧的轴
    pub axis: Vec3,

    /// 弧的首选轴
    pub pref_axis: Vec3,
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
pub struct Line3D {
    pub start: Vec3,
    pub end: Vec3,
    pub is_spine: bool,
}

impl Line3D {
    #[inline]
    pub fn length(&self) -> f32 {
        self.start.distance(self.end)
    }

    #[inline]
    pub fn get_dir(&self, start_or_end: bool) -> Vec3 {
        if start_or_end {
            (self.end - self.start).normalize_or_zero()
        } else {
            (self.start - self.end).normalize_or_zero()
        }
    }
}

impl Default for Line3D {
    fn default() -> Self {
        Self {
            start: Default::default(),
            end: Vec3::Z,
            is_spine: false,
        }
    }
}

pub fn circum_center(pt0: Vec3, pt1: Vec3, pt2: Vec3) -> Vec3 {
    let vec0 = pt1 - pt0;
    let vec1 = pt2 - pt0;
    let a2 = vec0.dot(vec0);
    let ab = vec0.dot(vec1);
    let b2 = vec1.dot(vec1);
    let det = a2 * b2 - ab * ab;
    let u = (b2 * a2 - ab * b2) / (2.0 * det);
    let v = (-ab * a2 + b2 * a2) / (2.0 * det);
    pt0 + u * vec0 + v * vec1
}

impl Spine3D {
    pub fn generate_paths(&self) -> (Vec<SweepPath3D>, Transform) {
        let mut paths = vec![];
        let mut transform = Transform::IDENTITY;
        let pref_axis = self.preferred_dir.normalize();
        match self.curve_type {
            SpineCurveType::THRU => {
                let center = circum_center(self.pt0, self.pt1, self.thru_pt);
                let vec0 = self.pt0 - self.thru_pt;
                let vec1 = self.pt1 - self.thru_pt;
                let angle = (PI - vec0.angle_between(vec1)) * 2.0;
                let axis = vec1.cross(vec0).normalize();

                let x_axis = (self.pt0 - center).normalize();
                let d = x_axis.dot(Vec3::Z).abs();
                let ref_axis = if approx::abs_diff_eq!(1.0, d) {
                    Vec3::Y
                } else {
                    Vec3::Z
                };
                let y_axis = ref_axis.cross(x_axis).normalize();
                let z_axis = x_axis.cross(y_axis).normalize();
                transform.rotation = Quat::from_mat3(&Mat3::from_cols(x_axis, y_axis, z_axis));
                let arc = Arc3D {
                    center: vec3_round_3(center),
                    radius: f32_round_3(center.distance(self.pt0)),
                    angle,
                    start_pt: self.pt0,
                    clock_wise: axis.z < 0.0,
                    axis,
                    pref_axis,
                };
                // dbg!(&arc);
                transform.translation = center;

                paths.push(SweepPath3D::SpineArc(arc));
            }
            SpineCurveType::CENT => {}
            SpineCurveType::LINE => {
                transform.translation = self.pt0;
                let extru = self.pt1 - self.pt0;
                let extru_dir = extru.normalize();
                let d = extru_dir.dot(pref_axis).abs();
                let ref_axis = if approx::abs_diff_eq!(1.0, d) {
                    Vec3::Y
                } else {
                    pref_axis
                };

                let p_axis = ref_axis.cross(extru_dir).normalize();
                let y_axis = extru_dir.cross(p_axis).normalize();
                // dbg!((p_axis, y_axis, extru_dir));
                transform.rotation =
                    Quat::from_mat3(&glam::f32::Mat3::from_cols(p_axis, y_axis, extru_dir));
                paths.push(SweepPath3D::Line(Line3D {
                    start: Default::default(),
                    end: Vec3::Z * extru.length(),
                    is_spine: true,
                }));
            }
            SpineCurveType::UNKNOWN => {}
        }
        (paths, transform)
    }
}
