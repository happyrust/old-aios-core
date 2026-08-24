use crate::parsed_data::CateAxisParam;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::pdms_types::PdmsGenericType;
#[cfg(feature = "occ")]
use crate::prim_geo::basic::OccSharedShape;
use crate::prim_geo::basic::{BOXI_GEO_HASH, TUBI_GEO_HASH};
use crate::prim_geo::{SBox, SCylinder};
use crate::shape::pdms_shape::{PlantMesh, RsVec3};
use crate::tool::hash_tool::hash_two_str;
use crate::{RefU64, RefnoEnum, gen_bytes_hash};
use bevy_ecs::prelude::Resource;
#[cfg(feature = "render")]
use bevy_render::mesh::PrimitiveTopology::TriangleList;
#[cfg(feature = "render")]
use bevy_render::mesh::{Indices, Mesh};
#[cfg(feature = "render")]
use bevy_render::render_asset::RenderAssetUsages;
use bevy_transform::components::Transform;
use dashmap::DashSet;
use glam::{Vec3, bool, i32, u64};
use nalgebra::Point3;
#[cfg(feature = "occ")]
use opencascade::primitives::{IntoShape, Shape};
use parry3d::bounding_volume::Aabb;
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// 几何体的基本类型
#[derive(
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Serialize,
    Deserialize,
    PartialEq,
    Debug,
    Clone,
    Default,
    strum_macros::Display,
    strum_macros::EnumString,
)]
pub enum GeoBasicType {
    #[default]
    UNKOWN,
    ///正实体
    Pos,
    ///普通负实体
    Neg,
    ///元件库的负实体
    CataNeg,
    ///元件库的需要和design运算的负实体
    CataCrossNeg,
    ///负实体运算过了
    Compound,
    ///属于隐含直段的类型
    Tubi,
}

/// 存储一个Element 包含的所有几何信息
#[derive(
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Default,
    Resource,
)]
#[serde_as]
pub struct EleGeosInfo {
    pub refno: RefnoEnum,
    pub sesno: i32,
    #[serde(default)]
    pub cata_hash: Option<String>,
    //记录对应的元件库参考号
    #[serde(default)]
    #[serde(skip)]
    #[with(rkyv::with::Skip)]
    pub cata_refno: Option<RefnoEnum>,
    //是否可见
    pub visible: bool,
    //所属一般类型，ROOM、STRU、PIPE等, 用枚举处理
    pub generic_type: PdmsGenericType,
    pub aabb: Option<Aabb>,

    //相对世界坐标系下的变换矩阵 rot, translation, scale
    pub world_transform: Transform,

    #[serde(default)]
    pub flow_pt_indexs: Vec<i32>,

    #[serde(skip, default)]
    pub ptset_map: BTreeMap<i32, CateAxisParam>,
    pub has_cata_neg: bool,
    pub is_solid: bool,
    // pub dt: chrono::NaiveDateTime,
}

impl EleGeosInfo {
    ///结合 version 和 refno 生成唯一的id
    #[inline]
    pub fn id_str(&self) -> String {
        let hash = self.cata_hash.clone();
        if hash.is_none()
            || hash.as_ref().unwrap().is_empty()
            || hash.as_ref().unwrap().contains("_")
        {
            format!("{}_{}", self.refno.to_string(), self.sesno)
        } else {
            hash.clone().unwrap()
        }
    }

