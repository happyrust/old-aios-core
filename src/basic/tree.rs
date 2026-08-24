use crate::RefU64;
use crate::pdms_types::{EleTreeNode, PdmsNodeTrait};
use bevy_ecs::component::Component;
use derive_more::{Deref, DerefMut};
use glam::{bool, u32};
use id_tree::Tree;
use serde_derive::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use surrealdb::sql::Thing;

pub type E3dTree = ElementTree<EleTreeNode>;

#[derive(Serialize, Deserialize, Clone, Debug, Default, Component, Deref, DerefMut)]
pub struct ElementTree<T: PdmsNodeTrait>(pub Tree<T>);

impl<T: PdmsNodeTrait> ElementTree<T> {
    ///获得世界refno
    #[inline]
    pub fn get_root_refno(&self) -> Option<RefU64> {
        self.root_node_id()
            .map(|x| self.get(x).map(|t| t.data().get_refno()).ok())
            .flatten()
    }

    #[inline]
    pub fn get_root_id(&self) -> Option<Thing> {
        self.root_node_id()
            .map(|x| self.get(x).map(|t| t.data().get_id().cloned()).ok())
            .flatten()?
    }
}
