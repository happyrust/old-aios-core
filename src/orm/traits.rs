use bevy_reflect::{DynamicStruct, reflect_trait};
use sea_orm::{DatabaseBackend, DatabaseConnection};

#[reflect_trait]
pub trait DbOpTrait {
    fn gen_insert_many(&self, models: Vec<DynamicStruct>, backend: DatabaseBackend) -> String;

    fn gen_create_table(&self, backend: DatabaseBackend) -> String;
}
