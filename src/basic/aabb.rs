use bevy_ecs::prelude::Component;
use derive_more::{Deref, DerefMut};
use parry3d::bounding_volume::Aabb;

#[derive(Debug, PartialEq, Copy, Clone, Component, Deref, DerefMut)]
pub struct ParryAabb(pub Aabb);

impl Default for ParryAabb {
    fn default() -> Self {
        Self(Aabb::new_invalid())
    }
}

impl ParryAabb {
    pub fn is_valid(&self) -> bool {
        let m = self.0.extents().magnitude();
        m > 1e-4 && m < f32::INFINITY
    }
}
