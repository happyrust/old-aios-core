pub mod ctorus;
pub mod cylinder;
pub mod dish;
pub mod extrusion;
pub mod facet;
pub mod helper;
pub mod lpyramid;
pub mod polyhedron;
pub mod pyramid;
pub mod revolution;
pub mod rtorus;
pub mod sbox;
pub mod snout;
pub mod sphere;
pub mod sweep_solid;
pub mod tubing;

pub mod category;
pub mod spine;

pub mod wire;

pub mod line;

pub mod basic;

pub mod profile;

use crate::prim_geo::category::CateBrepShape;
use crate::{RefU64, RefnoEnum};
pub use ctorus::*;
pub use cylinder::*;
use dashmap::DashMap;
pub use dish::*;
pub use extrusion::*;
pub use facet::*;
pub use lpyramid::*;
pub use polyhedron::*;
pub use pyramid::*;
pub use revolution::*;
pub use rtorus::*;
pub use sbox::*;
pub use snout::*;
pub use sphere::*;
pub use sweep_solid::*;
pub use tubing::*;

pub type CateBrepShapeMap = DashMap<RefnoEnum, Vec<CateBrepShape>>;
