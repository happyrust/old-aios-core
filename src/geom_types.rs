use bevy_transform::prelude::Transform;
use parry3d::bounding_volume::Aabb;
use serde::{Deserialize, Serialize};
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::types::*;
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use std::borrow::BorrowMut;
use std::hash::{Hash, Hasher};
use crate::geometry::EleInstGeo;
use crate::parsed_data::geo_params_data::PdmsGeoParam::PrimSCylinder;
use crate::geometry::GeoBasicType;
use crate::shape::pdms_shape::RsVec3;

#[derive(Serialize, Deserialize, Debug, Default,Clone)]
pub struct RvmGeoInfo {
    pub _key: String,
    pub aabb: Option<Aabb>,
    pub data: Vec<EleInstGeo>,
    // 相对世界坐标系下的变换矩阵 rot, translation, scale
    pub world_transform: Transform,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Default,Clone)]
pub struct RvmGeoInfos {
    #[serde_as(as = "DisplayFromStr")]
    pub refno: RefU64,
    pub att_type: String,
    pub world_transform: Transform,
    pub rvm_inst_geo: Vec<RvmInstGeo>,
}


impl RvmGeoInfos {

    ///获得关键点
    pub fn key_points(&self) -> Vec<RsVec3>{
        self.rvm_inst_geo.iter()
            .map(|x|
                x.geo_param.key_points()
                .into_iter()
                .map(|v| self.world_transform.transform_point(*v).into())
            )
            .flatten()
            .collect()
    }


}


#[serde_as]
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RvmTubiGeoInfos {
    #[serde_as(as = "DisplayFromStr")]
    pub refno: RefU64,
    pub att_type: String,
    pub aabb: Option<Aabb>,
    pub world_transform: Transform,
    pub rvm_inst_geo: Vec<RvmInstGeo>,
}

impl RvmTubiGeoInfos {
    pub fn into_rvmgeoinfos(self) -> RvmGeoInfos {
        let mut geos = self.rvm_inst_geo;
        for geo in geos.iter_mut() {
            match geo.geo_param.borrow_mut() {
                PrimSCylinder(data) => {
                    data.phei = self.world_transform.scale.z;
                    data.pdia = self.world_transform.scale.x;
                }
                _ => { continue; }
            }
            geo.aabb = self.aabb;
        }
        RvmGeoInfos {
            refno: self.refno,
            att_type: self.att_type,
            world_transform: self.world_transform,
            rvm_inst_geo: geos,
        }
    }
}

/// rvm 需要的 元件 geo 数据
#[derive(Serialize, Deserialize, Debug, Default,Clone)]
pub struct RvmInstGeo {
    pub geo_param: PdmsGeoParam,
    pub geo_hash: String,
    pub aabb: Option<Aabb>,
    //相对于自身的坐标系变换
    pub transform: Transform,
    pub visible: bool,
    pub is_tubi: bool,
    pub geo_type: GeoBasicType,
}

impl RvmInstGeo {

    #[inline]
    pub fn key_points(&self) -> Vec<RsVec3>{
        self.geo_param.key_points()
    }
}

