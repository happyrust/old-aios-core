use crate::SUL_DB;
use crate::parsed_data::CateAxisParam;
use crate::pdms_types::*;
use crate::{NamedAttrMap, RefU64, rs_surreal};
use glam::Vec3;
use std::sync::Arc;
use surrealdb::sql::Thing;

// #[tokio::test]
// async fn test_query_pe_by_refno() -> anyhow::Result<()> {
//     crate::init_test_surreal().await;
//     let refno = "13292_92".into();
//     let pe = rs_surreal::get_pe(refno).await.unwrap();
//     dbg!(pe);
//     Ok(())
// }

// #[tokio::test]
// async fn test_build_spre_relates() -> anyhow::Result<()> {
//     crate::init_test_surreal().await;
//     rs_surreal::build_cate_relate(false).await?;
//     let refnos = rs_surreal::query_ele_refnos_by_spres(&["13245/660780".into()]).await?;
//     dbg!(refnos);
//     Ok(())
// }

// #[tokio::test]
// async fn test_query_ancestor_by_refno() -> anyhow::Result<()> {
//     crate::init_test_surreal().await;
//     let refno: RefU64 = "17496_171659".into();
//     let type_name = rs_surreal::get_type_name(refno).await.unwrap_or_default();
//     dbg!(&type_name);
//     let ancestor = rs_surreal::get_ancestor(refno).await.unwrap();
//     dbg!(ancestor);
//     let ancestor_maps = rs_surreal::get_ancestor_attmaps(refno).await.unwrap();
//     dbg!(ancestor_maps);
//     Ok(())
// }

// #[tokio::test]
// async fn test_query_index_of_parent() -> anyhow::Result<()> {
//     // crate::init_test_surreal().await;
//     // let parent: RefU64 = "17496_273491".into();
//     // let refno: RefU64 = "17496_273497".into();
//     // let no_type_filter_index = rs_surreal::get_index_by_noun_in_parent(parent.into(), refno.into(), None)
//     //     .await
//     //     .unwrap();
//     // dbg!(no_type_filter_index);
//     // let ENDATU_filter_index =
//     //     rs_surreal::get_index_by_noun_in_parent(parent, refno, Some("ENDATU"))
//     //         .await
//     //         .unwrap();
//     // dbg!(ENDATU_filter_index);
//     Ok(())
// }

// #[tokio::test]
// async fn test_query_wtrans_by_refno() -> anyhow::Result<()> {
//     crate::init_test_surreal().await;
//     // let wtrans = mgr.get_world_transform("17496_118635".into()).await.unwrap();
//     // dbg!(wtrans);
//     //todo fix POSL attribute
//     // let wtrans = mgr.get_world_transform("17496_107068".into()).await.unwrap();
//     // dbg!(wtrans);

//     // let wtrans = mgr
//     //     .get_world_transform("17496_259211".into())
//     //     .await
//     //     .unwrap();
//     // assert_eq!(
//     //     wtrans.unwrap().translation,
//     //     Vec3::new(79800.0, -19000.0, 3460.0)
//     // );
//     Ok(())
// }

// #[tokio::test]
// async fn test_query_att_by_refno() {
//     crate::init_test_surreal().await;

//     let refno: RefU64 = "17496_292217".into();
//     // let sql = format!(
//     //     r#"select value refno.* from only {} limit 1;"#, refno.to_pe_key()
//     // );
//     // let mut response = SUL_DB
//     //     .query(sql)
//     //     .await.unwrap();
//     //
//     // let o: surrealdb::Value = response.take(0).unwrap();
//     // dbg!(&o);
//     // let named_attmap: NamedAttrMap = o.into_inner().into();
//     // dbg!(&named_attmap);

//     let attmap = rs_surreal::get_named_attmap(refno.into()).await;
//     dbg!(attmap);
//     // let eles = rs_surreal::get_children_pes("24383/74426".into()).await;
//     // dbg!(eles);
// }

// #[tokio::test]
// async fn test_get_siblings_by_refno() {
//     crate::init_test_surreal().await;
//     let refnos = rs_surreal::get_siblings("17496/258778".into())
//         .await
//         .unwrap();
//     dbg!(refnos);
//     let next = rs_surreal::get_next_prev("17496/258778".into(), true)
//         .await
//         .unwrap();
//     let prev = rs_surreal::get_next_prev("17496/258778".into(), false)
//         .await
//         .unwrap();
//     dbg!((next, prev));
// }

