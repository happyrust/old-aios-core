use crate::tool::direction_parse::{AXISES_MAP, parse_expr_to_dir, parse_rotation_struct};
use anyhow::anyhow;
use glam::{DMat3, DQuat, Mat3, Quat};

#[inline]
pub fn parse_ori_str_to_mat(ori_str: &str) -> anyhow::Result<Mat3> {
    parse_ori_str_to_quat(ori_str).map(|q| Mat3::from_quat(q))
}

pub fn parse_ori_str_to_dmat3(ori_str: &str) -> anyhow::Result<DMat3> {
    let dir_strs = ori_str.split(" and ").collect::<Vec<_>>();
    // dbg!(&dir_strs);
    if dir_strs.len() < 2 {
        return Err(anyhow!("不是方位字符串"));
    };
    let mut mat = DMat3::IDENTITY;
    let mut comb_dir_str = String::new();
    for i in 0..2 {
        let d = dir_strs[i].trim();
        let strs = d.split("is").collect::<Vec<_>>();
        if strs.len() != 2 {
            return Err(anyhow!("不是方位字符串"));
        }
        let f = strs[0].trim().to_uppercase();
        let dir_str = strs[1]
            .trim()
            .replace("E", "X")
            .replace("W", "-X")
            .replace("N", "Y")
            .replace("S", "-Y")
            .replace("U", "Z")
            .replace("D", "-Z");
        // dbg!(&dir_str);
        if let Some(dir) = parse_expr_to_dir(&dir_str) {
            comb_dir_str.push_str(f.as_str());
            match f.as_str() {
                "X" => mat.x_axis = dir,
                "Y" => mat.y_axis = dir,
                "Z" => mat.z_axis = dir,
                _ => {}
            }
        } else {
            return Err(anyhow!("方位字符串有错误"));
        }
    }

    match comb_dir_str.as_str() {
        "XY" => mat.z_axis = mat.x_axis.cross(mat.y_axis).normalize_or_zero(),
        "YZ" => mat.x_axis = mat.y_axis.cross(mat.z_axis).normalize_or_zero(),
        "XZ" => mat.y_axis = mat.z_axis.cross(mat.x_axis).normalize_or_zero(),
        _ => {}
    }

    Ok(mat)
}

pub fn parse_ori_str_to_dquat(ori_str: &str) -> anyhow::Result<DQuat> {
    let dmat = parse_ori_str_to_dmat3(ori_str)?;
    Ok(DQuat::from_mat3(&dmat))
}

//Y is N and Z is U
pub fn parse_ori_str_to_quat(ori_str: &str) -> anyhow::Result<Quat> {
    Ok(parse_ori_str_to_dquat(ori_str)?.as_quat())
}

#[test]
fn test_parse_vector() {
    let test_str = "X30Y";
    let dir = parse_expr_to_dir(test_str);
    dbg!(dir);
    let dir = parse_rotation_struct(test_str).unwrap();
    dbg!(&dir);
    dbg!(AXISES_MAP.get("-X"));
    let test_str = "-X(59)Y";
    let res = parse_expr_to_dir(test_str);
    println!("{:?}", res);
    let test_str = "Z(90.0)Y";
    let res = parse_expr_to_dir(test_str);
    println!("{:?}", res);
    let test_str = "-Y45Z";
    let res = parse_expr_to_dir(test_str);
    println!("test_str: {:?}", res);
    let test_str = "Y45-Z";
    let res = parse_expr_to_dir(test_str);
    println!("test_str: {:?}", res);

    let test_str = "X ( 36 ) Z";
    let res = parse_expr_to_dir(test_str);
    println!("test_str: {:?}", res);
    //Z DESIGN PARAM 14Y
}

#[test]
fn parse_ori() {
    let str = "Y is W and Z is U";
    let ori = parse_ori_str_to_quat(str);
    dbg!(ori);
}
