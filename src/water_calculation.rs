use crate::types::*;
use bevy_ecs::prelude::Component;
use bevy_ecs::prelude::Event;
use bevy_ecs::prelude::Resource;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

///水淹计算孔洞的结构体
#[derive(Resource, Clone, Debug, Default, Deserialize, Serialize)]
pub struct FloodingHole {
    ///参考号
    pub refno: RefU64,
    ///名称
    pub name: String,
    ///标记是孔洞还是门洞
    pub is_door: bool,
    ///标记是否被选中
    pub is_selected: bool,
    ///标记是否封堵
    pub is_plugged: bool,
}

///水淹计算墙与孔洞关系对应的结构体
#[derive(Resource, Clone, Debug, Default, Deserialize, Serialize)]
pub struct FloodingHoleVec {
    pub data: HashMap<RefU64, Vec<FloodingHole>>,
}

///导出水淹计算stp需要用到的数据
#[derive(Clone, Debug, Default, Event, Deserialize, Serialize)]
pub struct ExportFloodingStpEvent {
    ///文件名
    pub file_name: String,
    ///保存时间
    pub save_time: String,
    ///所有需要导出的模型列表, refno->name
    pub export_models_map: HashMap<RefU64, String>,
    ///墙与孔洞的对应关系
    pub walls_map: HashMap<RefU64, Vec<FloodingHole>>,
}

impl ExportFloodingStpEvent {
    pub fn export_refnos(&self) -> impl Iterator<Item = &RefU64> + '_ {
        self.export_models_map.keys()
    }

    pub fn walls(&self) -> impl Iterator<Item = &RefU64> + '_ {
        self.walls_map.keys()
    }

    pub fn opening_hole_refnos(&self, wall: RefU64) -> Option<impl Iterator<Item = RefU64> + '_> {
        self.walls_map
            .get(&wall)
            .map(|x| x.iter().filter(|x| !x.is_plugged).map(|x| x.refno))
    }

    ///返回所有封堵了的洞的参考号，排除门的情况
    pub fn all_plugged_hole_refnos(&self) -> impl Iterator<Item = RefU64> + '_ {
        self.walls_map
            .values()
            .map(|x| {
                x.iter()
                    .filter(|x| x.is_plugged && !x.is_door)
                    .map(|x| x.refno)
            })
            .flatten()
    }

    ///返回所有封堵了的门洞的参考号
    pub fn all_plugged_door_refnos(&self) -> impl Iterator<Item = RefU64> + '_ {
        self.walls_map
            .values()
            .map(|x| {
                x.iter()
                    .filter(|x| x.is_plugged && x.is_door)
                    .map(|x| x.refno)
            })
            .flatten()
    }

    // pub fn all_opening_holes(&self) -> Option<&[FloodingHole]>{
    //     self.walls_map.iter().map(|x| x.iter().filter(|x| !x.is_plugged))
    // }

    // pub fn plugged_holes(&self, wall: RefU64) -> Option<&[FloodingHole]>{
    //     self.walls_map.get(wall).map(|x| x.iter().filter(|x| x.is_plugged))
    // }

    pub fn to_arango_struct(self) -> FloodingStpToArangodb {
        let mut hasher = DefaultHasher::new();
        self.file_name.hash(&mut hasher);
        let hash_name = hasher.finish();
        FloodingStpToArangodb {
            _key: hash_name.to_string(),
            save_time: self.save_time,
            export_models_map: self.export_models_map,
            file_name: self.file_name,
            walls_map: self.walls_map,
        }
    }
}

///将水淹计算保存到图数据库
#[derive(Clone, Debug, Default, Event, Deserialize, Serialize)]
pub struct FloodingStpToArangodb {
    pub _key: String,
    //文件名
    pub file_name: String,
    //保存时间
    pub save_time: String,
    //所有选中的模型列表
    pub export_models_map: HashMap<RefU64, String>,
    //所有需要导出的模型列表
    pub walls_map: HashMap<RefU64, Vec<FloodingHole>>,
}

///将数据库中的数据组织成资源，导出历史记录时使用
#[derive(Resource, Clone, Debug, Default, Event, Deserialize, Serialize)]
pub struct FloodingStpToArangodbVec {
    pub data: Vec<FloodingStpToArangodb>,
}