// #[tokio::test]
// async fn test_query_children() {
//     crate::init_test_surreal().await;
//     // let refnos = rs_surreal::get_children_refnos("9304_0".into()).await;
//     // dbg!(refnos);
//     let nodes = rs_surreal::get_children_ele_nodes("17496_256208".into())
//         .await
//         .unwrap();
//     dbg!(nodes);

//     let children = rs_surreal::get_children_refnos("17496_256208".into())
//         .await
//         .unwrap();
//     dbg!(children);
// }

// #[tokio::test]
// async fn test_query_children_att() {
//     crate::init_test_surreal().await;
//     let children_pes =
//         rs_surreal::query_filter_children("17496/195273".into(), &GENRAL_NEG_NOUN_NAMES).await;
//     dbg!(children_pes);
// }

// #[tokio::test]
// async fn test_query_custom() -> anyhow::Result<()> {
//     crate::init_test_surreal().await;
//     let mut response = SUL_DB
//         .query(r#"(select owner, owner.noun as o_noun from type::thing("pe", $refno) )[0]"#)
//         .bind(("refno", "17496_171555"))
//         .await
//         .unwrap();
//     let owner_noun: Option<String> = response.take("o_noun").unwrap();
//     dbg!(owner_noun);
//     let owner: RefU64 = response.take::<Option<String>>("owner")?.unwrap().into();
//     dbg!(owner);
//     Ok(())
// }

// #[tokio::test]
// async fn test_query_custom_bend() -> anyhow::Result<()> {
//     crate::init_test_surreal().await;
//     let mut response = SUL_DB
//         .query(r#"
//         select
// id as id,
// string::split(string::split(if refno.SPRE.name == NONE { "//:" } else { refno.SPRE.name },'/')[2],':')[0] as code, // 编码
// refno.TYPE as noun, // 部件
// math::fixed((refno.ANGL / 360) * 2 *3.1415 * refno.SPRE.refno.CATR.refno.PARA[1],2) as count // 长度
// from $refnos
//         "#)
//         .bind(("refnos", [Thing::from(("pe", "24383_84092"))] ) )
//         .await
//         .unwrap();
//     dbg!(response);
//     // let owner_noun: Option<String> = response.take("o_noun").unwrap();
//     // dbg!(owner_noun);
//     // let owner: RefU64 = response.take::<Option<String>>("owner")?.unwrap().into();
//     // dbg!(owner);
//     Ok(())
// }

#[tokio::test]
async fn test_query_attmap_WELD() -> anyhow::Result<()> {
    crate::init_test_surreal().await;

    let refno: RefnoEnum = RefU64::from("17496_268302").into();
    let attmap = rs_surreal::get_ui_named_attmap(refno).await.unwrap();
    dbg!(attmap);
    Ok(())
}

#[tokio::test]
async fn test_query_attmap_SCTN() -> anyhow::Result<()> {
    // 初始化测试环境
    crate::init_test_surreal().await;

    // 创建 RefnoEnum 实例
    let refno: RefnoEnum = RefnoSesno::new("17496/265703".into(), 0).into();
    // 获取 attmap
    let attmap = rs_surreal::get_ui_named_attmap(refno).await.unwrap();
    // dbg!(&attmap);
    // 断言 JUSL 字段的值为 TOS
    assert_eq!(attmap.get_str("JUSL"), Some("TOS"));
    Ok(())
}

