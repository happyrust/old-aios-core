use crate::parsed_data::CateAxisParam;
use bevy_math::prelude::{Quat, Vec3};
use glam::{Mat3, Vec2};
use std::f32::consts::PI;

#[inline]
pub fn cal_ref_axis(v: &Vec3) -> Vec3 {
    let v = v.normalize();
    let a = v.x.abs();
    let b = v.y.abs();
    let c = v.z.abs();
    let mut dx = Vec3::new(1.0f32, 0.0, 0.0);
    if b <= a && b <= c {
        dx = Vec3::new(-v.z as f32, 0.0, v.x);
    } else if a <= b && a <= c {
        dx = Vec3::new(0.0, -v.z, v.y);
    } else {
        dx = Vec3::new(-v.y, v.x, 0.0);
    }
    dx
}

///针对torus的角度求解
pub fn rotate_from_vec3_to_vec3(dir: Vec3, from: Vec3, to: Vec3) -> Quat {
    let mut angle = from.angle_between(to);
    if angle.abs() < 1.0e-3 || (angle.abs() - std::f32::consts::PI).abs() < 1.0e-3 {
        let rotation_angle = angle;
        let ref_axis = dir;
        Quat::from_axis_angle(from.cross(ref_axis).normalize(), rotation_angle)
    } else {
        // 不平行
        let a1 = dir.angle_between(from);
        let a2 = dir.angle_between(to);

        let mut z_dir = from.cross(to).normalize();
        if ((a1 + a2) - angle).abs() > 1.0e-3 {
            angle = std::f32::consts::TAU - angle;
            z_dir = -z_dir;
        }
        Quat::from_axis_angle(z_dir, angle)
    }
}

pub fn quad_indices(
    indices: &mut Vec<usize>,
    l: &mut usize,
    o: usize,
    v0: usize,
    v1: usize,
    v2: usize,
    v3: usize,
) {
    indices.push(o + v0);
    indices.push(o + v1);
    indices.push(o + v2);
    indices.push(o + v2);
    indices.push(o + v3);
    indices.push(o + v0);
    *l += 6;
}

// 定义一个表示射线的结构体
#[derive(Debug, Default)]
struct Ray {
    origin: Vec3,
    direction: Vec3,
}

impl Ray {
    fn new(origin: Vec3, direction: Vec3) -> Self {
        Ray {
            origin,
            direction: direction.normalize(),
        }
    }
}

// 计算两个射线的最近点
fn closest_point_between_rays(ray1: &Ray, ray2: &Ray, epsilon: f32) -> Option<(Vec3, Vec3)> {
    let v1 = ray1.direction;
    let v2 = ray2.direction;
    let w0 = ray1.origin - ray2.origin;

    let a = v1.dot(v1);
    let b = v1.dot(v2);
    let c = v2.dot(v2);
    let d = v1.dot(w0);
    let e = v2.dot(w0);

    let denom = a * c - b * b;

    // 如果射线几乎平行，返回 None
    if denom.abs() < epsilon {
        return None;
    }

    let t1 = (b * e - c * d) / denom;
    let t2 = (a * e - b * d) / denom;

    let point1 = ray1.origin + v1 * t1;
    let point2 = ray2.origin + v2 * t2;

    Some((point1, point2))
}

// 计算两个射线的交点
fn ray_intersection(ray1: &Ray, ray2: &Ray, epsilon: f32) -> Option<Vec3> {
    if let Some((point1, point2)) = closest_point_between_rays(ray1, ray2, epsilon) {
        // 如果两个点足够接近，我们认为它们相交
        if (point1 - point2).length() < epsilon {
            return Some((point1 + point2) * 0.5); // 返回中点
        }
    }
    None
}

#[derive(Debug, Default)]
struct Ray2D {
    origin: Vec2,
    direction: Vec2,
}

impl Ray2D {
    fn new(origin: Vec2, direction: Vec2) -> Self {
        Self { origin, direction }
    }

