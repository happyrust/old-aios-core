use crate::{CataContext, eval_str_to_f64};
use anyhow::anyhow;
use glam::{DVec3, Vec3};
use nom::IResult;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_until};
use nom::character::complete::space0;
use nom::combinator::{map_res, opt, recognize};
use nom::error::Error;
use nom::number::complete::{double, float};
use nom::sequence::{delimited, preceded, tuple};

#[derive(Debug, PartialEq)]
pub struct Coordinate {
    x: Option<(String, bool)>,
    y: Option<(String, bool)>,
    z: Option<(String, bool)>,
}

impl Coordinate {
    pub fn get_dir(&self) -> Option<DVec3> {
        // let v = DVec3::new(
        //     self.x.unwrap_or(0.0) as f64,
        //     self.y.unwrap_or(0.0) as f64,
        //     self.z.unwrap_or(0.0) as f64,
        // )
        // .normalize_or_zero();
        // if v.is_normalized() {
        //     Some(v)
        // } else {
        //     None
        // }
        None
    }
}

#[derive(Debug, PartialEq)]
pub struct Direction {
    coordinate: Coordinate,
}

pub fn ws<'a, F: 'a, O>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O>
where
    F: FnMut(&'a str) -> IResult<&'a str, O>,
{
    delimited(space0, inner, space0)
}

fn parse_pos_expr(input: &str) -> IResult<&str, (String, bool)> {
    let (input, content) = recognize(take_until(")"))(input)?;
    // let (input, _) = tag(")")(input)?;
    Ok((input, (content.trim().to_string(), false)))
}

// fn parse_bracket_content(input: &str) -> IResult<&str, String> {
//     let (input, _) = tag("(");
//     let (input, content) = recognize(take_until(")"))(input)?;
//     let (input, _) = tag(")")(input)?;
//     Ok((input, content.replace(" ", "")))
// }

fn parse_neg_pos_expr(input: &str) -> IResult<&str, (String, bool)> {
    //这里也有可能是表达式，也有可能是数值，所以需要先都当作string处理
    map_res(
        preceded(
            ws(tag("NEG")),
            delimited(ws(tag("(")), ws(parse_pos_expr), ws(tag(")"))),
        ),
        |n| Ok::<_, ()>((n.0, true)),
    )(input)
}

fn parse_coordinate_value(input: &str) -> IResult<&str, (String, bool)> {
    alt((
        delimited(opt(ws(tag("("))), parse_neg_pos_expr, opt(ws(tag(")")))),
        delimited(opt(ws(tag("("))), ws(parse_pos_expr), opt(ws(tag(")")))),
    ))(input)
}

fn parse_axis_value(axis: &'static str) -> impl Fn(&str) -> IResult<&str, (String, String, bool)> {
    move |input: &str| {
        let (input, _) = ws(tag(axis))(input)?;
        let (input, (value, is_neg)) = parse_coordinate_value(input)?;
        Ok((input, (axis.to_string(), value, is_neg)))
    }
}

pub fn parse_coordinate(input: &str) -> IResult<&str, Coordinate> {
    let (input, values) = nom::multi::many0(alt((
        parse_axis_value("X"),
        parse_axis_value("Y"),
        parse_axis_value("Z"),
    )))(input)?;

    let mut coord = Coordinate {
        x: None,
        y: None,
        z: None,
    };
    for (axis, value, is_neg) in values {
        match axis.as_str() {
            "X" => coord.x = Some((value, is_neg)),
            "Y" => coord.y = Some((value, is_neg)),
            "Z" => coord.z = Some((value, is_neg)),
            _ => {}
        }
    }

    Ok((input, coord))
}

pub fn parse_str_to_vec3(input: &str) -> Option<DVec3> {
    let input = input.replace(" ", "").replace("mm", "");
    // 定义解析单个坐标的解析器
    fn parse_component<'a>(
        axis: char,
    ) -> impl FnMut(&'a str) -> IResult<&'a str, f64, Error<&'a str>> {
        let axis_str = axis.to_string();
        move |input: &'a str| {
            let (input, _) = tag(axis_str.as_str())(input)?;
            double(input)
        }
    }

    // 定义解析向量的解析器
    fn parse_vec3(input: &str) -> IResult<&str, DVec3, Error<&str>> {
        let (input, coords) = nom::multi::many0(alt((
            map_res(parse_component('X'), |v| Ok::<_, ()>((0, v))),
            map_res(parse_component('Y'), |v| Ok::<_, ()>((1, v))),
            map_res(parse_component('Z'), |v| Ok::<_, ()>((2, v))),
        )))(input)?;

        let mut values = [0.0, 0.0, 0.0];
        for (idx, val) in coords {
            values[idx] = val;
        }

        Ok((input, DVec3::new(values[0], values[1], values[2])))
    }

    // 执行解析
    match parse_vec3(&input) {
        Ok((_, vec)) => Some(vec),
        Err(_) => Some(DVec3::ZERO),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_str_to_vec3() {
        // Test case 1: Basic coordinates
        let result = parse_str_to_vec3("X1.0Y2.0Z3.0");
        assert_eq!(result, Some(DVec3::new(1.0, 2.0, 3.0)));

        // Test case 2: With spaces and mm
        let result = parse_str_to_vec3("X 1.0 mm Y 2.0 mm Z 3.0 mm");
        assert_eq!(result, Some(DVec3::new(1.0, 2.0, 3.0)));

        // Test case 3: Missing coordinates
        let result = parse_str_to_vec3("X1.0Z3.0");
        assert_eq!(result, Some(DVec3::new(1.0, 0.0, 3.0)));

        // Test case 4: Only one coordinate
        let result = parse_str_to_vec3("Y2.0");
        assert_eq!(result, Some(DVec3::new(0.0, 2.0, 0.0)));

        // Test case 5: Invalid input
        let result = parse_str_to_vec3("invalid");
        assert_eq!(result, Some(DVec3::new(0.0, 0.0, 0.0)));

        // Test case 6: Negative values
        let result = parse_str_to_vec3("X-1.0Y-2.0Z-3.0");
        assert_eq!(result, Some(DVec3::new(-1.0, -2.0, -3.0)));
    }
}

