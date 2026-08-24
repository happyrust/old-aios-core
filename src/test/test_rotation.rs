use crate::tool::dir_tool::parse_ori_str_to_quat;
use crate::tool::direction_parse::parse_expr_to_dir;
use crate::tool::math_tool::{
    quat_to_pdms_ori_xyz_str, to_pdms_ori_xyz_str, to_pdms_vec_str, to_pdms_vec_xyz_str,
};
use glam::{Mat3, Quat, Vec3};

#[test]
fn test_rotation() {
    // let rotation = parse_ori_str_to_quat("Y is X 6 Y and Z is Y 6 -X").unwrap_or(Quat::IDENTITY);
    // dbg!(quat_to_pdms_ori_xyz_str(&rotation, true));
    // let align_axis = rotation * Vec3::Y;
    // dbg!(to_pdms_vec_xyz_str(&align_axis));
    //
    // let axis_1 = rotation * Vec3::Z;
    // dbg!(to_pdms_vec_xyz_str(&axis_1));
    //
    // let rotation = parse_ori_str_to_quat("Y is Z and Z is Y 5 -X").unwrap_or(Quat::IDENTITY);
    // dbg!(quat_to_pdms_ori_xyz_str(&rotation, true));
    //
    // let align_axis = rotation * Vec3::Y;
    // dbg!(to_pdms_vec_xyz_str(&align_axis));
    //
    // let axis_1 = rotation * Vec3::Z;
    // dbg!(to_pdms_vec_xyz_str(&axis_1));
}

#[test]
fn test_rotation_complex() {
    //170
    // let rotation = parse_ori_str_to_quat("Y is X 23.729 Y 80 -Z and Z is Y 23.999 -X 0.0477 -Z").unwrap();
    // dbg!(Mat3::from_quat(rotation));
    // dbg!(quat_to_pdms_ori_xyz_str(&rotation, true));
    //
    // //0
    // let rotation = parse_ori_str_to_quat("Y is Y 23.999 -X 89.952 Z and Z is Y 23.999 -X 0.0477 -Z").unwrap();
    // dbg!(Mat3::from_quat(rotation));
    // dbg!(quat_to_pdms_ori_xyz_str(&rotation, true));
}