    fn are_parallel(&self, other: &Self) -> bool {
        self.direction.perp_dot(other.direction).abs() < f32::EPSILON
    }

    fn perpendicular_distance(&self, other: &Self) -> Option<f32> {
        // if !self.are_parallel(other) {
        //     return None;
        // }

        let origin_diff = other.origin - self.origin;
        let direction_perp = self.direction.perp();

        Some(origin_diff.dot(direction_perp).abs() / self.direction.length())
    }

    fn intersect(&self, other: &Self) -> Option<Vec2> {
        let p = self.origin;
        let r = self.direction;
        let q = other.origin;
        let s = other.direction;

        let r_cross_s = r.perp_dot(s);
        let q_minus_p = q - p;
        let qmp_cross_r = q_minus_p.perp_dot(r);

        if r_cross_s == 0.0 {
            // r and s are parallel
            return None;
        }

        let t = q_minus_p.perp_dot(s) / r_cross_s;
        let u = qmp_cross_r / r_cross_s;

        if t >= 0.0 && u >= 0.0 {
            Some(p + t * r)
        } else {
            None
        }
    }
}

#[derive(Default, Debug)]
pub struct RotateInfo {
    pub center: Vec3,
    pub angle: f32,
    pub rot_axis: Vec3,
    pub radius: f32,
}

impl RotateInfo {
    pub fn cal_rotate_info(
        a_dir: Vec3,
        a_pt: Vec3,
        b_dir: Vec3,
        b_pt: Vec3,
        default_r: f32,
    ) -> Option<RotateInfo> {
        let mut rotate_info = RotateInfo::default();
        //应该是从pb_dir 旋转到pb_dir
        let pa_dir = a_dir.normalize();
        let pb_dir = -b_dir.normalize();
        let dist = (a_pt - b_pt).length();
        let x_dir = (a_pt - b_pt).normalize();
        if x_dir.is_nan() {
            let rot_axis = pb_dir.cross(pa_dir).normalize();
            if rot_axis.is_nan() {
                return None;
            }
            return Some(RotateInfo {
                center: Default::default(),
                angle: pb_dir.angle_between(pa_dir).to_degrees(),
                rot_axis,
                radius: default_r,
            });
        }
        let quat = rotate_from_vec3_to_vec3(x_dir, pb_dir, pa_dir);
        let (axis_z, angle) = quat.to_axis_angle();
        rotate_info.rot_axis = axis_z;
        rotate_info.angle = angle.to_degrees();
        // dbg!(&rotate_info);
        let y_dir = axis_z.cross(pb_dir).normalize();
        let mat3 = Mat3::from_cols(pb_dir, y_dir, axis_z);

        //先转换到二维平面
        let trans = mat3.inverse();
        let pa_2d = (trans * pa_dir).truncate();
        let pb_2d = (trans * pb_dir).truncate();
        // dbg!(pa_2d);
        // dbg!(pb_2d);
        let dir_2d = (trans * x_dir).truncate();
        let pt_a = dir_2d * dist;

        let ray_a = Ray2D::new(pt_a, -pa_2d);
        let ray_b = Ray2D::new(Vec2::ZERO, pb_2d);
        // let epsilon = 1e-5;
        // dbg!(&ray_a);
        // dbg!(&ray_b);
        //将pa, pb 都变换到二维平面，然后求交点
        //如果pa, pb 平行，直接返回一个pb为基准的半圆
        // if a_dir.cross(b_dir).try_normalize().is_none(){
        let c_dir = pb_2d.rotate(Vec2::from_angle(PI / 2.0));
        let r = if let Some(f_pt) = ray_a.intersect(&ray_b) {
            // dbg!(f_pt);
            let r = f_pt.length() * (PI / 2.0 - angle / 2.0).tan();
            r
        } else {
            // dbg!("not intersect");
            ray_a.perpendicular_distance(&ray_b).unwrap_or(0.0) / 2.0
        };
        rotate_info.radius = r;
        rotate_info.center = b_pt + mat3 * c_dir.extend(0.0) * r;
        Some(rotate_info)
    }

