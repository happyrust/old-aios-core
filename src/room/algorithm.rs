use crate::pdms_types::RoomNodes;
use crate::{RefU64, RefnoEnum, SUL_DB};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use serde_with::DisplayFromStr;
use serde_with::serde_as;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::str::FromStr;
use tokio::sync::RwLock;

#[derive(Serialize, Deserialize, Default, Debug, Clone, Hash, Eq, PartialEq)]
pub struct RoomInfo {
    pub name: String,
    pub refno: RefnoEnum,
}

impl Ord for RoomInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for RoomInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// 查询项目的所有房间
///
/// 返回值: k 厂房 v 房间编号
pub async fn query_all_room_name() -> anyhow::Result<HashMap<String, BTreeSet<RoomInfo>>> {
    let mut map = HashMap::new();
    let mut response = SUL_DB
        .query(r#"
           let $f = array::flatten(select value array::flatten(array::flatten(<-pe_owner.in<-pe_owner.in<-pe_owner[where in.noun == 'FRMW'].in.refno.id) )
            from (select value REFNO from SITE where NAME != NONE && string::contains(NAME,'ARCH')));

            return select id as refno ,NAME as name from array::flatten($f) where NAME != NONE;
        "#)
        .await?;
    let results: Vec<RoomInfo> = response.take(1)?;
    for r in results.clone() {
        let room = r.name;
        let split = room.split("-").collect::<Vec<_>>();
        let Some(first) = split.first() else {
            continue;
        };
        let Some(last) = split.last() else {
            continue;
        };
        // if !match_room_name_hd(last) {
        //     continue;
        // };
        map.entry(first[1..].to_string())
            .or_insert_with(BTreeSet::new)
            .insert(RoomInfo {
                name: last.to_string(),
                refno: r.refno,
            });
    }
    Ok(map)
}

/// 查询多个refno所属的房间号，bran和equi也适用
pub async fn query_room_name_from_refnos(
    owner: Vec<RefnoEnum>,
) -> anyhow::Result<HashMap<RefU64, String>> {
    #[derive(Debug, Serialize, Deserialize)]
    struct RoomNameQueryRequest {
        pub id: RefU64,
        pub room: Option<String>,
    }

    let owners = owner
        .into_iter()
        .map(|o| o.to_pe_key())
        .collect::<Vec<String>>()
        .join(",");
    let sql = format!("select id,fn::room_code(id)[0] as room from [{}]", owners);
    let mut response = SUL_DB.query(sql).await?;
    let result: Vec<RoomNameQueryRequest> = response.take(0)?;
    let r = result
        .into_iter()
        .map(|x| (x.id, x.room.unwrap_or("".to_string())))
        .collect::<HashMap<RefU64, String>>();
    Ok(r)
}

/// 查找设备和阀门所属的楼板
pub async fn query_equi_or_valv_belong_floors(
    refnos: Vec<RefnoEnum>,
) -> anyhow::Result<HashMap<RefU64, (String, f32)>> {
    #[serde_as]
    #[derive(Serialize, Deserialize, Debug)]
    struct BelongFloorResponse {
        #[serde_as(as = "DisplayFromStr")]
        pub id: RefU64,
        pub floor: Option<String>,
        pub height: Option<f32>,
    }

    let refnos = refnos
        .into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>();
    let request = serde_json::to_string(&refnos)?;
    let sql = format!(
        "select id,(->nearest_relate.out.REFNO)[0] as floor,(->nearest_relate.dist)[0] as height from {}",
        request
    );
    let mut response = SUL_DB.query(sql).await?;
    let result: Vec<BelongFloorResponse> = response.take(0)?;
    let r = result
        .into_iter()
        .map(|x| {
            (
                x.id,
                (
                    x.floor.map_or("".to_string(), |x| {
                        RefU64::from_str(&x).unwrap().to_pdms_str()
                    }),
                    x.height.unwrap_or(0.0),
                ),
            )
        })
        .collect::<HashMap<RefU64, (String, f32)>>();
    Ok(r)
}

#[tokio::test]
async fn test_query_all_room_name() {
    // init_test_surreal().await;
    // let r = query_all_room_name().await.unwrap();
    // dbg!(&r);
}
