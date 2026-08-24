use crate::BHashMap;
use crate::types::named_attmap::NamedAttrMap;
use crate::types::named_attvalue::NamedAttrValue;

#[test]
fn test_serde_attr_value() {
    let test_att = NamedAttrValue::StringArrayType(vec!["123".to_string(), "456".to_string()]);
    let mut map = BHashMap::new();
    map.insert("NAMES".to_string(), test_att);
    let test_attmap = NamedAttrMap { map };
    let json = serde_json::to_string(&test_attmap).unwrap();
    dbg!(&json);
}
