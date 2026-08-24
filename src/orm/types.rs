use bevy_reflect::Reflect;
use bevy_reflect::prelude::ReflectDefault;
use sea_orm::FromJsonQueryResult;
use serde::{Deserialize, Serialize};

#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Default, FromJsonQueryResult, Reflect,
)]
#[reflect(Default)]
pub struct StringVec(pub Vec<String>);

#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Default, FromJsonQueryResult, Reflect,
)]
#[reflect(Default)]
pub struct F32Vec(pub Vec<f32>);

#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Default, FromJsonQueryResult, Reflect,
)]
#[reflect(Default)]
pub struct I32Vec(pub Vec<i32>);

#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Default, FromJsonQueryResult, Reflect,
)]
#[reflect(Default)]
pub struct BoolVec(pub Vec<bool>);
