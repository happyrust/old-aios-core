use bevy_ecs::component::Component;
use bevy_reflect::Reflect;
use bevy_transform::components::Transform;
use serde_derive::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Default,
    Component,
    Reflect,
    // Deref,
    // DerefMut,
)]
pub struct RsTransform(pub Transform);

impl Debug for RsTransform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("RsTransform").field(&self.0).finish()
    }
}
