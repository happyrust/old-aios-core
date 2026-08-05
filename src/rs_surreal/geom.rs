use crate::pdms_pluggin::heat_dissipation::InstPointMap;
use crate::pe::SPdmsElement;
use crate::{init_test_surreal, query_filter_deep_children, types::*};
use crate::{pdms_types::*, to_table_key, to_table_keys};
use crate::{NamedAttrMap, RefnoEnum};
use crate::SurlValue;
use bevy_transform::components::Transform;
use cached::proc_macro::cached;
use glam::Vec3;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use smol_str::ToSmolStr;
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use std::sync::Mutex;

//获得参考号对应的inst keys
pub fn get_inst_relate_keys(refnos: &[RefnoEnum]) -> String {
    if !refnos.is_empty() {
        refnos
            .iter()
            .map(|x| x.to_inst_relate_key())
            .collect::<Vec<_>>()
            .join(",")
        // format!("array::flatten([{pes}]->inst_relate)")
    } else {
        "inst_relate".to_string()
    }
}

///获得当前参考号对应的loops （例如Panel下的loops，可能有多个）
pub async fn fetch_loops_and_height(refno: RefnoEnum) -> anyhow::Result<(Vec<Vec<Vec3>>, f32)> {
    let sql = format!(
        r#"
        select value (select value [
            in.refno.POS[0] ?? type::thing("PAVE", record::id(in)).POS[0] ?? type::thing("VERT", record::id(in)).POS[0],
            in.refno.POS[1] ?? type::thing("PAVE", record::id(in)).POS[1] ?? type::thing("VERT", record::id(in)).POS[1],
            in.refno.FRAD ?? type::thing("PAVE", record::id(in)).FRAD ?? type::thing("VERT", record::id(in)).FRAD
        ] from <-pe_owner) from
            (select value in from {0}<-pe_owner where in.noun in ["LOOP", "PLOO"]);
        array::complement((select value refno.HEIG from [ (select value in.id from only {0}<-pe_owner where in.noun in ["LOOP", "PLOO"] limit 1), {0}]), [none])[0];
        "#,
        refno.to_pe_key()
    );
    // println!(" fetch_loops_and_height sql is {}", &sql);
    let mut response = super::staging::data_db().query(&sql).await.unwrap();
    let points: Vec<Vec<Vec3>> = response.take(0)?;
    let height: Option<f32> = response.take(1)?;

    Ok((points, height.unwrap_or_default()))
}

///通过surql查询pe数据
///
/// 刻意**不加** `#[cached]`：本函数产出的是重生成路径的「刷新集」——网格生成、
/// 包围盒刷新、房间触发全按它圈定范围。它的值随分支成员增删而变，而增量管线的
/// `clear_all_caches_batch` 只按「变更元素 + 其属主」失效，从不知道这里还有一份
/// 按**生成根**为键的快照；缓存命中一次陈旧值，新加的构件就整体缺席本轮刷新
/// （mesh 不生成、aabb 不落库、房间不触发），且无任何报错。真正贵的子树遍历在
/// `query_deep_children_refnos` 里另有缓存（并被失效机制覆盖），这里每根多跑的
/// 几条查询相对整根重生成（秒级）是噪音。
pub async fn query_deep_visible_inst_refnos(refno: RefnoEnum) -> anyhow::Result<Vec<RefnoEnum>> {
    let types = super::get_self_and_owner_type_name(refno).await?;
    if types[1] == "BRAN" || types[1] == "HANG" {
        return Ok(vec![refno]);
    }
    if types[0] == "BRAN" || types[0] == "HANG" {
        // 分支自身也要算：隐含直管段（TUBI/BOXI）的 inst_relate 行挂在 BRAN/HANG
        // 名下（cata_model 的 insert_tubi 以分支为键）。漏掉它，重生成后的包围盒
        // 刷新与房间归属就永远轮不到管段。
        let mut children_refnos = super::get_children_refnos(refno).await?;
        children_refnos.push(refno);
        return Ok(children_refnos);
    }
    //TODO，这里可以采用ZONE作为中间层去加速这个过程
    //按照所允许的层级关系去遍历？
    let branch_refnos = super::query_filter_deep_children(refno, &["BRAN", "HANG"]).await?;

    let mut target_refnos = super::query_multi_children_refnos(&branch_refnos).await?;
    // 同上：分支自身承载隐含直管段的几何行。
    target_refnos.extend(branch_refnos.iter().copied());

    let visible_refnos = super::query_filter_deep_children(refno, &VISBILE_GEO_NOUNS).await?;
    target_refnos.extend(visible_refnos);
    Ok(target_refnos)
}

/// 与 [`query_deep_visible_inst_refnos`] 同理不加 `#[cached]`：负实体集合同样随
/// 成员增删而变，键是生成根、失效机制够不着。
pub async fn query_deep_neg_inst_refnos(refno: RefnoEnum) -> anyhow::Result<Vec<RefnoEnum>> {
    let neg_refnos = super::query_filter_deep_children(refno, &TOTAL_NEG_NOUN_NAMES).await?;
    Ok(neg_refnos)
}

