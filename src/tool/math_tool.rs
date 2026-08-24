use crate::shape::pdms_shape::{ANGLE_RAD_F64_TOL, ANGLE_RAD_TOL};
use crate::tool::dir_tool::parse_ori_str_to_quat;
use crate::tool::float_tool::*;
use approx::{abs_diff_eq, abs_diff_ne};
use glam::{DMat3, DMat4, DQuat, DVec3, Mat3, Quat, Vec3};
use lazy_static::lazy_static;
use std::f32::consts::FRAC_PI_2;

use super::direction_parse::parse_expr_to_dir;

lazy_static! {
    pub static ref AXIS_VEC_TUPLES: [(glam::Vec3, &'static str); 6] = {
        [
            (Vec3::X, "E"),
            (-Vec3::X, "W"),
            (Vec3::Y, "N"),
            (-Vec3::Y, "S"),
            (Vec3::Z, "U"),
            (-Vec3::Z, "D"),
        ]
    };
    pub static ref AXIS_DVEC_TUPLES: [(glam::DVec3, &'static str); 6] = {
        [
            (DVec3::X, "E"),
            (-DVec3::X, "W"),
            (DVec3::Y, "N"),
            (-DVec3::Y, "S"),
            (DVec3::Z, "U"),
            (-DVec3::Z, "D"),
        ]
    };
}

//x轴作为参考轴去当目标轴
pub fn cal_quat_by_zdir_with_xref(zdir: DVec3) -> DQuat {
    let mut xdir = if zdir.z.abs() < 1.0e-5 {
        -DVec3::Z * zdir.x.signum()
    } else {
        DVec3::X * zdir.z.signum()
    };
    let ydir = zdir.cross(xdir).normalize();
    xdir = ydir.cross(zdir).normalize();
    let mat3 = DMat3::from_cols(xdir, ydir, zdir);
    DQuat::from_mat3(&mat3)
}

pub fn convert_to_xyz(s: &str) -> String {
    s.replace("E", "X")
        .replace("N", "Y")
        .replace("U", "Z")
        .replace("W", "-X")
        .replace("S", "-Y")
        .replace("D", "-Z")
}

pub fn to_pdms_vec_xyz_str(vec: &Vec3, convert_xyz: bool) -> String {
    convert_to_xyz(&to_pdms_vec_str(vec, convert_xyz))
}

pub fn to_pdms_vec_str(vec: &Vec3, convert_xyz: bool) -> String {
    to_pdms_dvec_str(&vec.as_dvec3(), convert_xyz)
}

pub const DVEC_STR_ANGLE_RAD_F64_TOL: f64 = 0.0000001;

#[inline]
pub fn to_pdms_dvec_str(v: &DVec3, convert_xyz: bool) -> String {
    let result = to_pdms_dvec_str_with_tol(v, DVEC_STR_ANGLE_RAD_F64_TOL);
    if convert_xyz {
        return convert_to_xyz(&result);
    } else {
        return result;
    }
}

pub fn to_pdms_dvec_str_with_tol(v: &DVec3, tol: f64) -> String {
    for (axis, v_str) in AXIS_DVEC_TUPLES.iter() {
        if abs_diff_eq!(axis.dot(*v), 1.0, epsilon = tol) {
            return (*v_str).to_string();
        }
    }
    //1个象限的组合
    if abs_diff_eq!(v.x, 0.0, epsilon = tol)
        || abs_diff_eq!(v.y, 0.0, epsilon = tol)
        || abs_diff_eq!(v.z, 0.0, epsilon = tol)
    {
        let mut x = 0.0;
        let mut y = 0.0;
        let mut x_str = "";
        let mut y_str = "";
        let mut angle = 0.0f64;
        if abs_diff_eq!(v.x, 0.0, epsilon = tol) {
            x = v.y;
            y = v.z;
            angle = (y / x).atan().to_degrees().abs();
            x_str = if x > 0.0 { "N" } else { "S" };
            y_str = if y > 0.0 { "U" } else { "D" };
        } else if abs_diff_eq!(v.y, 0.0, epsilon = tol) {
            x = v.x;
            y = v.z;
            angle = (y / x).atan().to_degrees().abs();
            x_str = if x > 0.0 { "E" } else { "W" };
            y_str = if y > 0.0 { "U" } else { "D" };
        } else if abs_diff_eq!(v.z, 0.0, epsilon = tol) {
            x = v.x;
            y = v.y;
            angle = (y / x).atan().to_degrees().abs();
            x_str = if x > 0.0 { "E" } else { "W" };
            y_str = if y > 0.0 { "N" } else { "S" };
        }
        angle = f64_round_4(angle);
        if angle.abs() < tol {
            return x_str.to_string();
        }
        if angle > 90.0 && angle <= 180.0 {
            angle = 180.0 - angle;
            x_str = neg_dir_str(x_str);
        } else if angle < -90.0 && angle >= -180.0 {
            angle += 180.0;
            x_str = neg_dir_str(x_str);
        }
        if angle.is_nan() {
            return "unset".to_string();
        }

        if abs_diff_eq!(angle, 0.0, epsilon = 0.01) {
            return x_str.to_string();
        }
        if angle > 45.0 {
            if y_str != "U" && y_str != "D" {
                return format!("{y_str} {} {x_str}", f64_round_3(90.0 - angle));
            }
        } else if angle == 45.0 {
            if x_str.contains("Y") && y_str.contains("X") {
                return format!("{y_str} {} {x_str}", f64_round_3(90.0 - angle));
            }
        }
        if angle == 90.0 {
            return y_str.to_string();
        }
        return format!("{x_str} {} {y_str}", f64_round_3(angle));
    }

    //2个象限的组合, 最后一个留给Z轴
    let plane_vec = DVec3::new(v.x, v.y, 0.0);
    let part_str = to_pdms_dvec_str(&plane_vec, false);
    let l = plane_vec.length();
    let mut theta = (v.z / l).atan().to_degrees();
    let mut z_str = "U";
    theta = f64_round_4(theta);
    if theta < 0.0 {
        theta = -theta;
        z_str = "D";
    }
    if theta < tol {
        return format!("{part_str}");
    }

    if theta == 90.0 {
        return z_str.to_string();
    }

    format!("{part_str} {} {z_str}", f64_round_3(theta))
}

fn neg_dir_str(y_str: &str) -> &str {
    let y_str = match y_str {
        "N" => "S",
        "S" => "N",
        "U" => "D",
        "D" => "U",
        "E" => "W",
        "W" => "E",
        _ => "unset",
    };
    y_str
}

#[inline]
pub fn to_pdms_ori_str(rot: &Mat3, convert_xyz: bool) -> String {
    let y_axis = &rot.y_axis;
    let z_axis = &rot.z_axis;

    format!(
        "Y is {:.3} and Z is {:.3}",
        to_pdms_vec_str(y_axis, convert_xyz),
        to_pdms_vec_str(z_axis, convert_xyz)
    )
}

#[inline]
pub fn to_pdms_dori_xyz_str(rot: &DMat3, convert_xyz: bool) -> String {
    let y_axis = &rot.y_axis;
    let z_axis = &rot.z_axis;
    // let s = to_pdms_dvec_str(y_axis);
    //讲s里面的 "E" 替换成 "X", "N" 替换成 "Y", "U" 替换成 "Z", "W" 替换成 "-X", "S" 替换成 "-Y", "D" 替换成 "-Z"
    format!(
        "Y is {} and Z is {}",
        convert_to_xyz(&to_pdms_dvec_str(y_axis, convert_xyz)),
        convert_to_xyz(&to_pdms_dvec_str(z_axis, convert_xyz))
    )
}

#[inline]
pub fn to_pdms_ori_xyz_str(rot: &Mat3, convert_xyz: bool) -> String {
    let y_axis = &rot.y_axis;
    let z_axis = &rot.z_axis;
    let s = to_pdms_vec_str(y_axis, convert_xyz);
    //讲s里面的 "E" 替换成 "X", "N" 替换成 "Y", "U" 替换成 "Z", "W" 替换成 "-X", "S" 替换成 "-Y", "D" 替换成 "-Z"
    format!(
        "Y is {} and Z is {}",
        to_pdms_vec_str(y_axis, convert_xyz),
        to_pdms_vec_str(z_axis, convert_xyz)
    )
}

#[inline]
pub fn quat_to_pdms_ori_str(rot: &Quat, convert_xyz: bool) -> String {
    let rot = Mat3::from_quat(*rot);
    let y_axis = &rot.y_axis;
    let z_axis = &rot.z_axis;

    // "E".to_string()
    format!(
        "Y is {} and Z is {}",
        to_pdms_vec_str(y_axis, convert_xyz),
        to_pdms_vec_str(z_axis, convert_xyz)
    )
}

#[inline]
fn format_f32(num: f32) -> String {
    if num.fract() == 0.0 {
        // 如果是整数，只显示整数部分
        format!("{}", num as i32)
    } else {
        // 如果有小数，保留三位小数
        format!("{:.3}", num)
    }
}

#[inline]
pub fn vec3_to_xyz_str(pos: Vec3) -> String {
    format!(
        "X {}mm, Y {}mm, Z {}mm",
        f32_round_3(pos[0]),
        f32_round_3(pos[1]),
        f32_round_3(pos[2])
    )
}

#[inline]
pub fn dvec3_to_xyz_str(pos: DVec3) -> String {
    format!(
        "X {}mm Y {}mm Z {}mm",
        f64_round_3(pos[0]),
        f64_round_3(pos[1]),
        f64_round_3(pos[2])
    )
}

// #[inline]
// pub fn vec3_to_xyz_str(pos: Vec3) -> String {
//     format!(
//         "X {}mm, Y {}mm, Z {}mm",
//         format_f32(pos[0]),
//         format_f32(pos[1]),
//         format_f32(pos[2])
//     )
// }

#[inline]
pub fn quat_to_pdms_ori_xyz_str(rot: &Quat, convert_xyz: bool) -> String {
    let rot = DMat3::from_quat((*rot).as_dquat());
    let y_axis = &rot.y_axis;
    let z_axis = &rot.z_axis;

    // "E".to_string()
    format!(
        "Y is {} and Z is {}",
        to_pdms_dvec_str(&y_axis, convert_xyz),
        to_pdms_dvec_str(&z_axis, convert_xyz)
    )
}

#[inline]
pub fn dquat_to_pdms_ori_xyz_str(rot: &DQuat, convert_xyz: bool) -> String {
    let rot = DMat3::from_quat(*rot);
    let y_axis = &rot.y_axis;
    let z_axis = &rot.z_axis;

    // "E".to_string()
    format!(
        "Y is {} and Z is {}",
        to_pdms_dvec_str(&y_axis, convert_xyz),
        to_pdms_dvec_str(&z_axis, convert_xyz)
    )
}

#[inline]
pub fn angles_to_ori(angs: DVec3) -> Option<DQuat> {
    let mat = DMat3::from_rotation_z(angs[2].to_radians())
        * DMat3::from_rotation_y(angs[1].to_radians())
        * DMat3::from_rotation_x(angs[0].to_radians());
    Some(DQuat::from_mat3(&mat))
}

#[inline]
pub fn angles_to_dori(angs: Vec3) -> Option<DQuat> {
    let mat = DMat3::from_rotation_z((angs[2] as f64).to_radians())
        * DMat3::from_rotation_y((angs[1] as f64).to_radians())
        * DMat3::from_rotation_x((angs[0] as f64).to_radians());
    Some(DQuat::from_mat3(&mat))
}

pub fn cal_end_points(center: DVec3, ori_str: &str, len: f64) -> Option<[DVec3; 2]> {
    let dir = parse_ori_str_to_quat(ori_str).ok()?.as_dquat() * DVec3::Y;
    let start = center - dir * len / 2.0;
    let end = center + dir * len / 2.0;
    Some([start, end])
}

#[test]
fn test_cal_end_points() {
    let pts = cal_end_points(
        DVec3::new(44200.0, -18050.0, 10293.0),
        "Y is N and Z is Z",
        900.0,
    )
    .unwrap();
    dbg!(pts);
}

#[test]
fn test_convert_to_dir_string() {
    let v = DVec3::new(
        -0.5150378973276237,
        -0.8571671240513605,
        0.0005357142014608308,
    );

    // dbg!(convert_to_xyz(&to_pdms_dvec_str(&v)));

    let v = DVec3::new(
        0.00027591317024652014,
        0.0004591967205145638,
        0.9999998565051292,
    );

    dbg!(to_pdms_dvec_str(&v, true));
}

fn assert_vec3_to_str(input: &str, assert_str: &str) {
    // let v_str = "X 120 Y"; // -X 60 Y
    let v = parse_expr_to_dir(input).unwrap();
    let new_v_str = to_pdms_dvec_str(&v, true);
    dbg!(&new_v_str);
    assert_eq!(assert_str, new_v_str);
}

#[test]
fn test_vec3_to_str() {
    assert_vec3_to_str("X 120 Y", "Y 30 -X");
    assert_vec3_to_str("X 45 Y", "X 45 Y");
    assert_vec3_to_str("Y 45 X", "X 45 Y");
    assert_vec3_to_str("Z 60 Y", "Y 30 Z");
    assert_vec3_to_str("Z 30 Y", "Y 60 Z");
}