    ///生成surreal的json文件
    pub fn gen_sur_json(&self, vec3_map: &mut HashMap<u64, String>) -> String {
        let id = self.id_str();
        let mut json = serde_json::to_string_pretty(&serde_json::json!({
            "visible": self.visible,
            "generic_type": self.generic_type,
            "ptset": self.ptset_map,
        }))
        .unwrap();

        json.remove(json.len() - 1);
        json.push_str(",");
        json.push_str(&format!(r#""id": inst_info:⟨{}⟩, "#, id));
        json.push_str("}");
        json
    }

    ///获取几何体数据的string key
    #[inline]
    pub fn get_inst_key(&self) -> String {
        self.id_str()
    }

    #[inline]
    pub fn get_ele_world_transform(&self) -> Transform {
        self.world_transform
    }

    #[inline]
    pub fn get_geo_world_transform(&self, geo: &EleInstGeo) -> Transform {
        let ele_trans = self.get_ele_world_transform();
        if geo.is_tubi {
            geo.transform
        } else {
            ele_trans * geo.transform
        }
    }
}

/// instane数据集合管理
#[derive(
    Serialize,
    Deserialize,
    Debug,
    Default,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Resource,
)]
pub struct ShapeInstancesData {
    /// 保存instance信息数据
    pub inst_info_map: HashMap<RefnoEnum, EleGeosInfo>,
    ///保存所有用到的的tubi数据
    pub inst_tubi_map: HashMap<RefnoEnum, EleGeosInfo>,
    ///保存instance几何数据
    pub inst_geos_map: HashMap<String, EleInstGeosData>,

    ///保存所有用到的的neg连接关系
    #[serde(skip)]
    pub neg_relate_map: HashMap<RefnoEnum, Vec<RefnoEnum>>,

    ///并保存所有ngmr的连接关系
    pub ngmr_neg_relate_map: HashMap<RefnoEnum, Vec<(RefnoEnum, RefnoEnum)>>,
}

/// shape instances 的管理方法
impl ShapeInstancesData {
    pub fn is_empty(&self) -> bool {
        self.inst_info_map.is_empty()
            && self.inst_geos_map.is_empty()
            && self.inst_tubi_map.is_empty()
            && self.neg_relate_map.is_empty()
            && self.ngmr_neg_relate_map.is_empty()
    }

    ///填充基本的形状
    pub fn fill_basic_shapes(&mut self) {
        let unit_cyli_aabb = Aabb::new(Point3::new(-0.5, -0.5, 0.0), Point3::new(0.5, 0.5, 1.0));
        let unit_box_aabb = Aabb::new(Point3::new(-0.5, -0.5, -0.5), Point3::new(0.5, 0.5, 0.5));
        self.insert_geos_data(
            TUBI_GEO_HASH.to_string(),
            EleInstGeosData {
                inst_key: TUBI_GEO_HASH.to_string(),
                refno: Default::default(),
                insts: vec![EleInstGeo {
                    geo_hash: TUBI_GEO_HASH,
                    refno: Default::default(),
                    geo_param: PdmsGeoParam::PrimSCylinder(SCylinder::default()),
                    pts: vec![],
                    aabb: Some(unit_cyli_aabb),
                    transform: Default::default(),
                    visible: true,
                    is_tubi: true,
                    geo_type: GeoBasicType::Tubi,
                    cata_neg_refnos: vec![],
                }],
                aabb: Some(unit_cyli_aabb),
                type_name: "TUBI".to_string(),
            },
        );
        self.insert_geos_data(
            BOXI_GEO_HASH.to_string(),
            EleInstGeosData {
                inst_key: BOXI_GEO_HASH.to_string(),
                refno: Default::default(),
                insts: vec![EleInstGeo {
                    geo_hash: BOXI_GEO_HASH,
                    refno: Default::default(),
                    geo_param: PdmsGeoParam::PrimBox(SBox::default()),
                    pts: vec![],
                    aabb: Some(unit_box_aabb),
                    transform: Default::default(),
                    visible: true,
                    is_tubi: true,
                    geo_type: GeoBasicType::Tubi,
                    cata_neg_refnos: vec![],
                }],
                aabb: Some(unit_box_aabb),
                type_name: "BOXI".to_string(),
            },
        );
    }

    #[inline]
    pub fn clear(&mut self) {
        self.inst_info_map.clear();
        self.inst_geos_map.clear();
        self.inst_tubi_map.clear();
        self.neg_relate_map.clear();
    }

    #[inline]
    pub fn get_show_refnos(&self) -> HashSet<RefnoEnum> {
        let mut ready_refnos: HashSet<RefnoEnum> = self.inst_info_map.keys().cloned().collect();
        ready_refnos.extend(self.inst_tubi_map.keys().cloned());
        ready_refnos
    }

    #[inline]
    pub fn inst_cnt(&self) -> usize {
        self.inst_info_map.len() + self.inst_tubi_map.len()
    }