    pub fn cal_rotate_info_old(
        a_dir: Vec3,
        a_pt: Vec3,
        b_dir: Vec3,
        b_pt: Vec3,
        default_r: f32,
    ) -> Option<RotateInfo> {
        let mut rotate_info = RotateInfo::default();
        let pa_dir = a_dir.normalize();
        let pb_dir = b_dir.normalize();

        let x_dir = (b_pt - a_pt).normalize();
        if x_dir.is_nan() {
            let rot_axis = pb_dir.cross(-pa_dir).normalize();
            if rot_axis.is_nan() {
                return None;
            }
            return Some(RotateInfo {
                center: Default::default(),
                angle: (-pa_dir).angle_between(pb_dir).to_degrees(),
                rot_axis,
                radius: default_r,
            });
        }
        let quat = rotate_from_vec3_to_vec3(x_dir, -pa_dir, pb_dir);
        let (axis_z, angle) = quat.to_axis_angle();
        rotate_info.rot_axis = axis_z;
        rotate_info.angle = angle.to_degrees();
        //pa的点位只是 只提供方位, 位置信息是可以移动的

        let mid_pt = (a_pt + b_pt) / 2.0;
        let x_len = b_pt.distance(a_pt);
        if x_len < 1.0e-3 {
            return None;
        }
        if (angle - std::f32::consts::PI).abs() < 1.0e-3 || angle.abs() < 1.0e-3 {
            rotate_info.center = mid_pt;
            rotate_info.radius = x_len / 2.0;
        } else {
            let y_dir = rotate_info.rot_axis.cross(x_dir);
            let ref_dir = rotate_info.rot_axis.cross(b_dir.normalize()).normalize();
            let p = b_pt - mid_pt;
            let px = p.dot(x_dir);
            let _py = p.dot(y_dir);
            if px < 1.0e-3 {
                return None;
            }
            let beta = angle / 2.0;
            rotate_info.radius = px / beta.sin().abs();
            rotate_info.center = b_pt + ref_dir * rotate_info.radius;
        }
        return Some(rotate_info);
    }
}

#[test]
fn test_pa_axis_pos_not_corret() {
    let pa: CateAxisParam = serde_json::from_str(
        r#"
        {
			"dir": [
				0.80901700258255,
				0,
				0.5877853035926819
			],
			"dir_flag": 1,
			"number": 8,
			"pbore": 0,
			"pconnect": "0",
			"pheight": 0,
			"pt": [
				456.6310119628906,
				140,
				393.1449890136719
			],
			"pwidth": 0,
			"ref_dir": null,
			"refno": "21436_3307"
		}
    "#,
    )
    .unwrap();
    let pa = -pa;

    let pb: CateAxisParam = serde_json::from_str(
        r#"
        {
            "dir": [
				-0.0,
				-1,
				-0.0
			],
			"dir_flag": 1,
			"number": 9,
			"pbore": 0,
			"pconnect": "0",
			"pheight": 0,
			"pt": [
				717.5,
				70,
				582.5
			],
			"pwidth": 0,
			"ref_dir": null,
			"refno": "21436_3308"
		}
    "#,
    )
    .unwrap();

    // dbg!((&pa, &pb));
    let info = RotateInfo::cal_rotate_info(pa.dir.unwrap(), pa.pt, pb.dir.unwrap(), pb.pt, 1.0);
    dbg!(&info);
}

#[test]
fn test_ray_intersection() {
    use glam::vec3;
    let ray1 = Ray::new(vec3(0.0, 0.0, 0.0), vec3(1.0, 0.0, 0.0));
    let ray2 = Ray::new(vec3(0.5, -1.0, 0.0), vec3(0.0, 1.0, 0.0));

    let epsilon = 1e-5; // 定义一个小的阈值

    match ray_intersection(&ray1, &ray2, epsilon) {
        Some(intersection) => println!("Rays intersect at: {:?}", intersection),
        None => println!("Rays do not intersect"),
    }
}
