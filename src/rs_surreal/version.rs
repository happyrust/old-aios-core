use crate::{RefU64, RefnoEnum, SUL_DB};

/// 将数据备份到history
pub async fn backup_data(
    refnos: impl Iterator<Item = &RefU64>,
    is_deleted: bool,
    sesno: u32,
) -> anyhow::Result<()> {
    let pe_keys = refnos
        .into_iter()
        .map(|x| x.to_pe_key())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!("fn::backup_data([{}], {}, {});", pe_keys, is_deleted, sesno);
    // println!("sql is {}", &sql);
    let mut response = SUL_DB.query(&sql).await.unwrap();
    // dbg!(&response);
    let mut erros = response.take_errors();
    if !erros.is_empty() {
        dbg!(&sql);
        dbg!(&erros);
    }
    Ok(())
}

/// 备份owner关系
pub async fn backup_owner_relate(refnos: impl Iterator<Item = &RefU64>) -> anyhow::Result<()> {
    let pe_keys = refnos
        .into_iter()
        .map(|x| x.to_pe_key())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!("fn::backup_owner_relate([{}], true);", pe_keys);
    println!("sql is {}", &sql);
    let mut response = SUL_DB.query(&sql).await.unwrap();
    // dbg!(&response);
    let mut erros = response.take_errors();
    if !erros.is_empty() {
        dbg!(&erros);
    }
    Ok(())
}