    pub fn merge_ref(&mut self, o: &Self) {
        for (k, v) in o.inst_info_map.clone() {
            self.insert_info(k, v);
        }
        for (k, v) in o.inst_geos_map.clone() {
            self.insert_geos_data(k, v);
        }
        for (k, v) in o.inst_tubi_map.clone() {
            self.insert_tubi(k, v);
        }
    }

    pub fn merge(&mut self, other: Self) {
        let Self {
            inst_info_map,
            inst_tubi_map,
            inst_geos_map,
            ..
        } = other;
        for (k, v) in inst_info_map {
            self.insert_info(k, v);
        }
        for (k, v) in inst_geos_map {
            self.insert_geos_data(k, v);
        }
        for (k, v) in inst_tubi_map {
            self.insert_tubi(k, v);
        }
    }

    ///获得所有的geo hash值
    #[inline]
    pub fn get_geo_hashs(&self) -> BTreeSet<u64> {
        let mut geo_hashes = BTreeSet::new();
        for g in self.inst_geos_map.values() {
            for inst in &g.insts {
                geo_hashes.insert(inst.geo_hash);
            }
        }
        geo_hashes
    }

    #[inline]
    pub fn get_inst_geos(&self, info: &EleGeosInfo) -> Option<&Vec<EleInstGeo>> {
        let k = info.get_inst_key();
        self.inst_geos_map.get(&k).map(|x| &x.insts)
    }

    #[inline]
    pub fn get_inst_geos_data(&self, info: &EleGeosInfo) -> Option<&EleInstGeosData> {
        let k = info.get_inst_key();
        self.inst_geos_map.get(&k)
    }

    #[inline]
    pub fn get_inst_geos_data_mut_by_refno(
        &mut self,
        refno: RefnoEnum,
    ) -> Option<&mut EleInstGeosData> {
        let info = self.get_inst_info(refno)?;
        self.inst_geos_map.get_mut(&info.get_inst_key())
    }

    #[inline]
    pub fn get_inst_geos_data_mut(&mut self, info: &EleGeosInfo) -> Option<&mut EleInstGeosData> {
        let k = info.get_inst_key();
        self.inst_geos_map.get_mut(&k)
    }

    #[inline]
    pub fn get_inst_tubi(&self, refno: RefnoEnum) -> Option<&EleGeosInfo> {
        self.inst_tubi_map.get(&refno)
    }

    #[inline]
    pub fn contains(&self, refno: &RefnoEnum) -> bool {
        self.inst_info_map.contains_key(refno) || self.inst_tubi_map.contains_key(refno)
    }

    #[inline]
    pub fn get_inst_info(&self, refno: RefnoEnum) -> Option<&EleGeosInfo> {
        self.inst_info_map.get(&refno)
    }

    #[inline]
    pub fn insert_info(&mut self, refno: RefnoEnum, info: EleGeosInfo) {
        self.inst_info_map.insert(refno, info);
    }

    ///插入 ngmr 数据
    #[inline]
    pub fn insert_ngmr(
        &mut self,
        ele_refno: RefnoEnum,
        owners: Vec<RefnoEnum>,
        ngmr_geom_refno: RefnoEnum,
    ) {
        for owner in owners {
            let mut d = self
                .ngmr_neg_relate_map
                .entry(owner)
                .or_insert_with(Vec::new);
            if !d.contains(&(ele_refno, ngmr_geom_refno)) {
                //这里应该是一个组合，所以不会重复，既有design 的参考号，也有元件库几何体的参考号
                d.push((ele_refno, ngmr_geom_refno));
            }
        }
    }

    ///插入neg数据
    #[inline]
    pub fn insert_negs(&mut self, refno: RefnoEnum, negs: &[RefnoEnum]) {
        self.neg_relate_map
            .entry(refno)
            .or_insert_with(Vec::new)
            .extend(negs);
    }

    #[inline]
    pub fn insert_geos_data(&mut self, hash: String, geo: EleInstGeosData) {
        if self.inst_geos_map.contains_key(&hash) {
            self.inst_geos_map
                .get_mut(&hash)
                .unwrap()
                .insts
                .extend_from_slice(&geo.insts);
        } else {
            self.inst_geos_map.insert(hash, geo);
        }
    }