//leave_or_arrive: true: leave, false: arrive
#[cached(result = true)]
pub async fn query_la_axis_attmap(
    refno: RefnoEnum,
    leave_or_arrive: bool,
) -> anyhow::Result<NamedAttrMap> {
    // let cata_refno = super::get_cat_refno(refno).await?.ok_or(anyhow::anyhow!("no cat_refno"))?;
    // dbg!(&cata_refno);
    // let axis_map = super::query_single_by_paths(
    //     cata_refno,
    //     &["->PTRE", "->PTSE"],
    //     &["refno"],
    // )
    // .await?;
    Ok(Default::default())
}

/// 参考号具有正负实体映射关系的信息结构体
#[derive(Serialize, Deserialize, Debug)]
pub struct RefnoHasNegPosInfo {
    // pub refno: RefnoEnum,
    /// 正实体的参考号
    pub pos: RefnoEnum,
    /// 负实体的参考号集合
    pub negs: Vec<RefnoEnum>,
}

/// 后面再创建一个 compound 的关系，负责连接这个参考号对应的 info，并标记为 compound， compound 优先
/// 是
///返回有负实体和正实体的参考号集合，还有对应的NOUN
///还要考虑下面有多个LOOP或者PLOO的情况，第二个开始都是负实体
/// 首先查询到所有的负实体，然后找到sibling和父节点
pub async fn query_refno_has_pos_neg_map(
    refno: RefnoEnum,
    is_cata: Option<bool>, //是否是元件库里的负实体查询
) -> anyhow::Result<HashMap<RefnoEnum, Vec<RefnoEnum>>> {
    //先查询负实体和它的neg children
    let nouns = match is_cata {
        Some(true) => CATE_NEG_NOUN_NAMES.as_slice(),
        Some(false) => &GENRAL_NEG_NOUN_NAMES.as_slice(),
        _ => &TOTAL_NEG_NOUN_NAMES.as_slice(),
    };
    //查询元件库下的负实体组合
    let refnos = query_filter_deep_children(refno, nouns).await.unwrap();
    if refnos.is_empty() {
        return Ok(HashMap::new());
    }
    //使用SUL_DB通过这些参考号反过来query查找父节点
    let sql = format!(
         "select pos, array::group(id) as negs from (select $this.id as id, array::first(->pe_owner.out) as pos from [{}]) group pos",
         refnos.iter().map(|x| x.to_pe_key()).collect::<Vec<_>>().join(","),
     );
    // println!("query_refno_has_pos_neg_map sql is {}", &sql);
    let mut response = super::staging::data_db().query(&sql).await?;
    let mut result = HashMap::new();
    if let Ok(r) = response.take::<Vec<RefnoHasNegPosInfo>>(0) {
        for info in r {
            result.insert(info.pos, info.negs);
        }
    }
    Ok(result)
}

/// 查询具有正负实体映射关系的参考号集合
///
/// # 参数
/// - `refno`: 参考号数组
/// - `is_cata`: 是否是元件库里的负实体查询
///
/// # 返回
/// 返回一个哈希映射，其中键是参考号，值是具有正负实体映射关系的参考号信息列表
pub async fn query_refnos_has_pos_neg_map(
    refno: &[RefnoEnum],
    is_cata: Option<bool>,
) -> anyhow::Result<HashMap<RefnoEnum, Vec<RefnoEnum>>> {
    let mut result = HashMap::new();
    for &refno in refno {
        let mut map = query_refno_has_pos_neg_map(refno, is_cata).await?;
        result.extend(map.drain());
    }
    Ok(result)
}

/// 查询bran下所有元件的点集
pub async fn query_bran_children_point_map(refno: RefnoEnum) -> anyhow::Result<Vec<InstPointMap>> {
    let sql = format!(
        "select in.id as refno,in.id->inst_relate.out.ptset[0]?:{{}} as ptset_map,in.noun as att_type from pe:{}<-pe_owner;",
        refno.to_string()
    );
    let mut response = super::staging::data_db().query(&sql).await?;
    match response.take(0) {
        Ok(r) => Ok(r),
        Err(e) => {
            dbg!(format!("sql 查询出错: {}", sql));
            dbg!(&e);
            Ok(vec![])
        }
    }
}

#[tokio::test]
async fn test_query_bran_children_point_map() {
    init_test_surreal().await;
    let refno = RefU64::from_str("24383/67947").unwrap();
    let r = query_bran_children_point_map(refno.into()).await.unwrap();
    dbg!(&r);
}

/// 查询参考号对应的点集
pub async fn query_point_map(refno: RefnoEnum) -> anyhow::Result<Option<InstPointMap>> {
    let sql = format!(
        "select id as refno,id->inst_relate.out.ptset[0]?:{{}} as ptset_map,noun as att_type from {};",
        refno.to_pe_key()
    );
    let mut response = super::staging::data_db().query(&sql).await?;
    let Ok(mut result) = response.take::<Vec<InstPointMap>>(0) else {
        dbg!(format!("sql 查询出错: {}", sql));
        return Ok(None);
    };
    if result.is_empty() {
        return Ok(None);
    }
    Ok(Some(result.remove(0)))
}

