use glam::*;
use nom::number::complete::float;
use nom::*;
use std::collections::HashMap;

use lazy_static::lazy_static;
use nom::branch::alt;
use nom::bytes::complete::*;
use nom::combinator::{complete, opt, recognize};
use nom::sequence::{delimited, pair};

lazy_static! {
    pub static ref AXISES_MAP: HashMap<&'static str, Vec3> = {
        let mut s = HashMap::new();
        s.insert("X", Vec3::X);
        s.insert("Y", Vec3::Y);
        s.insert("Z", Vec3::Z);
        s.insert("E", Vec3::X);
        s.insert("N", Vec3::Y);
        s.insert("U", Vec3::Z);
        s.insert("-X", -Vec3::X);
        s.insert("-Y", -Vec3::Y);
        s.insert("-Z", -Vec3::Z);
        s.insert("W", -Vec3::X);
        s.insert("S", -Vec3::Y);
        s.insert("D", -Vec3::Z);
        s
    };
}

#[derive(Debug, Default)]
pub struct Rotation {
    axis: Vec3,
    angle: f64,
}

#[derive(Debug, Default)]
pub struct RotationStruct {
    origin_axis: Vec3,
    rot1: Option<Rotation>,
    rot2: Option<Rotation>,
}

pub fn signed_axis(input: &str) -> IResult<&str, (Option<&str>, &str)> {
    pair(
        opt(tag("-")), // maybe sign?
        alt((
            tag("X"),
            tag("Y"),
            tag("Z"),
            tag("E"),
            tag("N"),
            tag("U"),
            tag("W"),
            tag("S"),
            tag("D"),
        )),
    )(input)
}

use crate::tool::math_tool::convert_to_xyz;
use crate::tool::parse_to_dir::parse_to_direction;
use nom::number::complete::double;

pub fn parse_angle(input: &str) -> IResult<&str, f64> {
    alt((double, delimited(tag("("), double, tag(")"))))(input)
}

fn parse_axis_rotation(input: &str) -> IResult<&str, Rotation> {
    let (input, angle) = parse_angle(input)?;
    let (input, axis) = recognize(signed_axis)(input)?;
    Ok((
        input,
        Rotation {
            axis: *AXISES_MAP.get(axis).unwrap(),
            angle,
        },
    ))
}

pub fn parse_rotation_struct(input: &str) -> IResult<&str, RotationStruct> {
    let (input, axis) = recognize(signed_axis)(input)?;
    if input.is_empty() {
        return Ok((
            input,
            RotationStruct {
                origin_axis: *AXISES_MAP.get(axis).unwrap(),
                rot1: None,
                rot2: None,
            },
        ));
    }
    let (input, rot1) = opt(complete(parse_axis_rotation))(input)?;
    if input.is_empty() {
        return Ok((
            input,
            RotationStruct {
                origin_axis: *AXISES_MAP.get(axis).unwrap(),
                rot1,
                rot2: None,
            },
        ));
    }
    let (input, rot2) = opt(complete(parse_axis_rotation))(input)?;
    Ok((
        input,
        RotationStruct {
            origin_axis: *AXISES_MAP.get(axis).unwrap(),
            rot1,
            rot2,
        },
    ))
}

///解析expression到direction
pub fn parse_expr_to_dir(expr: &str) -> Option<DVec3> {
    if let Ok(to_dir) = parse_to_direction(expr, None) {
        return to_dir;
    }

    let expr = convert_to_xyz(expr).replace(" ", "");
    // dbg!(&expr);
    if let Ok((_, rs)) = parse_rotation_struct(&expr) {
        // dbg!(&rs);
        let mut axis: DVec3 = rs.origin_axis.into();
        if let Some(rot1) = rs.rot1 {
            let target_axis = axis.cross(rot1.axis.into());
            let quat1 = DQuat::from_axis_angle(target_axis, rot1.angle.to_radians() as _);
            axis = (quat1 * axis).normalize();
            if let Some(rot2) = rs.rot2 {
                let target_axis = axis.cross(rot2.axis.into());
                let quat2 = DQuat::from_axis_angle(target_axis, rot2.angle.to_radians() as _);
                axis = (quat2 * axis).normalize();
            }
        }
        return Some(axis.into());
    }
    None
}