    #[inline]
    pub fn insert_tubi(&mut self, refno: RefnoEnum, info: EleGeosInfo) {
        self.inst_tubi_map.insert(refno, info);
    }

    pub fn get_info(&self, refno: &RefnoEnum) -> Option<&EleGeosInfo> {
        self.inst_info_map.get(refno)
    }

    //serialize_to_bytes
    // pub fn serialize_to_bytes(&self) -> Vec<u8> {
    //     let serialized = rkyv::to_bytes::<_, 512>(self).unwrap().to_vec();
    //     serialized
    // }

    // pub fn serialize_to_specify_file(&self, file_path: &str) -> bool {
    //     let mut file = File::create(file_path).unwrap();
    //     let serialized = rkyv::to_bytes::<_, 512>(self).unwrap().to_vec();
    //     file.write_all(serialized.as_slice()).unwrap();
    //     true
    // }

    // pub fn deserialize_from_bin_file(file_path: &dyn AsRef<Path>) -> anyhow::Result<Self> {
    //     let mut file = File::open(file_path)?;
    //     let mut buf: Vec<u8> = Vec::new();
    //     file.read_to_end(&mut buf).ok();
    //     use rkyv::Deserialize;
    //     use std::io::Read;
    //     let archived = unsafe { rkyv::archived_root::<Self>(buf.as_slice()) };
    //     let r: Self = archived.deserialize(&mut rkyv::Infallible)?;
    //     Ok(r)
    // }

    ///保存compound的edge关系到arango图数据库
    pub async fn save_compound_edges_to_arango() {}
}

//todo mesh 增量传输
#[derive(
    Serialize, Deserialize, Debug, Default, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,
)]
pub struct PdmsInstanceMeshData {
    pub shape_insts: ShapeInstancesData,
}

pub type GeoHash = u64;

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Default,
    Resource,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct PlantGeoData {
    pub geo_hash: u64,
    pub aabb: Option<Aabb>,
}

impl Clone for PlantGeoData {
    fn clone(&self) -> Self {
        Self {
            geo_hash: self.geo_hash.clone(),
            aabb: self.aabb.clone(),
        }
    }
}

impl PlantGeoData {
    ///返回三角模型 （tri_mesh, AABB）
    #[cfg(feature = "render")]
    pub fn gen_bevy_mesh_with_aabb(&self) -> Option<(Mesh, Option<Aabb>)> {
        let mut mesh =
            bevy_render::prelude::Mesh::new(TriangleList, RenderAssetUsages::RENDER_WORLD);
        let d = PlantMesh::default();
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, d.vertices.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, d.normals.clone());
        let n = d.vertices.len();
        let mut uvs = vec![];
        for i in 0..n {
            uvs.push([0.0f32, 0.0]);
        }
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        //todo 是否需要优化索引
        mesh.insert_indices(Indices::U32(d.indices.clone()));

        Some((mesh, self.aabb))
    }

    // pub fn serialize_to_specify_file(&self, file_path: &dyn AsRef<Path>) -> bool {
    //     let mut file = File::create(file_path).unwrap();
    //     let serialized = rkyv::to_bytes::<_, 1024>(self).unwrap().to_vec();
    //     file.write_all(serialized.as_slice()).unwrap();
    //     true
    // }
    //
    // pub fn deserialize_from_bin_file(file_path: &dyn AsRef<Path>) -> anyhow::Result<Self> {
    //     let mut file = File::open(file_path)?;
    //     let mut buf: Vec<u8> = Vec::new();
    //     file.read_to_end(&mut buf).ok();
    //     use rkyv::Deserialize;
    //     let archived = unsafe { rkyv::archived_root::<Self>(buf.as_slice()) };
    //     let r: Self = archived.deserialize(&mut rkyv::Infallible)?;
    //     Ok(r)
    // }
    //
    // pub fn deserialize_from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
    //     use rkyv::Deserialize;
    //     let archived = unsafe { rkyv::archived_root::<Self>(bytes) };
    //     let r: Self = archived.deserialize(&mut rkyv::Infallible)?;
    //     Ok(r)
    // }
}

