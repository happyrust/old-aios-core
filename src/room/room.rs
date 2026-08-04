use std::io::{Read, Write};
use once_cell::sync::Lazy;
use tokio::sync::RwLock;

use crate::{
    accel_tree::acceleration_tree::{AccelerationTree, RStarBoundingBox},
    SUL_DB, RefU64,
};

//或者改成第一次，需要去加载，后续就不用了
//启动的时候就要去加载到内存里
pub static GLOBAL_AABB_TREE: Lazy<RwLock<AccelerationTree>> =
    Lazy::new(|| RwLock::new(AccelerationTree::default()));

// 曾经这里还有一棵只装房间面板的 GLOBAL_ROOM_AABB_TREE，供 query_room_panel_by_point
// 反向查询用。它已被移除（ADR-010 §6）：唯一的填充入口 load_room_aabb_tree 里 SQL
// 括号未闭合、内层 select 无主表，解析必失败，那棵树从来没被填过；而两棵树并存本身
// 也意味着两套增量维护、两处会漂移。反向候选现在改为在下面这棵全局树上按 noun 过滤。

// 不要每次都加载，需要检查缓存，如果缓存有，就不用从数据库里刷新了
#[cfg(not(target_arch = "wasm32"))]
pub async fn load_aabb_tree() -> anyhow::Result<bool> {
    // 如果已生成了空间树的文件，直接读取文件中的数据即可
    //改成使用 AccelerationTree 的反序列化方法
    //不用重复加载
    if !GLOBAL_AABB_TREE.read().await.is_empty() {
        return Ok(true);
    }
    #[cfg(not(feature = "web"))]
    {
        // 缺文件 / 文件损坏都降级为空树并告警，交给 sync_aabb_tree_with_db 对账重建。
        // 静默 unwrap_or_default 的问题在于：空树跑增量时元素分支「先删后写」会拿不到
        // 候选面板，悄悄抹掉存量房间边，而没有任何日志解释树为什么是空的。
        *GLOBAL_AABB_TREE.write().await = match AccelerationTree::deserialize_from_bin_file() {
            Ok(tree) => tree,
            Err(error) => {
                eprintln!(
                    "加载 accel_tree.bin 失败（{error:#}），空间树从空树开始，等待与库对账重建"
                );
                AccelerationTree::default()
            }
        };
    }
    // {
    //     if !GLOBAL_AABB_TREE.read().await.is_empty() {
    //         return Ok(true);
    //     }
    // }
    // //如果有缓存文件，直接读取缓存文件
    // //测试分页查询
    // let mut rstar_objs = vec![];
    // let mut offset = 0;

    // let page_count = 1000;
    // loop {
    //     //需要过滤
    //     let sql = format!(
    //         "select in as refno, aabb.d.* as aabb, in.noun as noun from inst_relate where aabb.d!=none and aabb.d.mins[0] < 1000000 and solid start {} limit {page_count}",
    //         offset
    //     );
    //     let mut response = SUL_DB.query(&sql).await?;
    //     let refno_aabbs: Vec<RStarBoundingBox> = response.take(0).unwrap();
    //     if refno_aabbs.is_empty() {
    //         break;
    //     }
    //     rstar_objs.extend(refno_aabbs);
    //     offset += page_count;
    // }

    // //存储在全局变量里, 每次都重新加载，还是就用数据文件来表达？当做资源来加载，不用每次都去加载
    // //加个时间戳，来表达是不是最新的rtree
    // let tree = AccelerationTree::load(rstar_objs);

    // // 将查询好的数据写入到文件中
    // let json = serde_json::to_string(&tree)?;
    // let mut file = std::fs::File::create_new("spa_tree.json")?;
    // file.write_all(&json.into_bytes())?;

    // tree.serialize_to_bin_file();
    // *GLOBAL_AABB_TREE.write().await = tree;
    Ok(false)
}



