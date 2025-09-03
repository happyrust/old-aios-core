use crate::metadata::spatial_computation::*;
use crate::{init_test_surreal, RefU64, RefnoEnum};
use glam::Vec3;
use std::str::FromStr;

#[tokio::test]
async fn test_get_supp_panel() {
    init_test_surreal().await;
    let refno = RefU64::from_str("24383/89904").unwrap();
    let name = get_supp_panel(refno.into()).await.unwrap();
    assert_eq!(name, "1RS02TT0265P".to_string());
}

#[tokio::test]
async fn test_get_supp_bran() {
    init_test_surreal().await;
    let refno = RefU64::from_str("24383/92720").unwrap();
    let names = get_supp_bran(refno.into()).await.unwrap();
    assert_eq!(
        names,
        vec!["C-OR-1RB-4020E".to_string(), "C-CO-1RB-4021E".to_string()]
    );
}

#[tokio::test]
async fn test_get_supp_span() {
    init_test_surreal().await;
    let refno = RefU64::from_str("24383/86525").unwrap();
    let size = get_supp_span(refno.into()).await.unwrap();
    assert_eq!(size, [1100.0, 1600.0]);
}

#[tokio::test]
async fn test_get_bran_in_pcla() {
    init_test_surreal().await;
    let refno = RefU64::from_str("24383/69029").unwrap();
    let bran_fixing = get_bran_in_pcla(refno.into()).await.unwrap();
    assert_eq!(bran_fixing, RefU64::from_str("24383/68357").unwrap().into());
}

#[tokio::test]
async fn test_get_panel_size() {
    init_test_surreal().await;
    let refno = RefU64::from_str("24383_68574").unwrap();
    let size = get_panel_size(refno.into()).await.unwrap();
    dbg!(&size);
    assert_eq!(size, [160.0, 50.0]);
}

#[tokio::test]
async fn test_get_positon_with_panel() {
    init_test_surreal().await;
    let refno = RefU64::from_str("25688/47610").unwrap();
    let size = get_position_with_panel(refno.into()).await.unwrap();
    assert_eq!(size, Vec3::from([501.061, -174.729, 122.0]));
}
