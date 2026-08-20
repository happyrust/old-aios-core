use crate::data_center::RawHoleData;
use crate::get_default_pdms_db_info;
use crate::schema::generate_basic_versioned_schema;
use crate::tool::db_tool::db1_hash;
use crate::AttrVal;

#[test]
fn db_project_id_schema_is_an_integer_array() {
    let db_info = get_default_pdms_db_info();
    let db_attrs = db_info
        .noun_attr_info_map
        .get(&(db1_hash("DB") as i32))
        .expect("DB noun must exist in the schema snapshot");
    let project = db_attrs
        .get(&739708)
        .expect("DB PROJ/PROJID attribute must exist");

    assert!(matches!(
        project.default_val,
        AttrVal::IntArrayType(ref value) if value.is_empty()
    ));
}

#[test]
fn test_gen_att_schema() {
    // let db_info = get_default_pdms_db_info();
    // let schema = db_info.get_all_schemas();
    // let v = schema.into_iter().next().unwrap();
    // // let pretty_json = jsonxf::minimize(&v).unwrap();
    // dbg!(serde_json::to_string(&v));
}

#[test]
fn test_gen_schema_from_json() {
    // let test_data = VirtualHoleGraphNodeQuery::default();
    // let schema = VirtualHoleGraphNodeQuery::get_scheme();
    // dbg!(&schema);
    // let pretty_json = jsonxf::minimize(&v).unwrap();
    // dbg!(serde_json::to_string(&v));
}
