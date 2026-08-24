use crate::parsed_data::CateAxisParam;
use crate::types::*;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::BTreeMap;
//
//

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct InstPointMap {
    pub refno: RefnoEnum,
    pub att_type: String,
    #[serde(default)]
    pub ptset_map: BTreeMap<String, CateAxisParam>,
}

#[test]
fn test_de_ser() {
    let input = r#"{
    "att_type":"WELD",
    "ptset_map": None,
    "refno":"pe:17414_27235"
}"#;
    let r = serde_json::from_str::<InstPointMap>(input).unwrap();
    dbg!(&r);
}
// #[serde_as]
// #[derive(Serialize, Deserialize, Clone, Debug, Default)]
// pub struct InstPointVec {
//     pub id: RefnoEnum,
//     pub att_type: String,
//     #[serde(default)]
//     pub ptset_map: Vec<CateAxisParam>,
// }
//
// impl InstPointVec {
//     pub fn into_point_map(self) -> InstPointMap {
//         let mut map = BTreeMap::new();
//         for p in self.ptset_map {
//             map.entry(p.number).or_insert(p);
//         }
//         InstPointMap {
//             refno: self.id,
//             att_type: self.att_type,
//             ptset_map: map,
//         }
//     }
// }