/// 查询多个参考号对应的点集
pub async fn query_refnos_point_map(
    refnos: Vec<RefnoEnum>,
) -> anyhow::Result<HashMap<RefnoEnum, InstPointMap>> {
    let refnos = refnos
        .into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>();
    let sql = format!(
        "select id as refno,id->inst_relate.out.ptset[0]?:{{}} as ptset_map,noun as att_type from [{}];",
        refnos.join(",")
    );
    let mut response = super::staging::data_db().query(&sql).await?;
    let Ok(result) = response.take::<Vec<InstPointMap>>(0) else {
        dbg!(format!("sql 查询出错: {}", sql));
        return Ok(HashMap::default());
    };
    Ok(result.into_iter().map(|r| (r.refno, r)).collect())
}

///通过geo hash 查询参考号
pub async fn query_refnos_by_geo_hash(id: &str) -> anyhow::Result<Vec<RefnoEnum>> {
    let sql = format!(
        "array::distinct(array::flatten(select value in<-inst_relate.in from inst_geo:⟨{}⟩<-geo_relate));",
        id
    );
    let mut response = super::staging::data_db().query(&sql).await?;
    let result: Vec<RefnoEnum> = response.take(0)?;
    Ok(result)
}

/// 获取arrive和leave的世界坐标
pub fn get_arrive_leave_info(
    refno: RefU64,
    point_map: &HashMap<RefU64, InstPointMap>,
    attr: &NamedAttrMap,
    transform: Transform,
) -> (Vec3, Vec3) {
    let mut arrive_pos = Vec3::ZERO;
    let mut leave_pos = Vec3::ZERO;
    if let Some(points) = point_map.get(&refno) {
        if let Some(NamedAttrValue::IntegerType(arrive)) = attr.get_val("ARRI") {
            if let Some(point_info) = points.ptset_map.get(&arrive.to_string()) {
                let arrive_point = transform.transform_point(point_info.pt);
                arrive_pos = arrive_point;
            }
            if let Some(NamedAttrValue::IntegerType(leave)) = attr.get_val("LEAV") {
                if let Some(point_info) = points.ptset_map.get(&leave.to_string()) {
                    let leave_point = transform.transform_point(point_info.pt);
                    leave_pos = leave_point;
                }
            }
        }
    }
    (arrive_pos, leave_pos)
}

#[tokio::test]
async fn test_query_refnos_point_map() -> anyhow::Result<()> {
    init_test_surreal().await;
    let refno = RefnoEnum::from("24383/101165");
    let r = query_refnos_point_map(vec![refno]).await?;
    dbg!(&r);
    let bran_refno = RefnoEnum::from("24383/101155");
    let r = query_bran_children_point_map(bran_refno).await?;
    dbg!(&r);
    let r = query_point_map(refno).await?;
    dbg!(&r);
    Ok(())
}

//query_ptset
/// 查询RefnoEnum对应的点集合
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PtsetResult {
    pub transform: Transform,
    pub points: Vec<Vec3>,
}

pub async fn query_ptset(refno: RefnoEnum) -> anyhow::Result<Option<PtsetResult>> {
    let sql = format!(
        "(select world_trans.d as transform, object::values(out.ptset?:{{}}).pt as points from {0})[0]",
        to_table_key!(refno, "inst_relate")
    );
    let mut response = super::staging::data_db().query(&sql).await?;
    let result: Option<PtsetResult> = response.take(0)?;
    // dbg!(&result);
    Ok(result)
}

/// 查询RefnoEnum对应的关键点集合
pub async fn query_key_points(refno: RefnoEnum) -> anyhow::Result<Option<Vec<Vec3>>> {
    // let sql = format!(
    //     "select value inst_relate.pts.*.d from {}",
    //     refno.to_pe_key()
    // );
    // let mut response = super::staging::data_db().query(&sql).await?;
    // let result: Option<Vec<Vec3>> = response.take(0)?;
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_test_surreal;

    #[tokio::test]
    async fn test_query_ptset() -> anyhow::Result<()> {
        // Initialize the test database
        init_test_surreal().await;

        // Create a test RefnoEnum
        let refno = "17496_170587".into();

        // Query the point set
        let result = query_ptset(refno).await?;
        assert!(result.is_some(), "Point set result should not be None");

        let ptset_result = result.unwrap();
        let transform = ptset_result.transform;
        let points = ptset_result.points;

        // Perform assertions
        assert!(!points.is_empty(), "Point set should not be empty");

        // Check if the transform is valid
        assert!(transform.is_finite(), "Transform should be finite");

        // Check if all points are valid Vec3
        for point in &points {
            assert!(
                point.x.is_finite() && point.y.is_finite() && point.z.is_finite(),
                "All components of the point should be finite"
            );
        }

        // You might want to add more specific assertions based on expected data
        // For example, checking for a specific number of points or specific point values

        Ok(())
    }
}
