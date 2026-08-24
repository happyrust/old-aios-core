use derive_more::{Deref, DerefMut};
use glam::DMat4;
use lazy_static::lazy_static;
#[cfg(feature = "occ")]
use opencascade::primitives::*;
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(feature = "occ")]
lazy_static! {
    pub static ref BOX_SHAPE: OccSharedShape =
        OccSharedShape::new(Shape::box_centered(1.0, 1.0, 1.0));
    pub static ref SPHERE_SHAPE: OccSharedShape = OccSharedShape::new(Shape::sphere(0.5).build());
    pub static ref CYLINDER_SHAPE: OccSharedShape =
        OccSharedShape::new(Shape::cylinder_radius_height(0.5, 1.0));
    pub static ref BASIC_PRIM_SHAPE_MAP: HashMap<u64, OccSharedShape> = {
        let mut s = HashMap::new();
        s.insert(BOXI_GEO_HASH, BOX_SHAPE.clone());
        s.insert(CYLINDER_GEO_HASH, CYLINDER_SHAPE.clone());
        s.insert(SPHERE_GEO_HASH, SPHERE_SHAPE.clone());

        s
    };
}

#[cfg(feature = "occ")]
#[derive(Clone, Deref, DerefMut)]
pub struct OccSharedShape(pub Arc<Shape>);

#[cfg(feature = "occ")]
impl OccSharedShape {
    #[inline]
    pub fn new(shape: Shape) -> Self {
        OccSharedShape(Arc::new(shape))
    }

    #[inline]
    pub fn transformed(&self, m: &DMat4) -> anyhow::Result<Self> {
        let s = self.0.transformed_by_gmat(m)?;
        Ok(OccSharedShape::new(s))
    }
}

#[cfg(feature = "occ")]
impl AsRef<Shape> for OccSharedShape {
    fn as_ref(&self) -> &Shape {
        &self.0
    }
}

#[cfg(feature = "occ")]
impl AsMut<Shape> for OccSharedShape {
    fn as_mut(&mut self) -> &mut Shape {
        Arc::get_mut(&mut self.0).unwrap()
    }
}

#[cfg(feature = "occ")]
impl From<Shape> for OccSharedShape {
    fn from(s: Shape) -> Self {
        OccSharedShape::new(s)
    }
}

pub const BOX_GEO_HASH: u64 = 1u64;
pub const CYLINDER_GEO_HASH: u64 = 2u64;
pub const TUBI_GEO_HASH: u64 = 2u64;
pub const BOXI_GEO_HASH: u64 = 1u64;
pub const SPHERE_GEO_HASH: u64 = 3u64;
