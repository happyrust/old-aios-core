use derive_more::{Deref, DerefMut};
use glam::DMat4;
use lazy_static::lazy_static;
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::sync::Arc;

pub const BOX_GEO_HASH: u64 = 1u64;
/// 隐含直管段（TUBI）全项目共用的那一行单位圆柱 `inst_geo:⟨2⟩`。
///
/// 它是**唯一**仍按固定 id 寻址的曲面单位行：管段的口径在实例变换里，共用一行就
/// 带不了随口径变的段数。普通圆柱 / 球从 2026-09 起不再有固定 id——
/// `LCylinder` / `SCylinder` / `Sphere` 的 `hash_unit_mesh_params()` 按真实半径算出的
/// 段数分行（gen-model specs/009 T041），原来的 `CYLINDER_GEO_HASH` / `SPHERE_GEO_HASH`
/// 随之删除；`resource/surreal/gy_common.surql` 一类按 `inst_geo:⟨2⟩` 汇总管长的
/// 查询，依赖的是这一个常量。
pub const TUBI_GEO_HASH: u64 = 2u64;
pub const BOXI_GEO_HASH: u64 = 1u64;
