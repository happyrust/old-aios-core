use derive_more::{Deref, DerefMut};
use glam::DMat4;
use lazy_static::lazy_static;
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::sync::Arc;

pub const BOX_GEO_HASH: u64 = 1u64;
pub const CYLINDER_GEO_HASH: u64 = 2u64;
pub const TUBI_GEO_HASH: u64 = 2u64;
pub const BOXI_GEO_HASH: u64 = 1u64;
pub const SPHERE_GEO_HASH: u64 = 3u64;
