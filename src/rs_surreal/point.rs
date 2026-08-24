use crate::basic::aabb::ParryAabb;
use crate::parsed_data::CateAxisParam;
use crate::pdms_types::PdmsGenericType;
use crate::{RefU64, RefnoEnum, SUL_DB};
use bevy_transform::components::Transform;
use dashmap::DashMap;
use glam::Vec3;
use parry3d::bounding_volume::Aabb;
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct TubiInstQuery {
    #[serde(alias = "id")]
    pub refno: RefU64,
    pub world_aabb: Aabb,
    pub world_trans: Transform,
    pub geo_hash: String,
}

///查询离开点和到达点, local 为true时，表示使用局部坐标
pub async fn query_arrive_leave_points(
    refnos: impl IntoIterator<Item = &RefU64>,
    local: bool,
) -> anyhow::Result<DashMap<RefU64, [CateAxisParam; 2]>> {
    let pes: String = refnos
        .into_iter()
        .map(|x| x.to_pe_key())
        .collect::<Vec<_>>()
        .join(",");
    if pes.is_empty() {
        return Ok(DashMap::new());
    }
    dbg!(&pes);
    //[? owner.noun in ['BRAN', 'HANG']]
    let sql = format!(
        r#"
             select value [
                    in,
                    world_trans.d,
                    (select * from object::values(out.ptset) where number=$parent.in.refno.ARRI)[0],
                    (select * from object::values(out.ptset) where number=$parent.in.refno.LEAV)[0]
                ]
              from array::flatten([{}][? owner.noun in ['BRAN', 'HANG']]->inst_relate) where world_trans.d!=none
             "#,
        pes
    );
    // dbg!(&sql);
    let mut response = SUL_DB.query(&sql).await.unwrap();

    // dbg!(&response);
    let result: Vec<(
        RefU64,
        Transform,
        Option<CateAxisParam>,
        Option<CateAxisParam>,
    )> = response.take(0).unwrap();
    // dbg!(&r);
    let mut map = DashMap::new();
    for (refno, trans, a_pt, l_pt) in result {
        if a_pt.is_none() || l_pt.is_none() {
            continue;
        }
        let mut pts = [a_pt.unwrap(), l_pt.unwrap()];
        if !local {
            pts[0].transform(&trans);
            pts[1].transform(&trans);
        }
        map.insert(refno, pts);
    }
    Ok(map)
}

pub async fn query_arrive_leave_points_by_cata_hash(
    refnos: impl IntoIterator<Item = &RefnoEnum>,
) -> anyhow::Result<DashMap<RefnoEnum, [CateAxisParam; 2]>> {
    let pes: String = refnos
        .into_iter()
        .map(|x| x.to_pe_key())
        .collect::<Vec<_>>()
        .join(",");
    if pes.is_empty() {
        return Ok(DashMap::new());
    }
    //[? owner.noun in ['BRAN', 'HANG']]
    let sql = format!(
        r#"
             select value [id,
                (select * from object::values(type::thing("inst_info", cata_hash).ptset) where number=$parent.refno.ARRI)[0],
                (select * from object::values(type::thing("inst_info", cata_hash).ptset) where number=$parent.refno.LEAV)[0]
            ] from array::flatten([{}][? owner.noun in ['BRAN', 'HANG']])
             "#,
        pes
    );
    // dbg!(&sql);
    let mut response = SUL_DB.query(&sql).await?;

    // dbg!(&response);
    let result: Vec<(RefnoEnum, Option<CateAxisParam>, Option<CateAxisParam>)> =
        response.take(0)?;
    // dbg!(&r);
    let mut map = DashMap::new();
    for (refno, a_pt, l_pt) in result {
        if a_pt.is_none() || l_pt.is_none() {
            continue;
        }
        let mut pts = [a_pt.unwrap(), l_pt.unwrap()];
        map.insert(refno, pts);
    }
    Ok(map)
}
