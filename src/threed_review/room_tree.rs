use crate::pdms_types::PdmsElement;
use crate::{RefU64, SUL_DB};
use id_tree::Tree;

/// 获取房间树
pub async fn get_room_tree(refno: RefU64) -> anyhow::Result<Tree<PdmsElement>> {
    // 找到 bran 所在的所有房间
    let rooms = get_rooms_by_bran(refno).await?;
    Ok(Tree::new())
}

/// 获取 bran 穿过的所有房间
async fn get_rooms_by_bran(refno: RefU64) -> anyhow::Result<Vec<String>> {
    let sql = format!("return fn::get_room_names({});", refno.to_pe_key());
    let mut res = SUL_DB.query(sql).await?;
    let rooms = res.take(0)?;
    Ok(rooms)
}
