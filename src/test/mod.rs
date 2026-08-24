pub mod test_attmap;
pub mod test_shape;

pub mod test_schema;

pub mod test_refno;

pub mod test_rotation;

pub mod test_hash;

pub mod test_serde;

pub mod test_spatial_calculation;

pub mod test_parse_dir;

#[cfg(not(target_arch = "wasm32"))]
pub mod test_surreal;

pub mod test_expression;

pub mod test_different_dbs;

// pub mod test_transform;

// pub mod test_material;
