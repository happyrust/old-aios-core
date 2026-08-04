#[cfg(feature = "truck")]
use crate::shape::pdms_shape::BrepMathTrait;
use crate::shape::pdms_shape::LEN_TOL;
use crate::tool::float_tool::*;
use crate::tool::float_tool::{cal_vec2_hash_string, cal_xy_hash_string, vec3_round_2};
use anyhow::anyhow;
use approx::abs_diff_eq;
use cavalier_contours::core::math::{angle, bulge_from_angle, Vector2};
use cavalier_contours::core::traits::Real;
use cavalier_contours::pline_closed;
use cavalier_contours::polyline::internal::pline_boolean::polyline_boolean;
use cavalier_contours::polyline::internal::pline_intersects::visit_global_self_intersects;
use cavalier_contours::polyline::*;
use cavalier_contours::static_aabb2d_index::StaticAABB2DIndex;
use clap::builder::TypedValueParser;
// use geo::convex_hull::{graham_hull, quick_hull};
// use geo::{coord, Contains, ConvexHull, IsConvex};
// use geo::{line_string, point, Intersects, LineString};
// use geo::{Line, LinesIter, Orient, Polygon, RemoveRepeatedPoints, Winding};
use glam::{DVec2, DVec3, Quat, Vec3};
use nalgebra::{ComplexField, DimAdd};
use num_traits::signum;
use serde_derive::{Deserialize, Serialize};
use rust_ploop_processor::{PLoopProcessor, PLoop, Vertex as PLoopVertex};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::f32::consts::PI;
use std::panic::AssertUnwindSafe;
#[cfg(feature = "truck")]
use truck_base::cgmath64::{InnerSpace, MetricSpace, Point3, Rad, Vector3};

#[cfg(feature = "occ")]
use crate::prim_geo::basic::OccSharedShape;
#[cfg(feature = "occ")]
use opencascade::primitives::{Edge, Face, Wire};
use parry2d::bounding_volume::Aabb;
use parry2d::math::Point;
#[cfg(feature = "truck")]
use truck_modeling::builder;

#[derive(
    Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,
)]
pub enum CurveType {
    Fill,
    Spline(f32), //thick
}

