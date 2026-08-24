use crate::data_center::RawHoleData;
use crate::get_default_pdms_db_info;
use crate::schema::generate_basic_versioned_schema;

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
