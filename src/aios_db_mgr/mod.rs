use crate::pdms_types::{EleTreeNode, PdmsElement};
use crate::{AttrMap, NamedAttrMap, RefU64};
use async_trait::async_trait;
use bevy_transform::prelude::Transform;

pub mod aios_mgr;

#[async_trait]
pub trait PdmsDataInterface {
    /// 获取 world 节点
    async fn get_world(&self, mdb_name: &str) -> anyhow::Result<Option<PdmsElement>>;
    /// 获取该参考号的PdmsElement数据
    async fn get_pdms_element(&self, refno: RefU64) -> anyhow::Result<Option<PdmsElement>>;
    /// 获得属性
    async fn get_attr(&self, refno: RefU64) -> anyhow::Result<NamedAttrMap>;
    /// 获取children
    async fn get_children(&self, refno: RefU64) -> anyhow::Result<Vec<EleTreeNode>>;
    /// 获取bran的ipara
    async fn get_ipara_from_bran(&self, refno: RefU64) -> anyhow::Result<Vec<f32>>;
    /// 通过name获取PdmsElement
    async fn get_ele_from_name(&self, name: &str) -> anyhow::Result<Option<PdmsElement>>;
    /// 获取spre的属性
    async fn get_spre_attr(&self, refno: RefU64) -> anyhow::Result<Option<NamedAttrMap>>;
    /// 获取catr的属性
    async fn get_catr_attr(&self, refno: RefU64) -> anyhow::Result<Option<NamedAttrMap>>;
    /// 获取外键(一层)的属性
    /// catr这种就为两层
    async fn get_foreign_attr(
        &self,
        refno: RefU64,
        foreign_type: &str,
    ) -> anyhow::Result<Option<NamedAttrMap>>;
    /// 获取节点的name
    async fn get_name(&self, refno: RefU64) -> anyhow::Result<String>;
    /// 获得指定参考号的世界坐标系
    async fn get_world_transform(&self, refno: RefU64) -> anyhow::Result<Option<Transform>>;
    /// 获取pdms树中，指定节点同层级的上一个
    async fn get_prev(&self, refno: RefU64) -> anyhow::Result<RefU64>;
    /// 获取pdms树中，指定节点同层级的下一个
    async fn get_next(&self, refno: RefU64) -> anyhow::Result<RefU64>;
    /// 获取房间号,包括BRAN以及EQUI
    async fn get_room_code(&self, refno: RefU64) -> anyhow::Result<Option<String>>;
}