// fn parse_axis_dir(input: &str) -> IResult<&str, Coordinate> {
//     let (input, values) = nom::multi::many0(alt((
//         parse_axis_value("X"),
//         parse_axis_value("Y"),
//         parse_axis_value("Z"),
//     )))(input)?;
//
//     let mut coord = Coordinate {
//         x: None,
//         y: None,
//         z: None,
//     };
//     for (axis, value, is_neg) in values {
//         match axis.as_str() {
//             "X" => coord.x = Some((value, is_neg)),
//             "Y" => coord.y = Some((value, is_neg)),
//             "Z" => coord.z = Some((value, is_neg)),
//             _ => {}
//         }
//     }
//
//     Ok((input, coord))
// }

pub fn parse_to_direction(
    input: &str,
    context: Option<&CataContext>,
) -> anyhow::Result<Option<DVec3>> {
    let (remaining_input, _) = ws(tag("TO"))(input).map_err(|_| anyhow!("Parsing failed!"))?;
    // dbg!(input);
    let (remaining_input, coordinate) =
        parse_coordinate(remaining_input).map_err(|_| anyhow!("Parsing failed!"))?;
    // dbg!(&coordinate);
    let mut dir = DVec3::ZERO;
    let has_context = context.is_some();
    if let Some((s, neg)) = &coordinate.x {
        if let Ok(v) = s.parse::<f64>() {
            dir.x = if *neg { -v } else { v };
        } else if has_context && let Ok(v) = eval_str_to_f64(s, context.unwrap(), "") {
            dir.x = if *neg { -v } else { v };
        } else {
            return Ok(None);
        }
    }
    if let Some((s, neg)) = &coordinate.y {
        if let Ok(v) = s.parse::<f64>() {
            dir.y = if *neg { -v } else { v };
        } else if has_context && let Ok(v) = eval_str_to_f64(s, context.unwrap(), "") {
            dir.y = if *neg { -v } else { v };
        } else {
            return Ok(None);
        }
    }
    if let Some((s, neg)) = &coordinate.z {
        if let Ok(v) = s.parse::<f64>() {
            dir.z = if *neg { -v } else { v };
        } else if has_context && let Ok(v) = eval_str_to_f64(s, context.unwrap(), "") {
            dir.z = if *neg { -v } else { v };
        } else {
            return Ok(None);
        }
    }

    Ok(Some(dir.normalize_or_zero()))
}