#[serde_as]
#[derive(
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Serialize,
    Deserialize,
    Clone,
    Debug,
    Default,
    Resource,
)]
pub struct EleInstGeosData {
    pub inst_key: String,
    pub refno: RefnoEnum,
    pub insts: Vec<EleInstGeo>,

    pub aabb: Option<Aabb>,
    pub type_name: String,
}

impl EleInstGeosData {
    #[inline]
    pub fn id(&self) -> String {
        self.inst_key.clone()
    }

    ///生成surreal的json文件
    pub fn gen_sur_json(&self) -> String {
        let mut json_string = serde_json::to_string_pretty(&serde_json::json!({
            "id": self.inst_key.clone(),
            "type_name": self.type_name,
            "aabb": self.aabb,
            "insts": self.insts,
        }))
        .unwrap();

        json_string.remove(json_string.len() - 1);
        json_string.push_str(",");
        json_string.push_str(&format!(r#""refno": pe:{}"#, self.refno.to_string()));
        json_string.push_str("}");
        json_string
    }

    #[inline]
    pub fn has_neg(&self) -> bool {
        self.insts.iter().any(|x| x.geo_type == GeoBasicType::Neg)
    }

    #[inline]
    pub fn has_cata_neg(&self) -> bool {
        self.insts
            .iter()
            .any(|x| x.geo_type == GeoBasicType::CataNeg)
    }

    #[inline]
    pub fn has_ngmr(&self) -> bool {
        self.insts
            .iter()
            .any(|x| x.geo_type == GeoBasicType::CataCrossNeg)
    }
}

///分拆的基本体信息, 应该是不需要复用的
#[derive(
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Serialize,
    Deserialize,
    Clone,
    Debug,
    Default,
    Resource,
)]
#[serde_as]
pub struct EleInstGeo {
    /// 几何hash参数
    pub geo_hash: u64,
    ///对应几何体参考号
    pub refno: RefnoEnum,
    ///几何参数数据
    #[serde(default)]
    pub geo_param: PdmsGeoParam,
    pub pts: Vec<i32>,
    pub aabb: Option<Aabb>,
    //相对于自身的坐标系变换
    #[serde(default)]
    pub transform: Transform,
    pub visible: bool,
    pub is_tubi: bool,
    #[serde(default)]
    pub geo_type: GeoBasicType,

    //元件库里的负实体
    #[serde(default)]
    pub cata_neg_refnos: Vec<RefnoEnum>,
}

impl EleInstGeo {
    #[inline]
    pub fn is_cata_neg(&self) -> bool {
        self.geo_type == GeoBasicType::CataNeg
    }

    #[inline]
    pub fn is_neg(&self) -> bool {
        self.geo_type == GeoBasicType::Neg
    }

    #[inline]
    pub fn key_points(&self) -> Vec<Vec3> {
        self.geo_param
            .key_points()
            .into_iter()
            .map(|v| self.transform.transform_point(*v))
            .collect()
    }

    ///fix 生成surreal的geo json数据，其他数据放在边上
    pub fn gen_unit_geo_sur_json(&self) -> String {
        let mut json_string = "".to_string();
        let param = self.geo_param.convert_to_unit_param();
        json_string.push_str(&format!(
            "{{'id': inst_geo:⟨{}⟩, 'param': {} }}",
            self.geo_hash,
            /* gen_bytes_hash::<_, 64>(&self.aabb),*/
            serde_json::to_string(&param).unwrap()
        ));
        json_string
    }

    #[cfg(feature = "occ")]
    pub fn gen_occ_shape(&self) -> anyhow::Result<OccSharedShape> {
        let mut shape: OccSharedShape = self.geo_param.gen_occ_shape()?;
        //scale 不能要，已经包含在OCC的真实参数里
        let mut new_transform = self.transform;
        new_transform.scale = Vec3::ONE;
        shape
            .as_mut()
            .transform_by_mat(&new_transform.compute_matrix().as_dmat4());
        Ok(shape)
    }
}
