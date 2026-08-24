use glam::DVec3;
use glam::DVec4;
use glam::{Vec2, Vec3};
use ordered_float::OrderedFloat;
use std::hash::{Hash, Hasher};

pub fn cal_vec3_hash(v: Vec3) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hash_vec3(&v, &mut hasher);
    hasher.finish()
}

pub fn cal_vec2_hash(v: Vec2) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hash_f32(v[0], &mut hasher);
    hash_f32(v[1], &mut hasher);
    hasher.finish()
}

//保留三位有效数字的拼接
pub fn cal_vec2_hash_string(v: Vec2) -> String {
    format!("{:.3},{:.3}", v[0], v[1])
}

//保留三位有效数字的拼接
pub fn cal_xy_hash_string(x: f32, y: f32) -> String {
    format!("{:.3},{:.3}", x, y)
}

pub fn cal_xy_hash(x: f32, y: f32) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hash_f32(x, &mut hasher);
    hash_f32(y, &mut hasher);
    hasher.finish()
}

// pub fn cal_dxy_hash(x: f32, y: f32) -> u64 {
//     let mut hasher = std::collections::hash_map::DefaultHasher::new();
//     hash_f32(x, &mut hasher);
//     hash_f32(y, &mut hasher);
//     hasher.finish()
// }

#[inline]
pub fn hash_vec3<T: Hasher>(v: &Vec3, hasher: &mut T) {
    hash_f32(v[0], hasher);
    hash_f32(v[1], hasher);
    hash_f32(v[2], hasher);
}

#[inline]
pub fn hash_f64_slice<T: Hasher>(a: &[f64], hasher: &mut T) {
    for v in a {
        hash_f64(*v, hasher);
    }
}

#[inline]
pub fn hash_f32_slice<T: Hasher>(a: &[f32], hasher: &mut T) {
    for v in a {
        hash_f32(*v, hasher);
    }
}

//三位有效数字的精度
#[inline]
pub fn hash_f32<T: Hasher>(v: f32, hasher: &mut T) {
    OrderedFloat(f32_round_3(v)).hash(hasher);
}

#[inline]
pub fn hash_f64<T: Hasher>(v: f64, hasher: &mut T) {
    OrderedFloat(f64_round_3(v)).hash(hasher);
}

#[inline]
pub fn vec3_round_3(v: Vec3) -> Vec3 {
    Vec3::new(f32_round_3(v.x), f32_round_3(v.y), f32_round_3(v.z))
}

#[inline]
pub fn vec3_round_2(v: Vec3) -> Vec3 {
    Vec3::new(f32_round_2(v.x), f32_round_2(v.y), f32_round_2(v.z))
}

#[inline]
pub fn vec3_round_1(v: Vec3) -> Vec3 {
    Vec3::new(f32_round_1(v.x), f32_round_1(v.y), f32_round_1(v.z))
}

#[inline]
pub fn f32_round_3(v: f32) -> f32 {
    ((v as f64 * 1000.0).round() / 1000.0f64) as f32 //以防止溢出
}

#[inline]
pub fn f32_round_2(v: f32) -> f32 {
    ((v as f64 * 100.0).round() / 100.0f64) as f32 //以防止溢出
}

#[inline]
pub fn f32_round_1(v: f32) -> f32 {
    ((v as f64 * 10.0).round() / 10.0f64) as f32 //以防止溢出
}

#[inline]
pub fn f64_round_3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0f64 //以防止溢出
}

#[inline]
pub fn f64_round_4(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0f64 //以防止溢出
}

#[inline]
pub fn f64_round(v: f64) -> f64 {
    // (v * 10000.0).round() / 10000.0f64 //以防止溢出
    v
}

#[inline]
pub fn f64_round_2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0f64 //以防止溢出
}

#[inline]
pub fn f64_ceil_2(v: f64) -> f64 {
    (v * 100.0).ceil() / 100.0f64 //以防止溢出
}

#[inline]
pub fn f64_ceil_3(v: f64) -> f64 {
    (v * 1000.0).ceil() / 1000.0f64 //以防止溢出
}

#[inline]
pub fn f64_trunc_3(v: f64) -> f64 {
    ((v * 1000.0) as i64) as f64 / 1000.0f64 //以防止溢出
}

#[inline]
pub fn f64_trunc_4(v: f64) -> f64 {
    ((v * 10000.0) as i64) as f64 / 10000.0f64
}

#[inline]
pub fn f64_round_1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0f64 //以防止溢出
}

pub fn f64_trunc_1(v: f64) -> f64 {
    (v * 10.0).trunc() / 10.0f64 //以防止溢出
}

pub fn f64_trunc_2(v: f64) -> f64 {
    (v * 100.0).trunc() / 100.0f64 //以防止溢出
}

#[inline]
pub fn dvec3_round_3(v: DVec3) -> DVec3 {
    DVec3::new(f64_round_3(v.x), f64_round_3(v.y), f64_round_3(v.z))
}

#[inline]
pub fn dvec4_round_3(v: DVec4) -> DVec4 {
    DVec4::new(
        f64_round_3(v.x),
        f64_round_3(v.y),
        f64_round_3(v.z),
        f64_round_3(v.w),
    )
}