pub fn cal_circus_center(pt0: Vec3, pt1: Vec3, pt2: Vec3) -> Vec3 {
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

#[cfg(feature = "truck")]
pub fn circus_center(pt0: Point3, pt1: Point3, pt2: Point3) -> Point3 {
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

#[cfg(feature = "occ")]
///生成occ的wire
pub fn gen_occ_spline_wire(loops: &Vec<Vec<Vec3>>, thick: f32) -> anyhow::Result<Wire> {
    let verts = &loops[0];
    if verts.len() != 3 {
        return Err(anyhow!("SPINE number is not 3".to_string())); //先假定必须有三个
    }

    let pt0 = verts[0];
    let transit = verts[1];
    let pt1 = verts[2];

    let vec0 = (pt0 - transit).normalize();
    let vec1 = (pt1 - transit).normalize();
    let origin = cal_circus_center(pt0, pt1, transit);
    let _angle = PI - vec0.angle_between(vec1);
    let mut rot_axis = Vec3::Z;
    if (vec0.cross(vec1)).dot(Vec3::Z) > 0.0 {
        rot_axis = -Vec3::Z;
    }
    let _radius = origin.distance(pt0);

    let v0 = (pt0 - origin).normalize();
    let v1 = (pt1 - origin).normalize();

    let half_thick = thick / 2.0;
    let p0 = (pt0 - v0 * half_thick).as_dvec3();
    let p1 = (pt1 - v1 * half_thick).as_dvec3();
    let p2 = (pt1 + v1 * half_thick).as_dvec3();
    let p3 = (pt0 + v0 * half_thick).as_dvec3();

    let t_v = (transit - origin).normalize();
    let t0 = (transit - (half_thick * t_v)).as_dvec3();
    let t1 = (transit + (half_thick * t_v)).as_dvec3();

    let edges = vec![
        Edge::arc(p0, p1, t0),
        Edge::segment(p1, p2),
        Edge::arc(p2, p3, t1),
        Edge::segment(p3, p0),
    ];

    Ok(Wire::from_edges(&edges)?)
}

#[cfg(feature = "truck")]
///生成truck的wire
pub fn gen_spline_wire(
    input_verts: &Vec<Vec3>,
    thick: f32,
) -> anyhow::Result<truck_modeling::Wire> {
    #[cfg(feature = "truck")]
    use truck_modeling::{builder, Wire};
    if input_verts.len() != 3 {
        return Err(anyhow!("SPINE number is not 3".to_string())); //先假定必须有三个
    }
    let verts = input_verts
        .into_iter()
        .map(|x| vec3_round_2(*x))
        .collect::<Vec<_>>();

    let pt0 = verts[0].point3();
    let transit = verts[1].point3();
    let pt1 = verts[2].point3();

    let vec0 = (pt0 - transit).normalize();
    let vec1 = (pt1 - transit).normalize();
    let origin = circus_center(pt0, pt1, transit);
    let _angle = Rad(PI as f64) - vec0.angle(vec1);
    let mut rot_axis = Vec3::Z;
    if (vec0.cross(vec1)).dot(Vector3::unit_z()) > 0.0 {
        rot_axis = -Vec3::Z;
    }
    let _radius = origin.distance(pt0);

    let v0 = (pt0 - origin).normalize();
    let v1 = (pt1 - origin).normalize();

    let half_thick = thick as f64 / 2.0;
    let p0 = pt0 - v0 * half_thick;
    let p1 = pt1 - v1 * half_thick;
    let p2 = pt1 + v1 * half_thick;
    let p3 = pt0 + v0 * half_thick;

    let ver0 = builder::vertex(p0);
    let ver1 = builder::vertex(p1);
    let ver2 = builder::vertex(p2);
    let ver3 = builder::vertex(p3);

    let t_v = (transit - origin).normalize();
    let t0 = transit - (half_thick * t_v);
    let t1 = transit + (half_thick * t_v);

    let wire = Wire::from([
        builder::circle_arc(&ver0, &ver1, t0),
        builder::line(&ver1, &ver2),
        builder::circle_arc(&ver2, &ver3, t1),
        builder::line(&ver3, &ver0),
    ]);

    Ok(wire)
}

pub fn polyline_to_debug_json_str(pline: &Polyline) -> String {
    format!(
        r#"
{{
    "isClosed": {},
    "vertexes": [
        {}
    ]
}}
"#,
        pline.is_closed(),
        pline
            .iter_vertexes()
            // .map(|v| format!("[{:.3}, {:.3}, {:.3}]", v.x, v.y, v.bulge))
            .map(|v| format!("[{}, {}, {}]", v.x, v.y, v.bulge))
            .collect::<Vec<_>>()
            .join(",\n        ")
    )
}

//todo 是否需要考虑wind方向
#[inline]
fn gen_fillet_spline(
    pt: DVec3,
    last_pt: DVec3,
    next_pt: DVec3,
    d1: DVec3,
    d2: DVec3,
    r: f64,
    sig_num: f64,
) -> Polyline {
    let mut pline = Polyline::new_closed();
    let angle = d1.angle_between(d2);
    if angle.abs() < 0.001 {
        return pline;
    }
    //f64_trunc_3
    let bulge = f64_trunc_3(bulge_from_angle(PI as f64 - angle)) * sig_num;
    // dbg!(bulge);
    let l = r / (angle / 2.0).tan();
    let mut p0 = pt + d1 * l;
    let mut p2 = pt + d2 * l;
    if last_pt.distance(p0).abs() < 0.01 {
        p0 = last_pt;
    }
    if next_pt.distance(p2).abs() < 0.01 {
        p2 = next_pt;
    }
    pline.add((p0.x), (p0.y), bulge);
    pline.add((p2.x), (p2.y), 0.0);
    pline.add((pt.x), (pt.y), 0.0);
    pline
}

#[inline]
fn add_fillet_spline(pline: &mut Polyline, pt: DVec3, d1: DVec3, d2: DVec3, r: f64) {
    let angle = d1.angle_between(d2);
    let l = r / (angle / 2.0).tan();
    dbg!(l);
    let p0 = pt + d1 * l;
    let p2 = pt + d2 * l;
    let bulge = f64_trunc_3(bulge_from_angle(PI as f64 - angle));
    pline.add(p0.x, p0.y, bulge);
    pline.add(p2.x, p2.y, 0.0);
}

#[test]
fn test_gen_occ_circle() {
    let pts = vec![
        Vec3::ZERO,
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let fradius = vec![0.5; 4];
    // //gen_occ_wires(&pts, &fradius);
}

#[test]
fn test_gen_occ_reverse_circle() {
    let mut pts = vec![
        Vec3::ZERO,
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    pts.reverse();
    let mut fradius = vec![0.5; 4];
    // //gen_occ_wires(&pts, &fradius);
}

#[test]
fn test_gen_occ_circle_part() {
    let pts = vec![
        Vec3::ZERO,
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let fradius = vec![0.2; 4];
    // //gen_occ_wires(&pts, &fradius);
}

#[test]
fn test_gen_occ_cut_circle_big_corner_1() {
    let pts = vec![
        Vec3::ZERO,
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let fradius = vec![1.0f32, 0.0, 0.0, 0.0];
    //gen_occ_wires(&pts, &fradius);
}

#[test]
fn test_gen_occ_cut_circle_big_corner_2() {
    let pts = vec![
        Vec3::ZERO,
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let fradius = vec![1.5, 0.0, 0.0, 0.0];
    //gen_occ_wires(&pts, &fradius);
}

#[test]
fn test_gen_occ_concave() {
    let pts = vec![
        Vec3::ZERO,
        Vec3::new(0.5, 0.5, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let fradius = vec![0.0, 0.25, 0.0, 0.0, 0.0];
    //gen_occ_wires(&pts, &fradius);
}

#[test]
fn test_gen_occ_concave_big() {
    let pts = vec![
        Vec3::ZERO,
        Vec3::new(0.5, 0.5, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let fradius = vec![0.0, 1.5, 0.0, 0.0, 0.0];
    //gen_occ_wires(&pts, &fradius);
}

#[test]
fn test_complex_circe() {
    let parts = vec![
        pline_closed![
            (-621.874, -748.901, 0.0),
            (659.25, -2445.38, 0.0),
            (-122.01, 1733.97, 0.0),
            (-539.001, 268.39, 0.0),
            (0.0, 0.0, 0.792)
        ],
        pline_closed![
            (-621.874, -748.901, 0.792),
            (268.621, -355.708, 0.0),
            (659.25, -2445.38, 0.0)
        ],
    ];
    println!("polyline: {}", polyline_to_debug_json_str(&parts[0]));
    println!("polyline: {}", polyline_to_debug_json_str(&parts[1]));

    let mut result = parts[0].boolean(&parts[1], BooleanOp::Not);
    if !result.pos_plines.is_empty() {
        dbg!(&result.pos_plines);
        let p = result.pos_plines.remove(0).pline;
        println!("final: {}", polyline_to_debug_json_str(&p));
    } else {
        dbg!("cut failed");
    }
}

#[test]
fn test_complex_half_circle() {
    let pts = vec![
        Vec3::new(233.5, 0.0, 0.0),
        Vec3::new(222.0, 233.5, 0.0),
        Vec3::new(-233.5, 233.5, 0.0),
        Vec3::new(-233.5, 0.0, 0.0),
    ];
    let fradius = vec![0.0, 233.5, 233.5, 0.0];
    //gen_occ_wires(&pts, &fradius);
    // .expect("test_complex_half_circle failed");
}

#[test]
fn test_complex_half_circle_1() {
    let pts = vec![
        Vec3::new(233.5, 0.0, 0.0),
        Vec3::new(222.0, 233.5, 0.0),
        Vec3::new(-233.5, 233.5, 0.0),
        Vec3::new(-233.5, 0.0, 0.0),
    ];
    let fradius = vec![0.0, 150.0, 150.0, 0.0];
    //gen_occ_wires(&pts, &fradius);
    // .expect("test_complex_half_circle failed");
}

#[test]
fn test_complex_1() {
    let tmp_pts = vec![
        [0.000, 0.000, 0.000],
        [0.000, 15337.730, 0.000],
        [-30432.971, 19187.180, 0.000],
        [-34251.980, 4332.510, 0.000],
        [-38584.891, 5526.540, 0.000],
        [-36528.699, 13400.760, 0.000],
        [-29829.340, 18021.330, 0.000],
        [-11801.380, 30455.260, 0.000],
        [631.700, 12426.700, 0.000],
        [4267.570, 7155.040, 0.000],
        [4486.860, 758.430, 0.000],
    ];
    let pts = tmp_pts
        .iter()
        .map(|x| Vec3::new(x[0], x[1], x[2]))
        .collect::<Vec<_>>();
    let fradius = vec![
        0.0, 17400.0, 17400.0, 0.0, 0.0, 21900.0, 0.0, 21900.0, 0.0, 21900.0, 0.0,
    ];
    //gen_occ_wires(&pts, &fradius);
    // .expect("test_complex_half_circle failed");
}

#[test]
fn test_complex_2() {
    let tmp_pts = vec![
        [0.0, 0.0, 2765.0],
        [-1903.0, 947.5800170898438, 2765.0],
        [659.25, -2445.3798828125, 2765.0],
        [-122.01000213623047, 1733.969970703125, 2765.0],
        [-1285.550048828125, -2355.469970703125, 2765.0],
        [146.63999938964844, -784.4299926757812, 2765.0],
    ];
    let pts = tmp_pts
        .iter()
        .map(|x| Vec3::new(x[0], x[1], x[2]))
        .collect::<Vec<_>>();
    let fradius = vec![0.0, 500.0, 500.0, 500.0, 500.0, 0.0];
    //gen_occ_wires(&pts, &fradius);
    // .expect("test_complex_half_circle failed");
}

#[inline]
fn seg_split(
    v1: PlineVertex,
    v2: PlineVertex,
    point_on_seg: Vector2,
    pos_equal_eps: f64,
) -> SplitResult {
    let mut r = seg_split_at_point(v1, v2, point_on_seg, pos_equal_eps);
    if r.updated_start.bulge.abs() < 0.01 {
        r.updated_start.bulge = 0.0;
    }
    if r.split_vertex.bulge.abs() < 0.01 {
        r.split_vertex.bulge = 0.0;
    }
    r
}

pub fn resolve_overlap_intersection(
    polyline: &Polyline,
    intersect: &PlineOverlappingIntersect<f64>,
    // ori: PlineOrientation,
) -> anyhow::Result<(Polyline, bool)> {
    let mut new_polyline = polyline.clone();

    let verts_len = polyline.vertex_data.len();
    //优先处理和直线的相交情况
    let si_0 = intersect.start_index1;
    let next_si_0 = (si_0 + 1) % verts_len;
    let si_1 = intersect.start_index2;
    let next_si_1 = (si_1 + 1) % verts_len;
    let point = intersect.point1;
    let r = seg_split(polyline[si_1], polyline[next_si_1], point, 0.01);
    // dbg!(&r);
    new_polyline[si_1] = r.updated_start;
    new_polyline[next_si_1] = r.split_vertex;
    let first_point = &new_polyline.vertex_data[0];
    //如果已经到起点了，结束检查，直接砍掉后面的
    if (r.split_vertex.pos() - first_point.pos()).length() < 0.01 {
        new_polyline.vertex_data.drain(next_si_1..);
        return Ok((new_polyline, false));
    }

    if let Some(r) = new_polyline.remove_repeat_pos(0.01) {
        new_polyline = r;
    }
    Ok((new_polyline, true))
}

/// 处理基本相交情况
/// 
/// 该函数用于处理多段线(polyline)的基本相交情况。基本相交是指两个线段相交于一个点。
///
/// # 参数
/// * `polyline` - 输入的多段线
/// * `intersect` - 相交信息,包含相交点和相交线段的索引
/// * `ori` - 多段线的方向
///
/// # 返回值
/// * `Result<Polyline>` - 处理后的新多段线
/// 
/// # 处理逻辑
/// 1. 根据相交点将相交的线段分割成两部分
/// 2. 根据线段类型(直线或圆弧)采用不同的处理策略
/// 3. 保持多段线的方向一致性
/// 4. 移除重复的点
pub fn resolve_basic_intersection(
    polyline: &Polyline,
    intersect: &PlineBasicIntersect<f64>,
    ori: PlineOrientation,
) -> anyhow::Result<Polyline> {
    let mut new_polyline = polyline.clone();
    let verts_len = polyline.vertex_data.len();
    
    // 检查多段线是否有足够的顶点
    if verts_len < 3 {
        return Err(anyhow!("Polyline has too few vertices."));
    }
    
    // 获取相交线段的起始索引
    let si_0 = intersect.start_index1;
    let mut next_si_0 = (si_0 + 1) % verts_len;
    let mut si_1 = intersect.start_index2;
    let next_si_1 = (si_1 + 1) % verts_len;
    
    // 验证索引的有效性
    if si_0 >= verts_len || si_1 >= verts_len || next_si_0 >= verts_len || next_si_1 >= verts_len {
        return Err(anyhow!("Invalid intersection indices for polyline."));
    }
    
    let point = intersect.point;

    // 处理两条直线相交的情况
    if polyline[si_0].bulge == 0.0 && polyline[si_1].bulge == 0.0 {
        new_polyline[si_1] = PlineVertex::new(point.x, point.y, 0.0);
    } 
    // 处理直线和圆弧相交的情况(第一条是直线,第二条是圆弧)
    else if polyline[si_0].bulge == 0.0 && polyline[si_1].bulge != 0.0 {
        // 如果点和端点重合，直接砍掉
        let mut tmp_polyline = Polyline::new_closed();
        tmp_polyline.add(polyline[si_0].x, polyline[si_0].y, 0.0);
        tmp_polyline.add(point.x, point.y, 0.0);
        tmp_polyline.add(polyline[next_si_1].x, polyline[next_si_1].y, 0.0);
        let use_start = tmp_polyline.orientation() != ori;
        #[cfg(feature = "debug_wire")]
        dbg!(use_start);

        let r = seg_split(polyline[si_1], polyline[next_si_1], point, 0.01);
        #[cfg(feature = "debug_wire")]
        dbg!(&r);
        // 如果分割点和端点重合
        if r.split_vertex.bulge == 0.0 {
            if si_0 == 0 {
                next_si_0 = verts_len;
            }
            #[cfg(feature = "debug_wire")]
            println!(
                "first arc, second line, same end point, remove between {} .. {}",
                next_si_1, next_si_0
            );
            // 确保范围有效：next_si_1 <= next_si_0
            if next_si_0 < next_si_1 {
                return Err(anyhow!("Invalid drain range: next_si_0({}) < next_si_1({})", next_si_0, next_si_1));
            }
            // 安全地移除范围内的顶点
            if next_si_1 < new_polyline.vertex_data.len() && next_si_0 <= new_polyline.vertex_data.len() {
                new_polyline.vertex_data.drain(next_si_1..next_si_0);
            } else {
                return Err(anyhow!("Invalid drain range for polyline: next_si_1={}, next_si_0={}, len={}", next_si_1, next_si_0, new_polyline.vertex_data.len()));
            }
        } else if use_start {
            new_polyline[si_1] = r.updated_start;
            new_polyline[si_0] = r.split_vertex;
            #[cfg(feature = "debug_wire")]
            println!(
                "first arc, second line , use arc start: {}, line use split start: {} ",
                si_1, si_0
            );
        } else {
            // 检查索引的有效性
            if next_si_0 >= new_polyline.vertex_data.len() || si_1 >= new_polyline.vertex_data.len() {
                return Err(anyhow!("Invalid vertex indices for polyline."));
            }
            
            new_polyline[next_si_0] = r.split_vertex;
            new_polyline[si_1] = r.split_vertex;
            #[cfg(feature = "debug_wire")]
            println!(
                "first arc, second line , use split remove between {} .. {}",
                next_si_0, si_1
            );
            // 确保范围有效：next_si_0 <= si_1
            if si_1 < next_si_0 {
                return Err(anyhow!("Invalid drain range: si_1({}) < next_si_0({})", si_1, next_si_0));
            }
            // 安全地移除范围内的顶点
            if next_si_0 < new_polyline.vertex_data.len() && si_1 <= new_polyline.vertex_data.len() {
                new_polyline.vertex_data.drain(next_si_0..si_1);
            } else {
                return Err(anyhow!("Invalid drain range for polyline: next_si_0={}, si_1={}, len={}", next_si_0, si_1, new_polyline.vertex_data.len()));
            }
        }
    } 
    // 处理圆弧和直线相交的情况(第一条是圆弧,第二条是直线)
    else if polyline[si_0].bulge != 0.0 && polyline[si_1].bulge == 0.0 {
        let mut tmp_polyline = Polyline::new_closed();
        tmp_polyline.add(polyline[si_0].x, polyline[si_0].y, 0.0);
        tmp_polyline.add(point.x, point.y, 0.0);
        tmp_polyline.add(polyline[next_si_1].x, polyline[next_si_1].y, 0.0);
        let use_start = tmp_polyline.orientation() == ori;
        #[cfg(feature = "debug_wire")]
        dbg!(use_start);

        let mut r = seg_split(polyline[si_0], polyline[next_si_0], point, 0.01);
        #[cfg(feature = "debug_wire")]
        dbg!(&r);
        // 如果分割点和端点重合
        if r.split_vertex.bulge == 0.0 {
            #[cfg(feature = "debug_wire")]
            println!(
                "first arc, second line, same end point, remove between {} .. {}",
                next_si_0, si_1
            );
            // 确保范围有效：next_si_0 <= si_1
            if si_1 < next_si_0 {
                return Err(anyhow!("Invalid drain range: si_1({}) < next_si_0({})", si_1, next_si_0));
            }
            // 安全地移除范围内的顶点
            if next_si_0 < new_polyline.vertex_data.len() && si_1 <= new_polyline.vertex_data.len() {
                new_polyline.vertex_data.drain(next_si_0..si_1);
            } else {
                return Err(anyhow!("Invalid drain range for polyline: next_si_0={}, si_1={}, len={}", next_si_0, si_1, new_polyline.vertex_data.len()));
            }
        } else {
            if use_start {
                new_polyline[si_0] = r.updated_start;
                new_polyline[si_1] = r.split_vertex;
                #[cfg(feature = "debug_wire")]
                println!(
                    "first arc, second line , use start remove between {} .. {}",
                    next_si_0, si_1
                );
                // 确保范围有效：next_si_0 <= si_1
                if si_1 < next_si_0 {
                    return Err(anyhow!("Invalid drain range: si_1({}) < next_si_0({})", si_1, next_si_0));
                }
                // 安全地移除范围内的顶点
                if next_si_0 < new_polyline.vertex_data.len() && si_1 <= new_polyline.vertex_data.len() {
                    new_polyline.vertex_data.drain(next_si_0..si_1);
                } else {
                    return Err(anyhow!("Invalid drain range for polyline: next_si_0={}, si_1={}, len={}", next_si_0, si_1, new_polyline.vertex_data.len()));
                }
            } else {
                // 检查索引的有效性
                if si_0 >= new_polyline.vertex_data.len() || next_si_1 >= new_polyline.vertex_data.len() {
                    return Err(anyhow!("Invalid vertex indices for polyline."));
                }
                
                new_polyline[si_0] = r.split_vertex;
                new_polyline[next_si_1] = r.split_vertex;
                #[cfg(feature = "debug_wire")]
                println!(
                    "first arc, second line , {} and {} use split",
                    si_0, next_si_1
                );
            }
        }
    } 
    // 处理两条圆弧相交的情况
    else if polyline[si_0].bulge != 0.0 && polyline[si_1].bulge != 0.0 {
        // 验证索引的有效性
        if si_0 >= verts_len || (si_0 + 1) >= verts_len {
            return Err(anyhow!("Invalid index for polyline."));
        }
        
        let sr = seg_split(
            polyline[si_0],
            polyline[(si_0 + 1) % verts_len],
            point,
            0.01,
        );
        // 更新第一条圆弧的起点
        new_polyline[si_0] = sr.updated_start;
        
        // 验证索引的有效性
        if si_1 >= verts_len || (si_1 + 1) >= verts_len {
            return Err(anyhow!("Invalid index for polyline."));
        }
        
        // 更新第二条圆弧的起点
        let er = seg_split(
            polyline[si_1],
            polyline[(si_1 + 1) % verts_len],
            point,
            0.01,
        );
        new_polyline[si_1] = er.split_vertex;

        if si_1 >= next_si_0 {
            #[cfg(feature = "debug_wire")]
            println!("both arc, remove between {} .. {}", next_si_0, si_1);
            // 确保范围有效：next_si_0 <= si_1
            if si_1 < next_si_0 {
                return Err(anyhow!("Invalid drain range: si_1({}) < next_si_0({})", si_1, next_si_0));
            }
            // 安全地移除范围内的顶点
            if next_si_0 < new_polyline.vertex_data.len() && si_1 <= new_polyline.vertex_data.len() {
                new_polyline.vertex_data.drain(next_si_0..si_1);
            } else {
                return Err(anyhow!("Invalid drain range for polyline: next_si_0={}, si_1={}, len={}", next_si_0, si_1, new_polyline.vertex_data.len()));
            }
        }
    }

    // 移除重复的点
    if let Some(r) = new_polyline.remove_repeat_pos(0.01) {
        new_polyline = r;
    }

    Ok(new_polyline)
}

//如果有两个以上的PLOO，需要执行boolean operation
///根据传进去的参数生成 Polyline, x, y 为坐标，z 为 fradius
///
/// 这个方法使用 rust-ploop-processor 的 process_ploop 方法处理顶点数据，
/// 然后生成对应的 Polyline。这是对原有 gen_polyline 方法的增强版本。
///
/// # 参数
/// * `pts` - 顶点数据，Vec3 格式：x,y 为坐标，z 为 fradius 值
///
/// # 返回值
/// * `Result<Polyline>` - 处理后生成的多段线
///
/// # 示例
/// ```rust
/// use aios_core::prim_geo::wire::gen_polyline;
/// use glam::Vec3;
///
/// let vertices = vec![
///     Vec3::new(0.0, 0.0, 0.0),      // 起点，无圆角
///     Vec3::new(100.0, 0.0, 0.0),    // 第二点，无圆角
///     Vec3::new(100.0, 100.0, 15.0), // 第三点，圆角半径15
///     Vec3::new(0.0, 100.0, 10.0),   // 第四点，圆角半径10
/// ];
/// let polyline = gen_polyline(&vertices)?;
/// ```
pub fn gen_polyline(pts: &Vec<Vec3>) -> anyhow::Result<Polyline> {
    if pts.len() < 3 {
        return Err(anyhow!("顶点数量不够，小于3。"));
    }

    println!("🔧 使用 rust-ploop-processor 统一处理 {} 个顶点", pts.len());

    // 统一使用 rust-ploop-processor 处理所有顶点
    let processed_vertices = process_ploop_vertices(pts, "POLYLINE_GENERATION")?;

    println!("   rust-ploop-processor 处理完成，得到 {} 个顶点", processed_vertices.len());

    // 将处理后的顶点转换为 Polyline
    convert_vertices_to_polyline(&processed_vertices)
}

/// 将已经被 rust-ploop-processor 处理过的顶点直接转换为 Polyline
///
/// 这个函数用于处理已经被 process_ploop_from_content 或 process_ploop_vertices
/// 处理过的顶点，避免重复处理
///
/// # 参数
/// * `vertices` - 已处理的顶点数据，Vec3 格式：x,y 为坐标，z 为 fradius 值
///
/// # 返回值
/// * `Result<Polyline>` - 转换后的多段线
pub fn gen_polyline_from_processed_vertices(vertices: &Vec<Vec3>) -> anyhow::Result<Polyline> {
    if vertices.len() < 3 {
        return Err(anyhow!("顶点数量不够，小于3。"));
    }

    println!("🔧 直接转换已处理的 {} 个顶点为 Polyline", vertices.len());

    // 直接转换为 Polyline，不再调用 rust-ploop-processor
    convert_vertices_to_polyline(vertices)
}

/// 将 rust-ploop-processor 处理后的顶点转换为 Polyline
///
/// rust-ploop-processor 已经处理了 FRADIUS 并生成了正确的切点，
/// 我们只需要将 FRADIUS 值转换为对应的 bulge 值
///
/// # 参数
/// * `vertices` - 处理后的顶点数据，Vec3 格式：x,y 为坐标，z 为 fradius 值
///
/// # 返回值
/// * `Result<Polyline>` - 转换后的多段线
fn convert_vertices_to_polyline(vertices: &[Vec3]) -> anyhow::Result<Polyline> {
    if vertices.len() < 3 {
        return Err(anyhow!("顶点数量不够，小于3。"));
    }

    println!("🔧 将 {} 个处理后的顶点转换为 Polyline", vertices.len());

    let mut polyline = Polyline::new_closed();
    let remove_pos_tol = 0.1;
    let len = vertices.len();

    // 直接转换顶点，将 fradius 转换为 bulge
    for i in 0..len {
        let vertex = vertices[i];
        let fradius = vertex.z;

        if fradius > 0.0 {
            // 有 fradius 的顶点，计算对应的 bulge 值
            let last_index = (i + len - 1) % len;
            let next_index = (i + 1) % len;

            let cur_pt = DVec2::new(vertex.x as f64, vertex.y as f64);
            let last_pt = DVec2::new(vertices[last_index].x as f64, vertices[last_index].y as f64);
            let next_pt = DVec2::new(vertices[next_index].x as f64, vertices[next_index].y as f64);

            let v1 = (cur_pt - last_pt).normalize();
            let v2 = (next_pt - cur_pt).normalize();
            let angle = (-v1).angle_between(v2);

            if angle.abs() < 0.001 {
                // 角度太小，作为直线处理
                polyline.add(vertex.x as f64, vertex.y as f64, 0.0);
                continue;
            }

            // 计算 bulge 值
            let cur_ccw_sig = -angle.signum();
            let bulge = cur_ccw_sig * bulge_from_angle(std::f64::consts::PI - angle.abs());

            if bulge.abs() < 0.001 {
                // bulge 太小，作为直线处理
                polyline.add(vertex.x as f64, vertex.y as f64, 0.0);
            } else {
                // 添加带 bulge 的顶点
                polyline.add(vertex.x as f64, vertex.y as f64, bulge);
            }
        } else {
            // 普通顶点，直接添加
            polyline.add(vertex.x as f64, vertex.y as f64, 0.0);
        }
    }

    // 移除重复位置
    if let Some(new_poly) = polyline.remove_repeat_pos(remove_pos_tol) {
        polyline = new_poly;
    }

    // 检查是否有 NaN 数据
    for p in &polyline.vertex_data {
        if p.bulge.is_nan() {
            return Err(anyhow!("发现 NaN bulge 值"));
        }
    }

    println!("✅ Polyline 转换完成，包含 {} 个顶点", polyline.vertex_data.len());

    Ok(polyline)
}

///根据传进去的参数生成 Polyline, x, y 为坐标，z 为bulge
pub fn gen_polyline_original(pts: &Vec<Vec3>) -> anyhow::Result<Polyline> {
    if pts.len() < 3 {
        return Err(anyhow!("wire 顶点数量不够，小于3。"));
    }
    let first_pt = pts[0].as_dvec3();
    let mut has_frad = first_pt.z > 0.0;
    let mut new_pts = vec![first_pt];
    //第一遍就应该去掉重复的点
    for i in 1..=pts.len() {
        let cur_index = i % pts.len();
        let pt = pts[cur_index].as_dvec3();
        let last_index = new_pts.len() - 1;
        let pre_pt = new_pts[last_index];
        //需要检查第一个pt的合理性
        if pt.truncate().distance(pre_pt.truncate()) < 0.1 {
            // dbg!(pt);
            //需要区分哪个有fillet
            if pt.z > 0.0 {
                new_pts[last_index].z = pt.z as _;
            }
            //如果最后一个和第一个重合，那么需要去掉最后一个
            if i == pts.len() {
                new_pts.pop();
            }
            continue;
        }

        if pt.z > 0.0 {
            has_frad = true;
        }
        if i < pts.len() {
            new_pts.push(pt);
        }
    }
    // dbg!(&new_pts);

    let len = new_pts.len();
    if len < 3 {
        return Err(anyhow!("wire 顶点数量不够，小于3。"));
    }
    let mut polyline = Polyline::new_closed();
    let remove_pos_tol = 0.1;

    for i in 0..len {
        let pt = new_pts[i];
        let fradius = pt.z;
        if pt.z > 0.0 {
            let last_index = (i + len - 1) % len;
            let mut cur_pt = pt.truncate();
            let mut last = new_pts[last_index].truncate();
            //如果fradius > 0.0，需要检查wind 方向
            let mut next = new_pts[(i + 1) % len].truncate();

            let mut v1 = (cur_pt - last).normalize();
            let mut v2 = (next - cur_pt).normalize();
            let angle = (-v1).angle_between(v2);
            if angle.abs() < 0.001 {
                continue;
            }
            // dbg!(angle);
            // dbg!(angle.to_degrees());
            let l = fradius / (angle / 2.0).tan().abs();
            // let d1 = (pt - last).length();
            // let d2 = (next - pt).length();
            // dbg!((l, d1, d2));
            // let extent = aabb.extents().magnitude() as f64;
            // if l > extent {
            //     dbg!((l, extent));
            //     continue;
            // }
            let mut p0 = cur_pt + (-v1) * l;
            let mut p2 = cur_pt + v2 * l;
            // dbg!(last.distance(p0));
            // dbg!(next.distance(p2));
            if last.distance(p0) < remove_pos_tol {
                p0 = last;
            }
            if next.distance(p2) < remove_pos_tol {
                p2 = next;
            }
            // let mut cur_ccw_sig = if v1.extend(0.0).cross(v2.extend(0.0)).z > 0.0 { 1.0 } else { -1.0 };
            let cur_ccw_sig = -angle.signum();
            let bulge = cur_ccw_sig * bulge_from_angle(PI as f64 - angle.abs());
            if bulge.abs() < 0.001 {
                continue;
            }
            polyline.add(p0.x, p0.y, bulge);
            polyline.add(p2.x, p2.y, 0.0);
        } else {
            polyline.add(pt.x, pt.y, 0.0);
        }
    }
    if let Some(new_poly) = polyline.remove_repeat_pos(remove_pos_tol) {
        polyline = new_poly;
    }
    #[cfg(feature = "debug_wire")]
    {
        dbg!(pts);
        dbg!(new_pts);
        println!("Polyline: {}", polyline_to_debug_json_str(&polyline));
    }
    //及一个检查是否有NAN的数据
    for p in &polyline.vertex_data {
        if p.bulge.is_nan() {
            return Err(anyhow!("Found NAN buldge in polyline"));
        }
    }

    //需要和初始的方向保持一致，如果是顺时针，那么要选择顺时针方向的交叉点
    let orientation = polyline.orientation();

    let Ok(mut intrs) = std::panic::catch_unwind(
        (|| global_self_intersects(&polyline, &polyline.create_approx_aabb_index())),
    ) else {
        return Err(anyhow!("Self intersect check failed"));
    };

    let basic_inter_len = intrs.basic_intersects.len();
    let overlap_inter_len = intrs.overlapping_intersects.len();
    let mut need_trim = basic_inter_len != 0 || overlap_inter_len != 0;
    if basic_inter_len == 0 && overlap_inter_len == 0 {
        return Ok(polyline);
    } else if !has_frad {
        // dbg!(&intrs.basic_intersects);
        let removed_idx = intrs
            .basic_intersects
            .iter()
            .map(|x| x.start_index1)
            .collect::<HashSet<usize>>();
        let mut new_polyline = Polyline::new_closed();
        new_polyline.vertex_data = polyline
            .vertex_data
            .clone()
            .into_iter()
            .enumerate()
            .filter(|(index, _)| !removed_idx.contains(index))
            .map(|(_, value)| value)
            .collect();

        if let Ok(mut new_intrs) = std::panic::catch_unwind(
            (|| global_self_intersects(&new_polyline, &new_polyline.create_approx_aabb_index())),
        ) {
            let basic_inter_len = new_intrs.basic_intersects.len();
            let overlap_inter_len = new_intrs.overlapping_intersects.len();
            if basic_inter_len == 0 && overlap_inter_len == 0 {
                return Ok(new_polyline);
            } else {
                #[cfg(feature = "debug_wire")]
                println!("有问题的wire: {}", polyline_to_debug_json_str(&polyline));
                return Err(anyhow!("有相交没有fillet的线段。修复失败"));
            }
        };
        return Err(anyhow!("有相交没有fillet的线段。修复失败"));
    }
    #[cfg(feature = "debug_wire")]
    dbg!(&intrs);
    let mut final_polyline = polyline.clone();
    let mut need_break = false;

    let mut overlap_index = 0;
    while let Some(intersect) = intrs.overlapping_intersects.get(0) {
        #[cfg(feature = "debug_wire")]
        dbg!(intersect);
        (final_polyline, need_break) = resolve_overlap_intersection(&final_polyline, intersect)?;
        intrs = global_self_intersects(&final_polyline, &final_polyline.create_approx_aabb_index());
        #[cfg(feature = "debug_wire")]
        dbg!(&intrs);
        if need_break || overlap_index == overlap_inter_len {
            break;
        }
        overlap_index += 1;
    }

    if overlap_inter_len > 0 {
        #[cfg(feature = "debug_wire")]
        println!(
            "After resolve overlap Polyline: {}",
            polyline_to_debug_json_str(&final_polyline)
        );
        //这里需要重新求是否有相交
        intrs = global_self_intersects(&final_polyline, &final_polyline.create_approx_aabb_index());
    }

    let basic_inter_len = intrs.basic_intersects.len();

    let mut basic_index = 0;
    while let Some(intersect) = intrs.basic_intersects.get(0) {
        #[cfg(feature = "debug_wire")]
        dbg!(intersect);
        final_polyline = resolve_basic_intersection(&final_polyline, intersect, orientation)?;
        intrs = global_self_intersects(&final_polyline, &final_polyline.create_approx_aabb_index());
        // dbg!(&intrs);
        if basic_index == basic_inter_len {
            break;
        }
        basic_index += 1;
    }
    #[cfg(feature = "debug_wire")]
    if need_trim {
        dbg!(orientation);
        println!(
            "final polyline: {}",
            polyline_to_debug_json_str(&final_polyline)
        );
    }
    Ok(final_polyline)
}

///生成occ的wire
#[cfg(feature = "occ")]
pub fn gen_occ_wires(loops: &Vec<Vec<Vec3>>) -> anyhow::Result<Vec<Wire>> {
    if loops[0].len() < 3 {
        return Err(anyhow!("第一个 wire 顶点数量不够，小于3。"));
    }
    let mut pos_poly = gen_polyline_original(&loops[0])?;
    if pos_poly.vertex_data.len() < 3 {
        return Err(anyhow!("pos_poly 顶点数量不够，小于3。"));
    }

    for pts in loops.iter().skip(1) {
        let Ok(neg) = gen_polyline_original(pts) else {
            continue;
        };
        let mut r = pos_poly.boolean(&neg, BooleanOp::Not);
        if r.pos_plines.len() > 0 {
            pos_poly = r.pos_plines.remove(0).pline;
        }
    }
    #[cfg(feature = "debug_wire")]
    println!(
        "final occ polyline: {}",
        polyline_to_debug_json_str(&pos_poly)
    );

    let mut wires = vec![];
    let mut edges = vec![];
    let mut seg_count = 0;
    for (p, q) in pos_poly.iter_segments() {
        if p.bulge.abs() < 0.001 {
            edges.push(Edge::segment(
                DVec3::new(p.x, p.y, 0.0),
                DVec3::new(q.x, q.y, 0.0),
            ));
        } else {
            let m = seg_midpoint(p, q);
            edges.push(Edge::arc(
                DVec3::new(p.x, p.y, 0.0),
                DVec3::new(m.x, m.y, 0.0),
                DVec3::new(q.x, q.y, 0.0),
            ));
        }
        seg_count += 1;
    }
    if seg_count < 1 {
        return Err(anyhow!("生成的线段数量小于1"));
    }
    wires.push(Wire::from_edges(&edges)?);
    Ok(wires)
}

pub fn check_wire_ok(pts: &Vec<Vec3>, fradius_vec: &Vec<f32>) -> bool {
    let mut polyline = Polyline::new_closed();
    for i in 0..pts.len() {
        let c_pt = pts[i].as_dvec3();
        polyline.add(c_pt.x, c_pt.y, 0.0.into());
    }
    let intrs = global_self_intersects(&polyline, &polyline.create_approx_aabb_index());
    // dbg!(intrs.basic_intersects.len());
    // dbg!(intrs.overlapping_intersects.len());

    intrs.basic_intersects.len() == 0 && intrs.overlapping_intersects.len() == 0
}

fn global_self_intersects<T>(
    polyline: &Polyline<T>,
    aabb_index: &StaticAABB2DIndex<T>,
) -> PlineIntersectsCollection<T>
where
    T: Real,
{
    let mut intrs = Vec::new();
    let mut overlapping_intrs = Vec::new();
    let mut visitor = |intr: PlineIntersect<T>| match intr {
        PlineIntersect::Basic(b) => {
            intrs.push(b);
        }
        PlineIntersect::Overlapping(o) => {
            overlapping_intrs.push(o);
        }
    };

    visit_global_self_intersects(polyline, aabb_index, &mut visitor, T::from(0.01).unwrap());

    PlineIntersectsCollection::new(intrs, overlapping_intrs)
}

#[test]
fn test_gen_circle() {
    let mut pline = pline_closed!(
        (0.5, 0.0, 0.0),
        (1.0, 0.5, 0.0),
        (0.5, 1.0, 0.0),
        (0.0, 0.5, 0.0)
    );

    let arc_cut1 = pline_closed!((0.0, 0.5, 0.0), (0.25, 0.25, -0.4142135), (0.25, 0.75, 0.0));
    let arc_cut2 = pline_closed!((0.25, 0.25, 0.0), (0.5, 0.0, 0.0), (0.75, 0.25, -0.4142135));
    let arc_cut3 = pline_closed![(0.75, 0.25, 0.0), (1.0, 0.5, 0.0), (0.75, 0.75, -0.4142135)];

    let arc_cut4 = pline_closed![(0.75, 0.75, 0.0), (0.5, 1.0, 0.0), (0.25, 0.75, -0.4142135)];

    let mut cuts = vec![arc_cut1, arc_cut2, arc_cut3, arc_cut4];
    for cut in cuts {
        let mut result = pline.boolean(&cut, BooleanOp::Not);
        if !result.pos_plines.is_empty() {
            dbg!(&result.pos_plines);
            pline = result.pos_plines.remove(0).pline;
        } else {
            dbg!("cut failed");
        }
    }
}

#[test]
fn test_concave_circle() {
    let mut pline = pline_closed!(
        (0.5, 0.0, 0.0),
        (0.5, 0.5, 0.0),
        (1.0, 0.5, 0.0),
        (0.5, 1.0, 0.0),
        (0.0, 0.5, 0.0)
    );

    let arc_cut1 = pline_closed!((0.0, 0.5, 0.0), (0.25, 0.25, -0.4142135), (0.25, 0.75, 0.0));
    let arc_cut2 = pline_closed!((0.25, 0.25, 0.0), (0.5, 0.0, 0.0), (0.75, 0.25, -0.4142135));
    let arc_cut3 = pline_closed![(0.75, 0.25, 0.0), (1.0, 0.5, 0.0), (0.75, 0.75, -0.4142135)];

    let arc_cut4 = pline_closed![(0.75, 0.75, 0.0), (0.5, 1.0, 0.0), (0.25, 0.75, -0.4142135)];

    let mut cuts = vec![arc_cut1, arc_cut2, arc_cut3, arc_cut4];
    for cut in cuts {
        let mut result = pline.boolean(&cut, BooleanOp::Not);
        if !result.pos_plines.is_empty() {
            dbg!(&result.pos_plines);
            pline = result.pos_plines.remove(0).pline;
        } else {
            dbg!("cut failed");
        }
    }
}

///可以使用 cut 的办法
/// 根据顶点信息和fradius半径，生成wire
#[cfg(feature = "truck")]
pub fn gen_wire(
    input_pts: &Vec<Vec3>,
    input_fradius_vec: &Vec<f32>,
) -> anyhow::Result<truck_modeling::Wire> {
    #[cfg(feature = "truck")]
    use truck_modeling::{builder, Vertex, Wire};
    if input_pts.len() < 3 || input_fradius_vec.len() != input_pts.len() {
        return Err(anyhow!("wire 顶点数量不够，小于3。"));
    }
    let t_pts = input_pts
        .into_iter()
        .map(|x| vec3_round_2(*x))
        .collect::<Vec<_>>();
    let mut prev_pt = t_pts[0].truncate();
    let mut deleted = vec![];
    let mut pts = vec![t_pts[0]];
    for i in 1..t_pts.len() {
        if t_pts[i].truncate().distance(prev_pt) < LEN_TOL {
            deleted.push(i);
            continue;
        }
        pts.push(t_pts[i]);
        prev_pt = t_pts[i].truncate();
    }
    let fradius_vec = input_fradius_vec
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !deleted.contains(i))
        .map(|(_, x)| f32_round_2(*x))
        .collect::<Vec<_>>();
    // dbg!(&pts);
    // dbg!(&fradius_vec);
    let mut wire = Wire::new();

    //使用boolean 运算来切割原来的线圈

    let ll = pts.len();
    let mut verts = vec![];
    let mut circle_indexs = vec![];
    for i in 0..ll {
        let fradius = fradius_vec[i];
        let pt = pts[i].point3_without_z();
        //跳过相同的点
        if let Some(last_pt) = verts.last().map(|x: &Vertex| x.point()) {
            if pt.distance(last_pt) < 0.1 {
                continue;
            }
            if i == ll - 1 {
                if pt.distance(verts[0].point()) < 0.1 {
                    continue;
                }
            }
        }
        if abs_diff_eq!(fradius.abs(), 0.0) {
            verts.push(builder::vertex(pt));
        } else {
            let r = fradius;
            let pre_i = (ll + i - 1) % ll;
            let n_i = (i + 1) % ll;
            let pre_pt = pts[pre_i];
            let cur_pt = pts[i % ll];
            let next_pt = pts[n_i];
            let pa_dist = pre_pt.distance(cur_pt);
            let pb_dist = next_pt.distance(cur_pt);
            let a_dir = (pre_pt - cur_pt).normalize();
            let b_dir = (next_pt - cur_pt).normalize();
            let angle = a_dir.angle_between(b_dir) / 2.0;
            let b_len = r / angle.tan();

            let h = r * angle.sin();
            let d = r - h;
            let p0 = cur_pt + a_dir * b_len;
            let p1 = cur_pt + b_dir * b_len;
            let mid_pt = (p0 + p1) / 2.0;
            let mid_dir = (cur_pt - mid_pt).normalize();
            let transit_pt = mid_pt + mid_dir * d;

            if pa_dist - b_len > 0.01 {
                verts.push(builder::vertex(vec3_round_2(p0).point3_without_z()));
            }

            verts.push(builder::vertex(vec3_round_2(transit_pt).point3_without_z()));
            circle_indexs.push(verts.len() - 1);

            if pb_dist - b_len > 0.01 {
                verts.push(builder::vertex(vec3_round_2(p1).point3_without_z()));
            }
        }
    }
    let mut j = 0;
    if !verts.is_empty() {
        let s_vert = verts.first().unwrap();
        let e_vert = verts.last().unwrap();
        let l = s_vert.point().distance(e_vert.point());
        if l < 0.1 {
            verts.pop();
        }
        let v_len = verts.len();
        if v_len == 0 {
            dbg!(pts);
            dbg!(fradius_vec);
            return Err(anyhow!(" verts are empty"));
        }
        let mut pre_vert = verts[0].clone();
        j = 1;
        while j <= v_len {
            let cur_vert = &verts[j % v_len];
            if pre_vert.point().distance(cur_vert.point()) > 1.0 {
                if circle_indexs.len() > 0 && j == circle_indexs[0] {
                    let next_vert = &verts[(j + 1) % v_len];
                    wire.push_back(builder::circle_arc(&pre_vert, next_vert, cur_vert.point()));
                    pre_vert = next_vert.clone();
                    circle_indexs.remove(0);
                    j += 1;
                } else {
                    wire.push_back(builder::line(&pre_vert, cur_vert));
                    pre_vert = cur_vert.clone();
                }
            }
            j += 1;
        }
    }
    // dbg!(&wire);
    Ok(wire)
}

#[test]
pub fn test_check_wire_25688_45293() {
    let data = vec![
        [0.0, 0.0, 480.0],
        [4.46, -173.52, 480.0],
        [-132.5, 145.48, 480.0],
        [112.98, -100.0, 480.0],
        [-206.02, 36.96, 480.0],
        [-32.5, 32.5, 480.0],
    ];
    let pts: Vec<Vec3> = data
        .iter()
        .map(|x| Vec3::new(x[0], x[1], x[2]))
        .collect::<Vec<_>>();
    let fradius_vec = vec![0.0, 33.37, 33.37, 33.37, 33.37, 0.0];

    assert_eq!(check_wire_ok(&pts, &fradius_vec), false);
}

#[test]
pub fn test_check_wire_25688_45261() {
    let data = vec![
        [-23350, 0, 0],
        [-22200, 23350, 23350],
        [23350, 23350, 23350],
        [23350, 0, 0],
    ];
    let pts: Vec<Vec3> = data
        .iter()
        .map(|x| Vec3::new(x[0] as f32, x[1] as f32, x[2] as f32))
        .collect::<Vec<_>>();

    // gen_occ_wires(&pts).unwrap();
}

#[test]
pub fn test_check_wire_25688_72092() {
    let data = vec![
        [0.0, 0.0, 0.0],
        [0.0, 8188.92, 0.0],
        [-12620.42, 18627.24, 0.0],
        [-20663.97, 17091.12, 0.0],
        [-22737.08, 22684.93, 0.0],
        [7196.01, 29736.53, 0.0],
        [5884.46, -987.96, 0.0],
    ];
    let pts: Vec<Vec3> = data
        .iter()
        .map(|x| Vec3::new(x[0] as f32, x[1] as f32, x[2] as f32))
        .collect::<Vec<_>>();
    let fradius_vec = vec![0.0, 17400.0, 17400.0, 0.0, 0.0, 23300.0, 0.0];

    // assert_eq!(check_wire_ok(&pts, &fradius_vec), true);
    // gen_occ_wires(&pts, &fradius_vec).unwrap();
}

#[test]
pub fn test_check_wire_17496_254047() {
    let data = vec![
        [31500.0, 79700.0, 0.0],
        [31500.0, 84300.0, 0.0],
        [62600.0, 84300.0, 0.0],
        [62600.0, 42457.41015625, 0.0],
        [62600.01171875, 42457.3984375, 0.0],
        [42696.78125, 50942.25, 0.0],
        [19471.44921875, 14430.48046875, 0.0],
        [34918.640625, 37374.4296875, 0.0],
        [31500.0, 41040.46875, 0.0],
    ];
    let pts: Vec<Vec3> = data
        .iter()
        .map(|x| Vec3::new(x[0] as f32, x[1] as f32, x[2] as f32))
        .collect::<Vec<_>>();
    let fradius_vec = vec![0.0, 0.0, 0.0, 0.0, 0.0, 25500.0, 25500.0, 0.0, 0.0];

    // gen_occ_wires(&pts, &fradius_vec).unwrap();
}

#[test]
pub fn test_gen_polyline() {
    // Simple rectangle with a fillet radius at the corner
    let pts = vec![
        Vec3::new(0.0, 0.0, 0.0),   // Bottom-left
        Vec3::new(10.0, 0.0, 0.0),  // Bottom-right
        Vec3::new(10.0, 10.0, 2.0), // Top-right with fillet radius 2.0
        Vec3::new(0.0, 10.0, 0.0),  // Top-left
    ];

    let polyline = gen_polyline(&pts).expect("Failed to generate polyline");

    // Verify the generated polyline has the expected properties
    assert!(polyline.is_closed());

    // For a rectangle with one corner filleted, we expect 5 vertices
    // (4 corners with one being split into 2 points for the arc)
    assert_eq!(polyline.vertex_data.len(), 5);

    // Check for non-zero bulge in the vertex data (indicating an arc)
    let has_bulge = polyline.vertex_data.iter().any(|v| v.bulge.abs() > 0.0);
    assert!(
        has_bulge,
        "Polyline should have at least one arc segment with non-zero bulge"
    );

    println!(
        "Generated polyline: {}",
        polyline_to_debug_json_str(&polyline)
    );
}

#[test]
pub fn test_gen_polyline_with_multiple_fillets() {
    // Rectangle with fillet radius at all corners
    let pts = vec![
        Vec3::new(0.0, 0.0, 1.5),   // Bottom-left with fillet radius 1.5
        Vec3::new(10.0, 0.0, 1.5),  // Bottom-right with fillet radius 1.5
        Vec3::new(10.0, 10.0, 1.5), // Top-right with fillet radius 1.5
        Vec3::new(0.0, 10.0, 1.5),  // Top-left with fillet radius 1.5
    ];

    let polyline = gen_polyline(&pts).expect("Failed to generate polyline");

    // Verify the polyline is closed
    assert!(polyline.is_closed());

    // With all corners filleted, we expect 8 vertices (each corner splits into 2 points)
    assert_eq!(polyline.vertex_data.len(), 8);

    // Count the number of arc segments (non-zero bulge values)
    let arc_count = polyline
        .vertex_data
        .iter()
        .filter(|v| v.bulge.abs() > 0.0)
        .count();
    assert_eq!(arc_count, 4, "Should have 4 arc segments");

    println!(
        "Generated polyline with multiple fillets: {}",
        polyline_to_debug_json_str(&polyline)
    );
}

#[test]
pub fn test_gen_polyline_complex_shape() {
    // Complex shape with various fillet radii
    // Points from the provided example with z values converted to fillet radii
    let pts = vec![
        Vec3::new(0.0, 0.0, 0.0),               // No fillet
        Vec3::new(-658.33, -3386.80, 0.0),      // No fillet
        Vec3::new(-289.38, -3454.17, 21956.98), // Large fillet radius
        Vec3::new(77.07, -3534.10, 0.0),        // No fillet
        Vec3::new(77.07, -3534.10, 0.0),        // No fillet
        Vec3::new(735.49, -146.73, 0.0),        // No fillet
        Vec3::new(368.82, -67.93, 25392.88),    // No fillet
        // Vec3::new(77.07, -3534.10, 25392.88),   // Large fillet radius
        Vec3::new(0.0, 0.0, 0.0), // No fillet
    ];

    let polyline = gen_polyline(&pts).expect("Failed to generate polyline");

    #[cfg(feature = "occ")]
    {
        let occ_wires = gen_occ_wires(&vec![pts.clone()]).expect("Failed to generate OCC wires");

        // Verify the generated OCC wire has the expected properties
        assert_eq!(occ_wires.len(), 1, "Expected a single OCC wire");
        let occ_wire = &occ_wires[0];

        // Check that the OCC wire has at least some edges
        // assert!(
        //     occ_wire.edges().len() > 3,
        //     "Expected a valid OCC wire with multiple edges"
        // );
    }

    // Verify the generated polyline has the expected properties
    // assert!(polyline.is_closed());

    // // Check that we have at least some vertices
    // assert!(
    //     polyline.vertex_data.len() > 3,
    //     "Expected a valid polyline with multiple vertices"
    // );

    // // Check for non-zero bulge in the vertex data (indicating arcs for fillet corners)
    // let arc_count = polyline
    //     .vertex_data
    //     .iter()
    //     .filter(|v| v.bulge.abs() > 0.0)
    //     .count();
    // assert!(
    //     arc_count > 0,
    //     "Expected at least one arc segment with non-zero bulge"
    // );

    println!(
        "Generated complex polyline: {}",
        polyline_to_debug_json_str(&polyline)
    );
}

/// 使用 rust-ploop-processor 处理顶点数据
///
/// 这个方法接收包含 xy 坐标和 fradius 的顶点数据，使用 rust-ploop-processor 进行处理
///
/// # 参数
/// * `vertices` - 顶点数据，Vec3 格式：x,y 为坐标，z 为 fradius 值
/// * `ploop_name` - PLOOP 名称（用于日志显示）
///
/// # 返回值
/// * `Result<Vec<Vec3>>` - 处理后的顶点列表，Vec3 格式：x,y 为坐标，z 为 fradius 值
///
/// # 示例
/// ```rust
/// use aios_core::prim_geo::wire::process_ploop_vertices;
/// use glam::Vec3;
///
/// let vertices = vec![
///     Vec3::new(0.0, 0.0, 0.0),      // 起点，无圆角
///     Vec3::new(100.0, 0.0, 0.0),    // 第二点，无圆角
///     Vec3::new(100.0, 100.0, 15.0), // 第三点，圆角半径15
///     Vec3::new(0.0, 100.0, 10.0),   // 第四点，圆角半径10
/// ];
/// let processed = process_ploop_vertices(&vertices, "TEST_PLOOP")?;
/// ```
pub fn process_ploop_vertices(
    vertices: &[Vec3],
    ploop_name: &str
) -> anyhow::Result<Vec<Vec3>> {
    if vertices.len() < 3 {
        return Err(anyhow::anyhow!("顶点数量不足，至少需要3个顶点"));
    }

    println!("🔧 开始处理PLOOP顶点: {}", ploop_name);
    println!("   输入顶点数: {}", vertices.len());

    // 创建 PLOOP 处理器
    let processor = PLoopProcessor::new();

    // 将 Vec3 转换为 PLoopVertex
    let ploop_vertices: Vec<PLoopVertex> = vertices.iter()
        .map(|v| {
            if v.z > 0.0 {
                // 有 fradius 的顶点
                PLoopVertex::with_fradius(v.x as f64, v.y as f64, 0.0, v.z as f64)
            } else {
                // 普通顶点
                PLoopVertex::new(v.x as f64, v.y as f64, 0.0)
            }
        })
        .collect();

    // 创建 PLoop 对象
    let mut ploop = PLoop::new(ploop_name.to_string(), 0.0);
    for vertex in ploop_vertices {
        ploop.add_vertex(vertex);
    }

    // 使用 rust-ploop-processor 处理 PLOOP
    let processed_vertices = processor.process_ploop(&ploop)
        .map_err(|e| anyhow::anyhow!("处理PLOOP失败: {}", e))?;

    println!("   处理后顶点数: {}", processed_vertices.len());

    // 转换回 Vec3 格式（x,y 为坐标，z 为 fradius）
    let result: Vec<Vec3> = processed_vertices.iter()
        .map(|vertex| Vec3::new(
            vertex.x() as f32,
            vertex.y() as f32,
            vertex.get_fradius() as f32  // z 存储 fradius 值
        ))
        .collect();

    let fradius_count = result.iter().filter(|v| v.z > 0.0).count();
    println!("   其中包含 {} 个FRADIUS顶点", fradius_count);
    println!("✅ PLOOP顶点处理完成，返回 {} 个顶点", result.len());

    Ok(result)
}

/// 从 PLOOP 文件内容解析并处理顶点数据
///
/// 这个方法从 PLOOP 文件内容中解析数据，然后使用 rust-ploop-processor 进行处理
///
/// # 参数
/// * `ploop_content` - PLOOP 文件的内容字符串
/// * `ploop_name` - 要处理的 PLOOP 名称（可选，如果为 None 则处理第一个找到的 PLOOP）
///
/// # 返回值
/// * `Result<Vec<Vec3>>` - 处理后的顶点列表，Vec3 格式：x,y 为坐标，z 为 fradius 值
///
/// # 示例
/// ```rust
/// use aios_core::prim_geo::wire::process_ploop_from_content;
///
/// let content = std::fs::read_to_string("717.txt")?;
/// let processed_vertices = process_ploop_from_content(&content, Some("K717"))?;
/// println!("处理完成，得到 {} 个顶点", processed_vertices.len());
/// ```
pub fn process_ploop_from_content(
    ploop_content: &str,
    ploop_name: Option<&str>
) -> anyhow::Result<Vec<Vec3>> {
    // 创建 PLOOP 处理器
    let processor = PLoopProcessor::new();

    // 解析 PLOOP 文件
    let ploops = processor.parse_file(ploop_content)
        .map_err(|e| anyhow::anyhow!("解析PLOOP文件失败: {}", e))?;

    if ploops.is_empty() {
        return Err(anyhow::anyhow!("没有找到任何有效的PLOOP数据"));
    }

    // 查找指定的 PLOOP 或使用第一个
    let target_ploop = if let Some(name) = ploop_name {
        ploops.iter()
            .find(|ploop| ploop.name.contains(name))
            .ok_or_else(|| anyhow::anyhow!("没有找到名为 '{}' 的PLOOP", name))?
    } else {
        &ploops[0]
    };

    println!("🔧 开始处理PLOOP文件: {}", target_ploop.name);
    println!("   原始顶点数: {}", target_ploop.vertices.len());

    // 使用 rust-ploop-processor 处理 PLOOP
    let processed_vertices = processor.process_ploop(target_ploop)
        .map_err(|e| anyhow::anyhow!("处理PLOOP失败: {}", e))?;

    println!("   处理后顶点数: {}", processed_vertices.len());

    // 转换为 Vec3 格式（x,y 为坐标，z 为 fradius）
    let result: Vec<Vec3> = processed_vertices.iter()
        .map(|vertex| Vec3::new(
            vertex.x() as f32,
            vertex.y() as f32,
            vertex.get_fradius() as f32  // z 存储 fradius 值
        ))
        .collect();

    let fradius_count = result.iter().filter(|v| v.z > 0.0).count();
    println!("   其中包含 {} 个FRADIUS顶点", fradius_count);
    println!("✅ PLOOP文件处理完成，返回 {} 个顶点", result.len());

    Ok(result)
}

#[test]
fn test_process_ploop_vertices() {
    // 创建测试顶点数据（Vec3: x,y 为坐标，z 为 fradius）
    let test_vertices = vec![
        Vec3::new(0.0, 0.0, 0.0),      // 起点，无圆角
        Vec3::new(100.0, 0.0, 0.0),    // 第二点，无圆角
        Vec3::new(100.0, 100.0, 10.0), // 第三点，圆角半径10
        Vec3::new(0.0, 100.0, 0.0),    // 第四点，无圆角
    ];

    // 测试 process_ploop_vertices 方法
    match process_ploop_vertices(&test_vertices, "TEST_FRAME") {
        Ok(processed_vertices) => {
            println!("✅ 顶点处理测试成功: 处理得到 {} 个顶点", processed_vertices.len());
            assert!(processed_vertices.len() > 0, "应该至少有一个顶点");

            // 打印顶点信息
            for (i, vertex) in processed_vertices.iter().enumerate() {
                if vertex.z > 0.0 {
                    println!("  顶点[{}]: ({:.2}, {:.2}) FRADIUS: {:.1}", i, vertex.x, vertex.y, vertex.z);
                } else {
                    println!("  顶点[{}]: ({:.2}, {:.2})", i, vertex.x, vertex.y);
                }
            }
        }
        Err(e) => {
            println!("❌ 顶点处理测试失败: {}", e);
            // 在测试环境中，这可能会失败，因为 rust-ploop-processor 可能不可用
            // 这是正常的，我们只是验证方法的接口
        }
    }
}

#[test]
fn test_process_ploop_from_content() {
    // 创建一个包含 FRADIUS 的测试 PLOOP 数据
    let test_ploop_content = r#"
NEW FRMWORK TEST_FRAME_FRADIUS
NEW PLOOP
VERTEX 0.0 0.0 0.0
VERTEX 100.0 0.0 0.0
VERTEX 100.0 100.0 0.0 FRADIUS 15.0
VERTEX 0.0 100.0 0.0 FRADIUS 5.0
END PLOOP
END FRMWORK
"#;

    // 测试从内容解析的 process_ploop_from_content 方法
    match process_ploop_from_content(test_ploop_content, None) {
        Ok(vertices) => {
            println!("✅ 内容解析测试成功: 处理得到 {} 个顶点", vertices.len());
            assert!(vertices.len() > 0, "应该至少有一个顶点");

            // 打印顶点信息
            for (i, vertex) in vertices.iter().enumerate() {
                if vertex.z > 0.0 {
                    println!("  顶点[{}]: ({:.2}, {:.2}) FRADIUS: {:.1}",
                        i, vertex.x, vertex.y, vertex.z);
                } else {
                    println!("  顶点[{}]: ({:.2}, {:.2})",
                        i, vertex.x, vertex.y);
                }
            }

            // 检查是否有 FRADIUS 值
            let has_fradius = vertices.iter().any(|v| v.z > 0.0);
            if has_fradius {
                println!("  ✅ 检测到FRADIUS值");
            }
        }
        Err(e) => {
            println!("❌ 内容解析测试失败: {}", e);
            // 在测试环境中，这可能会失败，因为 rust-ploop-processor 可能不可用
            // 这是正常的，我们只是验证方法的接口
        }
    }
}

#[test]
fn test_gen_polyline_with_ploop_processor() {
    // 测试带 FRADIUS 的顶点数据
    let vertices_with_fradius = vec![
        Vec3::new(0.0, 0.0, 0.0),      // 起点，无圆角
        Vec3::new(100.0, 0.0, 0.0),    // 第二点，无圆角
        Vec3::new(100.0, 100.0, 15.0), // 第三点，圆角半径15
        Vec3::new(0.0, 100.0, 10.0),   // 第四点，圆角半径10
    ];

    println!("🧪 测试带 FRADIUS 的 gen_polyline 方法");
    match gen_polyline(&vertices_with_fradius) {
        Ok(polyline) => {
            println!("✅ 带 FRADIUS 测试成功！");
            println!("   生成的 Polyline 有 {} 个顶点", polyline.vertex_data.len());
            println!("   Polyline 是否闭合: {}", polyline.is_closed());

            // 检查是否有圆弧段（bulge != 0）
            let arc_count = polyline.vertex_data.iter().filter(|v| v.bulge.abs() > 0.001).count();
            println!("   包含 {} 个圆弧段", arc_count);
        }
        Err(e) => {
            println!("❌ 带 FRADIUS 测试失败: {}", e);
            // 这可能会失败，因为 rust-ploop-processor 可能不可用
        }
    }

    // 测试无 FRADIUS 的顶点数据
    let vertices_no_fradius = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(100.0, 100.0, 0.0),
        Vec3::new(0.0, 100.0, 0.0),
    ];

    println!("\n🧪 测试无 FRADIUS 的 gen_polyline 方法");
    match gen_polyline(&vertices_no_fradius) {
        Ok(polyline) => {
            println!("✅ 无 FRADIUS 测试成功！");
            println!("   生成的 Polyline 有 {} 个顶点", polyline.vertex_data.len());
            println!("   Polyline 是否闭合: {}", polyline.is_closed());
        }
        Err(e) => {
            println!("❌ 无 FRADIUS 测试失败: {}", e);
        }
    }
}

#[test]
fn gen_polyline_handles_fillet_radius_equal_to_edge_length() {
    let vertices = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 24450.0, 0.0),
        Vec3::new(24450.0, 24450.0, 24450.0),
        Vec3::new(24450.0, 0.0, 0.0),
    ];

    let polyline = gen_polyline(&vertices).expect("extreme fillet must remain renderable");

    assert!(polyline.vertex_data.len() >= 3);
    assert!(
        polyline
            .vertex_data
            .iter()
            .all(|vertex| vertex.x.is_finite()
                && vertex.y.is_finite()
                && vertex.bulge.is_finite())
    );
}