#[tokio::test]
async fn test_query_attmap() -> anyhow::Result<()> {
    crate::init_test_surreal().await;

    let refno: RefnoEnum = RefnoSesno::new("17496_265703".into(), 961).into();
    let attmap = rs_surreal::get_ui_named_attmap(refno).await.unwrap();
    dbg!(attmap);

    let children =
        rs_surreal::query_multi_deep_versioned_children_filter_inst(&[refno], &[], false).await?;
    dbg!(&children);

    let pe = rs_surreal::get_pe(refno).await.unwrap();
    dbg!(pe);

    let refno = "17496_171640,733".into();
    dbg!(&refno);
    let world_trans = rs_surreal::get_world_transform(refno).await.unwrap();
    dbg!(world_trans);

    let refno = "17496_171640".into();
    dbg!(&refno);
    let world_trans = rs_surreal::get_world_transform(refno).await.unwrap();
    dbg!(world_trans);

    // let history_pes = rs_surreal::query_history_pes(refno).await.unwrap();
    // dbg!(&history_pes);

    //test query children full names
    let refno = "17496_171606,733".into();
    let children_names = rs_surreal::query_children_full_names_map(refno)
        .await
        .unwrap();
    dbg!(&children_names);

    // let children = rs_surreal::get_children_named_attmaps(refno).await.unwrap();
    // dbg!(&children);

    // let world = rs_surreal::get_world_refno("/ALL".into()).await.unwrap();
    // dbg!(world);

    //select value ptset from inst_info limit 1

    Ok(())
}

// #[tokio::test]
// async fn test_query_ptset() -> anyhow::Result<()> {
//     crate::init_test_surreal().await;
//     use std::collections::BTreeMap;
//     let sql = "select value ptset from inst_info limit 1";
//     let mut response = SUL_DB.query(sql).await?;
//     dbg!(&response);
//     let ptset: Option<BTreeMap<String, CateAxisParam>> = response.take(0).unwrap();
//     dbg!(ptset);

//     //select value ptset from inst_info limit 1

//     Ok(())
// }

// #[tokio::test]
// async fn test_query_cata() -> anyhow::Result<()> {
//     crate::init_test_surreal().await;

//     let refno = "17496/171646".into();
//     let cat_refno = rs_surreal::get_cat_refno(refno).await?.unwrap();
//     assert_eq!(cat_refno, "13245_888353".into());

//     // get_cat_attmap
//     let cat_attmap = rs_surreal::get_cat_attmap(refno).await?;
//     dbg!(cat_attmap);

//     let refno = "17496/171647".into();
//     let cat_refno = rs_surreal::get_cat_refno(refno).await?.unwrap();
//     assert_eq!(cat_refno, "13245_887529".into());
//     // get_cat_attmap
//     let cat_attmap = rs_surreal::get_cat_attmap(refno).await.unwrap();
//     dbg!(cat_attmap);

//     let refno = "17496/172806".into();
//     let cat_refno = rs_surreal::get_cat_refno(refno).await.unwrap();
//     dbg!(cat_refno);

//     Ok(())
// }

// #[tokio::test]
// async fn test_query_path() -> anyhow::Result<()> {
//     crate::init_test_surreal().await;
//     let cat_refno = rs_surreal::query_single_by_paths("15194/5835".into(), &["->GMRE"], &[])
//         .await
//         .unwrap();
//     dbg!(cat_refno);
//     Ok(())
// }

// #[tokio::test]
// async fn test_query_paths() -> anyhow::Result<()> {
//     crate::init_test_surreal().await;
//     let cat_refno =
//         rs_surreal::query_single_by_paths("15194/5835".into(), &["->GMRE", "->GMSR"], &["id"])
//             .await
//             .unwrap();
//     dbg!(cat_refno);
//     Ok(())
// }

// #[tokio::test]
// async fn test_query_record_link() -> anyhow::Result<()> {
//     crate::init_test_surreal().await;

//     let str1 = r#"
//             {
//     "id": "25688_32684",
//     "refno": FITT:25688_32684,
//     "owner": pe:25688_32682,
//     "name": "/1AR07WW0002R",
//     "noun": "FITT",
//     "dbnum": 1112,
//     "sesno": 0,
//     "cata_hash": "110329119932332",
//     "lock": false
// }
//         "#;

//     let str2 = r#"
//     {
//          "id": "25688_32682",
//         "refno": TEST:25688_32682,
//         "owner": pe:25688_32681,
//         "name": "/1AR07WW0002R",
//         "noun": "TEST",
//         "dbnum": 1112,
//         "sesno": 0,
//         "cata_hash": "110329119932332",
//         "lock": false
//     }
//         "#;

//     // let d1: pdms_element::Model = serde_json::from_str(
//     //     str1
//     // ).unwrap();
//     //
//     // let d2: pdms_element::Model = serde_json::from_str(
//     // str2
//     // ).unwrap();

