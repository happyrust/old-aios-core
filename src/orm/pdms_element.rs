use crate::impl_db_op_trait;
use crate::orm::traits::{DbOpTrait, ReflectDbOpTrait};
use crate::types::*;
use bevy_reflect::{
    DynamicStruct, Reflect, ReflectFromReflect, Struct, TypeRegistry, Typed,
    std_traits::ReflectDefault,
};
use sea_orm::{DatabaseBackend, QueryTrait, Schema, entity::prelude::*};
use serde::{Deserialize, Serialize};
use serde_with::DisplayFromStr;
use serde_with::serde_as;
use std::any::TypeId;
use surrealdb::sql::Thing;

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug, Default, DeriveEntityModel)]
#[sea_orm(table_name = "PdmsElement")]
#[derive(Reflect)]
#[reflect(Default, DbOpTrait)]
pub struct Model {
    //todo 用来作为sql的主键
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[serde_as(as = "DisplayFromStr")]
    pub refno: RefU64,
    #[serde_as(as = "DisplayFromStr")]
    pub owner: RefU64,
    pub name: String,
    pub noun: String,
    pub dbnum: i32,
    pub sesno: i32,
    ///大版本号
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_tag: Option<String>,
    ///小版本号
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_tag: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cata_hash: Option<String>,
    ///锁定模型
    pub lock: bool,
}

impl_db_op_trait!();

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    #[inline]
    pub fn get_type_str(&self) -> &str {
        return self.noun.as_str();
    }
    #[inline]
    pub fn get_owner(&self) -> RefU64 {
        return self.owner;
    }
}

#[test]
fn test_ele_reflect() {
    let mut data = Model::default();
    data.name = "PdmsElement".to_owned();
    for (i, v) in data.iter_fields().enumerate() {
        let field_name = data.name_at(i).unwrap();
        if let Some(value) = v.downcast_ref::<i32>() {
            println!("{} is a u32 with the value: {}", field_name, *value);
        }
    }

    let mut dynamic_struct = DynamicStruct::default();
    let type_info = <Model as Typed>::type_info();
    dynamic_struct.set_represented_type(Some(type_info));
    dynamic_struct.insert("name", "Test".to_string());

    let mut type_registry: TypeRegistry = TypeRegistry::default();
    type_registry.register::<Model>();

    // Get type data
    let type_id = TypeId::of::<Model>();
    let rfr = type_registry
        .get_type_data::<ReflectFromReflect>(type_id)
        .expect("the FromReflect trait should be registered");

    //  // Call from_reflect
    let mut dynamic_struct = DynamicStruct::default();
    dynamic_struct.insert("name", "test".to_string());
    let reflected = rfr
        .from_reflect(&dynamic_struct)
        .expect("the type should be properly reflected");

    let reflect_do_thing = type_registry
        .get_type_data::<ReflectDbOpTrait>(TypeId::of::<Model>())
        .unwrap();
    let entity_trait: &dyn DbOpTrait = reflect_do_thing.get(&*reflected).unwrap();

    // // Which means we can now call do_thing(). Magic!
    println!(
        "{}",
        entity_trait.gen_insert_many(vec![dynamic_struct], DatabaseBackend::MySql)
    );
    let create_sql = entity_trait.gen_create_table(DatabaseBackend::MySql);
    dbg!(&create_sql);
}
