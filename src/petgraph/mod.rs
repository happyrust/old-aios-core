use crate::RefU64;
use crate::noun_graph::NOUN_GRAPH;
use crate::tool::db_tool::db1_dehash;
use petgraph::prelude::*;
use serde_derive::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PetRefnoNode {
    pub id: u64,
    // pub noun: String,
    pub noun_hash: u32,
}

// #[test]
// fn test_serde_refno_node() {
//     let node = PetRefnoNode{
//         id: 123,
//         noun_hash: 456,
//     };
//     let serialized = serde_json::to_string(&node).unwrap();
//     let deserialized: PetRefnoNode = serde_json::from_str(&serialized).unwrap();
//     dbg!(deserialized);
//     // let config = bincode::config::standard();
//     let serialized = bincode::serialize(&node).unwrap();
//     let deserialized: PetRefnoNode = bincode::deserialize(&serialized).unwrap();
//     // let (r, _) : (PetRefnoNode, usize) = bincode::decode_from_slice(&encoded, bincode::config::standard()).unwrap();
//     dbg!(deserialized);
// }

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PetRefnoGraph {
    pub ses_pgno: u32,
    pub graph: DiGraph<PetRefnoNode, ()>,
    pub node_indices: HashMap<u64, NodeIndex>,
}

impl PetRefnoGraph {
    //实现保存成bincode文件的方法
    pub fn save(&self, path: &str) -> anyhow::Result<()> {
        let file = std::fs::File::create(path)?;
        bincode::serialize_into(file, self)?;
        Ok(())
    }

    //实现反序列化bincode文件的方法
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let file = std::fs::File::open(path)?;
        let graph = bincode::deserialize_from(file)?;
        Ok(graph)
    }

    pub fn find_path(&self, start: RefU64, end: RefU64) -> Option<Vec<&PetRefnoNode>> {
        let start_node = self.node_indices.get(&start)?;
        let end_node = self.node_indices.get(&end)?;
        let mut paths = petgraph::algo::all_simple_paths::<Vec<_>, &DiGraph<_, _>>(
            &self.graph,
            *start_node,
            *end_node,
            0,
            None,
        )
        .collect::<Vec<_>>();
        let mut result = Vec::new();
        for path in paths {
            for node in path {
                result.push(&self.graph[node]);
            }
        }
        Some(result)
    }

    pub fn search_path_refnos(
        &self,
        start: RefU64,
        predicate: impl Fn(u32) -> bool,
    ) -> Option<Vec<RefU64>> {
        self.search_path(start, predicate)
            .map(|nodes| nodes.iter().map(|node| RefU64(node.id)).collect::<Vec<_>>())
    }

    pub fn search_path(
        &self,
        start: RefU64,
        predicate: impl Fn(u32) -> bool,
    ) -> Option<Vec<&PetRefnoNode>> {
        // let mut visited: HashSet<NodeIndex> = HashSet::new();
        let mut result: Vec<NodeIndex> = Vec::new();
        let mut queue: VecDeque<NodeIndex> = VecDeque::new();
        let start_indice = *self.node_indices.get(&start)?;

        let graph = &self.graph;

        // visited.insert(start);
        queue.push_back(start_indice);

        while let Some(node) = queue.pop_front() {
            let noun = graph[node].noun_hash;

            if predicate(noun) {
                result.push(node);
            }

            for neighbor in graph.neighbors(node) {
                // if !visited.contains(&neighbor) {
                //     visited.insert(neighbor);
                queue.push_back(neighbor);
                // }
            }
        }
        Some(result.iter().map(|&i| &graph[i]).collect::<Vec<_>>())
    }
}