//     SUL_DB
//         .query("delete pe:25688_32684; delete pe:25688_32682;")
//         // .query("use ns 1516;use db AvevaMarineSample;INSERT IGNORE INTO pe $values")
//         // .bind(("values", &[str1, str2]))
//         .await
//         .unwrap();

//     // let response = SUL_DB
//     //     .query("use ns 1516;use db AvevaMarineSample;INSERT IGNORE INTO pe $values")
//     //     .bind(("values", &[serde_json::to_string_pretty(&d1).unwrap(),
//     //         serde_json::to_string_pretty(&d2).unwrap()]))
//     //     .await
//     //     .unwrap();
//     // let s1 = serde_json::to_string_pretty(&d1).unwrap();
//     // let s2 = serde_json::to_string_pretty(&d2).unwrap();
//     // let mut v1: serde_json::Value = serde_json::to_value(d1.clone()).unwrap();
//     // v1.as_object_mut().unwrap().insert("owner".into(), format!("pe:{}", d1.owner.to_string()).into());
//     //
//     // let mut v2: serde_json::Value = serde_json::to_value(d2).unwrap();
//     // v2.as_object_mut().unwrap().insert("owner".into(), format!("pe:{}", "0_0").into());

//     let mut sql = format!(
//         r#"use ns 1516;use db AvevaMarineSample;INSERT IGNORE INTO pe [{}, {}];"#,
//         // serde_json::to_string_pretty(&[str1, str2]).unwrap()
//         str1,
//         str2
//     );

//     println!("{}", &sql);

//     let response = SUL_DB
//         .query(&sql)
//         // .query("use ns 1516;use db AvevaMarineSample;INSERT IGNORE INTO pe $values")
//         // .bind(("values", &[str1, str2]))
//         .await
//         .unwrap();

//     let q = SUL_DB
//         .query("select owner.* from pe:25688_32684;")
//         .await
//         .unwrap();
//     dbg!(q);
//     // let q1 = SUL_DB.query("select owner.* from pe:17496_100102;").await.unwrap();
//     // dbg!(q1);
//     //
//     Ok(())
// }

// //test query_prev_version_refno
// #[tokio::test]
// async fn test_query_prev_version_refno() -> anyhow::Result<()> {
//     crate::init_test_surreal().await;
//     let refno = rs_surreal::query_prev_version_refno("17496_272476".into()).await.unwrap();
//     dbg!(refno);
//     Ok(())
// }

#[tokio::test]
async fn test_query_ancestor_of_type() -> anyhow::Result<()> {
    crate::init_test_surreal().await;

    let refno = RefnoEnum::from("pe:24383_73928");

    let site_ancestor = rs_surreal::query_ancestor_of_type(refno, "SITE".to_string()).await?;
    assert!(site_ancestor.is_some());
    let site_ancestor = site_ancestor.unwrap();
    assert_eq!(site_ancestor.to_string(), "24383_73927");

    let non_existent = rs_surreal::query_ancestor_of_type(refno, "NONEXISTENT".to_string()).await?;
    assert!(non_existent.is_none());

    Ok(())
}

#[tokio::test]
async fn test_get_named_attmap_with_uda() -> anyhow::Result<()> {
    crate::init_test_surreal().await;
    use crate::NamedAttrValue;
    let refno = RefnoEnum::from("pe:24383_66563");
    let attmap = rs_surreal::get_named_attmap_with_uda(refno).await?;
    dbg!(&attmap);

    // Basic attributes should exist
    assert!(attmap.contains_key("NAME"));
    assert!(attmap.contains_key("REFNO"));

    // Check UDA values
    // Get both default UDAs and overwritten UDAs
    let uda_keys: Vec<_> = attmap.map.keys().filter(|k| k.starts_with(":")).collect();
    assert!(!uda_keys.is_empty(), "Should contain UDA attributes");

    // Verify UDA values are properly typed
    // for key in uda_keys {
    //     let value = attmap.map.get(key).unwrap();
    //     match value {
    //         NamedAttrValue::StringType(_) |
    //         NamedAttrValue::IntegerType(_) |
    //         // NamedAttrValue::FloatType(_) |
    //         // NamedAttrValue::BooleanType(_) => {},
    //         _ => panic!("Unexpected UDA value type for key {}", key)
    //     }
    // }

    Ok(())
}
