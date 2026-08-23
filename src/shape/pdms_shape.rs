use anyhow::anyhow;
use bevy_ecs::component::Component;
#[cfg(feature = "render")]
use bevy_render::prelude::*;
#[cfg(feature = "render")]
use bevy_render::render_asset::RenderAssetUsages;
use bevy_transform::prelude::Transform;
use derive_more::{Deref, DerefMut};
use downcast_rs::*;
use dyn_clone::DynClone;
use glam::{DMat4, DVec3};
use glam::{Mat4, Vec3, Vec4};
use itertools::Itertools;
use parry3d::bounding_volume::Aabb;
use parry3d::math::{Point, Vector};
use parry3d::shape::{TriMesh, TriMeshFlags};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::fs::File;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::BufWriter;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::vec;
#[cfg(feature = "truck")]
use truck_base::bounding_box::BoundingBox;
#[cfg(feature = "truck")]
use truck_base::cgmath64::{Matrix4, Point3, Vector3, Vector4};
#[cfg(feature = "truck")]
use truck_meshalgo::prelude::{MeshableShape, MeshedShape};
#[cfg(feature = "truck")]
use truck_modeling::{Curve, Shell};

use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::tool::float_tool::f32_round_3;
use parry3d::bounding_volume::BoundingVolume;

use crate::geometry::PlantGeoData;
pub trait VerifiedShape {
    fn check_valid(&self) -> bool {
        true
    }
}

#[cfg(feature = "truck")]
#[inline]
pub fn gen_bounding_box(shell: &Shell) -> BoundingBox<Point3> {
    let mut bdd_box = BoundingBox::new();
    shell
        .iter()
        .flat_map(truck_modeling::Face::boundaries)
        .flatten()
        .for_each(|edge| {
            let curve = edge.oriented_curve();
            bdd_box += match curve {
                Curve::Line(line) => vec![line.0, line.1].into_iter().collect(),
                Curve::BSplineCurve(curve) => {
                    let bdb = curve.roughly_bounding_box();
                    vec![bdb.max(), bdb.min()].into_iter().collect()
                }
                Curve::NurbsCurve(curve) => curve.roughly_bounding_box(),
                Curve::IntersectionCurve(_) => BoundingBox::new(),
            };
        });
    bdd_box
}

//todo 增加LOD的实现
#[derive(
    Serialize,
    Deserialize,
    Component,
    Debug,
    Default,
    Clone,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct PlantMesh {
    pub indices: Vec<u32>,
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub wire_vertices: Vec<Vec<Vec3>>,
    pub aabb: Option<Aabb>,
}

impl PlantMesh {
    ///合并两个mesh
    pub fn merge(&mut self, other: &Self) {
        let vertex_offset = self.vertices.len() as u32;
        self.indices
            .extend(other.indices.iter().map(|&i| i + vertex_offset));
        self.vertices.extend(other.vertices.iter());
        self.normals.extend(other.normals.iter());
        // self.wire_vertices.extend(other.wire_vertices.iter());

        // Merge aabb if present
        if let Some(other_aabb) = &other.aabb {
            if let Some(self_aabb) = &mut self.aabb {
                *self_aabb = self_aabb.merged(other_aabb);
            } else {
                self.aabb = Some(*other_aabb);
            }
        }
    }
}

impl PlantMesh {

    ///生成tri mesh
    #[inline]
    pub fn get_tri_mesh(&self, trans: Mat4) -> Option<TriMesh> {
        self.get_tri_mesh_with_flag(trans, TriMeshFlags::default())
    }

    ///生成带flag的tri mesh
    #[inline]
    pub fn get_tri_mesh_with_flag(&self, trans: Mat4, flag: TriMeshFlags) -> Option<TriMesh> {
        if self.indices.len() < 3 {
            return None;
        }
        let mut points: Vec<Point<f32>> = vec![];
        let mut indices: Vec<[u32; 3]> = vec![];
        //如果 数量太大，需要使用LOD的模型去做碰撞检测
        self.vertices.iter().for_each(|p| {
            let new_pt = trans.transform_point3(*p);
            points.push(new_pt.into())
        });
        // dbg!(&self.indices);
        self.indices.chunks(3).for_each(|i| {
            indices.push([i[0] as u32, i[1] as u32, i[2] as u32]);
        });
        let mut tri_mesh = TriMesh::with_flags(points, indices, flag);
        Some(tri_mesh)
    }

    ///计算aabb
    pub fn cal_aabb(&self) -> Option<Aabb> {
        let mut aabb = Aabb::new_invalid();
        self.vertices.iter().for_each(|v| {
            aabb.take_point(nalgebra::Point3::new(v.x, v.y, v.z));
        });
        if Vec3::from(aabb.mins).is_nan() || Vec3::from(aabb.maxs).is_nan() {
            return None;
        }
        Some(aabb)
    }

    ///计算法线
    pub fn cal_normals(&mut self) {
        for (_i, c) in self.indices.chunks(3).enumerate() {
            let a: Vec3 = self.vertices[c[0] as usize];
            let b: Vec3 = self.vertices[c[1] as usize];
            let c: Vec3 = self.vertices[c[2] as usize];

            let normal = ((b - a).cross(c - a)).normalize();
            self.normals.push(normal);
            self.normals.push(normal);
            self.normals.push(normal);
        }
    }

    ///todo 后面需要把uv使用上
    #[cfg(feature = "render")]
    pub fn gen_bevy_mesh(&self) -> Mesh {
        use bevy_render::mesh::Indices;
        use bevy_render::render_resource::PrimitiveTopology::TriangleList;

        let mut mesh = Mesh::new(TriangleList, RenderAssetUsages::default());
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.vertices.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals.clone());
        // mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_indices(Indices::U32(self.indices.clone()));
        mesh
    }

    ///变换mesh
    pub fn transform_by(&self, t: &DMat4) -> Self {
        let mut vertices = Vec::with_capacity(self.vertices.len());
        let mut normals = Vec::with_capacity(self.vertices.len());
        let len = self.vertices.len();
        for i in 0..len {
            vertices.push(t.transform_point3(self.vertices[i].as_dvec3()).as_vec3());
            if i < self.normals.len() {
                normals.push(
                    t.transform_vector3(self.normals[i].as_dvec3())
                        .normalize()
                        .as_vec3(),
                );
            }
        }
        Self {
            indices: self.indices.clone(),
            vertices,
            normals,
            wire_vertices: vec![],
            aabb: None,
        }
    }

    ///缩放mesh
    pub fn scale_by(&mut self, scale: f32) {
        self.vertices.iter_mut().for_each(|v| {
            *v *= scale;
        });
    }

    ///序列化
    #[inline]
    pub fn ser_to_bytes(&self) -> Vec<u8> {
        rkyv::to_bytes::<_, 1024>(self).unwrap().to_vec()
    }

    ///序列化到文件
    #[inline]
    pub fn ser_to_file(&self, file_path: &dyn AsRef<Path>) -> anyhow::Result<()> {
        let bytes = rkyv::to_bytes::<_, 1024>(self).unwrap().to_vec();
        let mut file = File::create(file_path).unwrap();
        file.write_all(&bytes)?;
        Ok(())
    }

    ///从文件反序列化
    pub fn des_mesh_file(file_path: &dyn AsRef<Path>) -> anyhow::Result<Self> {
        let mut file = File::open(file_path)?;
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).ok();
        use rkyv::Deserialize;
        let archived = unsafe { rkyv::archived_root::<Self>(buf.as_slice()) };
        let r: Self = archived.deserialize(&mut rkyv::Infallible)?;
        Ok(r)
    }

    ///从bytes反序列化
    pub fn des_from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        use rkyv::Deserialize;
        let archived = unsafe { rkyv::archived_root::<Self>(bytes) };
        let r: Self = archived.deserialize(&mut rkyv::Infallible)?;
        Ok(r)
    }

    ///压缩bytes
    #[inline]
    pub fn into_compress_bytes(&self) -> Vec<u8> {
        use flate2::write::DeflateEncoder;
        use flate2::Compression;
        let mut e = DeflateEncoder::new(Vec::new(), Compression::default());
        e.write_all(&bincode::serialize(&self).unwrap());
        e.finish().unwrap_or_default()
    }

    ///从压缩bytes反序列化
    #[inline]
    pub fn from_compress_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        use flate2::write::DeflateDecoder;
        let writer = Vec::new();
        let mut deflater = DeflateDecoder::new(writer);
        deflater.write_all(bytes)?;
        Ok(bincode::deserialize(&deflater.finish()?)?)
    }

    ///导出obj
    pub fn export_obj(&self, reverse: bool, file_path: &str) -> std::io::Result<()> {
        let mut buffer = BufWriter::new(File::create(file_path)?);
        buffer.write_all(b"# List of geometric vertices, with (x, y, z [,w]) coordinates, w is optional and defaults to 1.0.\n")?;
        for (vd, n) in self.vertices.iter().zip(self.normals.iter()) {
            buffer.write_all(
                format!(
                    "v {:.3} {:.3} {:.3}\n",
                    vd[0] as f32, vd[1] as f32, vd[2] as f32
                )
                .as_ref(),
            )?;
            buffer.write_all(
                format!(
                    "vn {:.3} {:.3} {:.3}\n",
                    n[0] as f32, n[1] as f32, n[2] as f32
                )
                .as_ref(),
            )?;
        }
        buffer.write_all(b"# Polygonal face element\n")?;
        for id in self.indices.chunks(3) {
            if reverse {
                buffer.write_all(
                    format!("f {} {} {}\n", id[2] + 1, id[1] + 1, id[0] + 1,).as_ref(),
                )?;
            } else {
                buffer.write_all(
                    format!("f {} {} {}\n", id[0] + 1, id[1] + 1, id[2] + 1,).as_ref(),
                )?;
            }
        }

        buffer.flush()?;
        Ok(())
    }
}

/// 三角形容差
pub const TRI_TOL: f32 = 0.001;
/// 长度容差
pub const LEN_TOL: f32 = 0.001;
/// 角度容差(弧度)
pub const ANGLE_RAD_TOL: f32 = 0.001;
/// 角度容差(弧度,f64)
pub const ANGLE_RAD_F64_TOL: f64 = 0.001;
/// 最小尺寸容差
pub const MIN_SIZE_TOL: f32 = 0.01;
/// 最大尺寸容差
pub const MAX_SIZE_TOL: f32 = 1.0e5;
/// 为BrepShapeTrait实现Clone特征
dyn_clone::clone_trait_object!(BrepShapeTrait);

#[derive(
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Serialize,
    Deserialize,
    Deref,
    DerefMut,
    Clone,
    Default,
    Debug,
)]
pub struct RsVec3(pub Vec3);

impl RsVec3 {
    pub fn gen_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::default();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

impl Hash for RsVec3 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        format!("{:.3}", self.x).hash(state);
        format!("{:.3}", self.y).hash(state);
        format!("{:.3}", self.z).hash(state);
    }
}

impl PartialEq<Self> for RsVec3 {
    fn eq(&self, other: &Self) -> bool {
        self.distance(other.0) < 1.0E-5
    }
}

impl Eq for RsVec3 {}

impl From<Vec3> for RsVec3 {
    fn from(value: Vec3) -> Self {
        Self(value)
    }
}

/// 将truck库的Point3类型转换为RsVec3类型
///
/// 当启用truck特性时，实现从Point3到RsVec3的转换
/// 将Point3的x,y,z坐标转换为f32类型并构造Vec3
#[cfg(feature = "truck")]
impl From<Point3> for RsVec3 {
    fn from(value: Point3) -> Self {
        Self(Vec3::new(value.x as f32, value.y as f32, value.z as f32))
    }
}

///brep形状trait
pub trait BrepShapeTrait: Downcast + VerifiedShape + Debug + Send + Sync + DynClone {
    fn is_reuse_unit(&self) -> bool {
        false
    }

    //拷贝函数
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait>;

    #[cfg(feature = "truck")]
    ///生成shell
    fn gen_brep_shell(&self) -> Option<Shell> {
        return None;
    }

    ///获得关键点
    fn key_points(&self) -> Vec<RsVec3> {
        #[cfg(feature = "truck")]
        {
            return self
                .gen_unit_shape()
                .gen_brep_shell()
                .map(|x| {
                    x.vertex_iter()
                        .map(|x| RsVec3::from(x.point()))
                        .into_iter()
                        .unique()
                        .collect()
                })
                .unwrap_or(vec![Vec3::ZERO.into()]);
        }
        {
            Default::default()
        }
    }

    ///限制参数大小，主要是对负实体的不合理进行限制
    fn apply_limit_by_size(&mut self, _limit_size: f32) {}

    //计算单元模型的参数hash值，也就是做成被可以复用的模型后的hash
    fn hash_unit_mesh_params(&self) -> u64 {
        0
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait>;

    ///生成对应的单位长度的模型，比如Dish，就是以R为1的情况生成模型
    /// box
    /// cylinder
    /// sphere
    #[cfg(not(target_arch = "wasm32"))]
    fn gen_unit(&self, tol_ratio: Option<f32>) -> anyhow::Result<PlantGeoData> {
        // self.gen_unit_shape().gen_plant_geo_data(tol_ratio)
        todo!("not support")
    }

    #[cfg(target_arch = "wasm32")]
    fn gen_unit(&self, tol_ratio: Option<f32>) -> anyhow::Result<PlantGeoData> {
        todo!("wasm32 not support")
    }

    ///获得缩放向量
    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        Vec3::ONE
    }

    ///获得变换矩阵
    #[inline]
    fn get_trans(&self) -> Transform {
        Transform {
            rotation: Default::default(),
            translation: Default::default(),
            scale: self.get_scaled_vec3(),
        }
    }

    #[inline]
    fn tol(&self) -> f32 {
        TRI_TOL
    }

    ///生成mesh
    #[cfg(feature = "truck")]
    fn gen_plant_geo_data_truck(&self, tol_ratio: Option<f32>) -> Option<PlantGeoData> {
        let mut aabb = Aabb::new_invalid();
        let geo_hash = self.hash_unit_mesh_params();
        if self.need_use_csg() {
            if let Some(csg_mesh) = self.gen_csg_mesh() {
                for vertex in &csg_mesh.vertices {
                    aabb.take_point((*vertex).into());
                }
                return Some(PlantGeoData {
                    geo_hash,
                    mesh: Some(csg_mesh),
                    aabb: Some(aabb),
                });
            }
            return None;
        }
        if let Some(brep) = self.gen_brep_shell() {
            let brep_bbox = gen_bounding_box(&brep);
            let d = brep_bbox.diameter() as f32;
            if d < MIN_SIZE_TOL || d > MAX_SIZE_TOL {
                return None;
            }
            let (_size, c) = (brep_bbox.diameter(), brep_bbox.center());
            let d = brep_bbox.diagonal() / 2.0;
            aabb = Aabb::from_half_extents(
                Point::<f32>::new(c[0] as f32, c[1] as f32, c[2] as f32),
                Vector::<f32>::new(d[0] as f32, d[1] as f32, d[2] as f32),
            );
            let tolerance = self.tol() as f64 * tol_ratio.unwrap_or(2.0) as f64;
            let meshed_shape = brep.triangulation(tolerance);
            let mut polygon_mesh = meshed_shape.to_polygon();
            if polygon_mesh.positions().is_empty() {
                return None;
            }
            let vertices = polygon_mesh
                .positions()
                .iter()
                .map(|&x| x.vec3())
                .collect::<Vec<_>>();
            let normals = polygon_mesh
                .normals()
                .iter()
                .map(|&x| x.vec3())
                .collect::<Vec<_>>();
            let _uvs = polygon_mesh
                .uv_coords()
                .iter()
                .map(|x| [x[0] as f32, x[1] as f32])
                .collect::<Vec<_>>();
            let indices = polygon_mesh
                .faces()
                .triangle_iter()
                .flatten()
                .map(|x| x.pos as u32)
                .collect::<Vec<_>>();

            let curves = meshed_shape
                .edge_iter()
                .map(|edge| edge.curve())
                .collect::<Vec<_>>();
            let wire_vertices: Vec<Vec<Vec3>> = curves
                .iter()
                .map(|poly| poly.iter().map(|x| x.vec3()).collect::<Vec<_>>())
                .collect();

            return Some(PlantGeoData {
                geo_hash,
                mesh: Some(PlantMesh {
                    indices,
                    vertices,
                    normals,
                    wire_vertices,
                }),
                aabb: Some(aabb),
            });
        }
        None
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        None
    }

    fn gen_csg_mesh(&self) -> Option<PlantMesh> {
        None
    }

    fn need_use_csg(&self) -> bool {
        false
    }
}

impl_downcast!(BrepShapeTrait);

#[cfg(feature = "truck")]
pub trait BrepMathTrait {
    fn vector3(&self) -> Vector3;
    fn vector4(&self) -> Vector4;
    fn point3(&self) -> Point3;
    fn point3_without_z(&self) -> Point3;
}

#[cfg(feature = "truck")]
impl BrepMathTrait for Vec3 {
    #[inline]
    fn vector3(&self) -> Vector3 {
        Vector3::new(self[0] as f64, self[1] as f64, self[2] as f64)
    }

    #[inline]
    fn vector4(&self) -> Vector4 {
        Vector4::new(self[0] as f64, self[1] as f64, self[2] as f64, 0.0f64)
    }

    #[inline]
    fn point3(&self) -> Point3 {
        Point3::new(self[0] as f64, self[1] as f64, self[2] as f64)
    }

    #[inline]
    fn point3_without_z(&self) -> Point3 {
        Point3::new(
            f32_round_3(self[0]) as f64,
            f32_round_3(self[1]) as f64,
            0.0 as f64,
        )
    }
}

#[cfg(feature = "truck")]
impl BrepMathTrait for Vec4 {
    #[inline]
    fn vector3(&self) -> Vector3 {
        Vector3::new(self[0] as f64, self[1] as f64, self[2] as f64)
    }

    #[inline]
    fn vector4(&self) -> Vector4 {
        Vector4::new(
            self[0] as f64,
            self[1] as f64,
            self[2] as f64,
            self[3] as f64,
        )
    }

    #[inline]
    fn point3(&self) -> Point3 {
        Point3::new(self[0] as f64, self[1] as f64, self[2] as f64)
    }

    #[inline]
    fn point3_without_z(&self) -> Point3 {
        Point3::new(self[0] as f64, self[1] as f64, 0.0 as f64)
    }
}

pub trait BevyMathTrait {
    fn vec3(&self) -> Vec3;
    fn array(&self) -> [f32; 3];
}

#[cfg(feature = "truck")]
impl BevyMathTrait for Vector3 {
    #[inline]
    fn vec3(&self) -> Vec3 {
        Vec3::new(self[0] as f32, self[1] as f32, self[2] as f32)
    }

    #[inline]
    fn array(&self) -> [f32; 3] {
        [self[0] as f32, self[1] as f32, self[2] as f32]
    }
}

#[cfg(feature = "truck")]
impl BevyMathTrait for Point3 {
    #[inline]
    fn vec3(&self) -> Vec3 {
        Vec3::new(self[0] as f32, self[1] as f32, self[2] as f32)
    }

    #[inline]
    fn array(&self) -> [f32; 3] {
        [self[0] as f32, self[1] as f32, self[2] as f32]
    }
}

#[cfg(feature = "truck")]
#[inline]
pub fn convert_to_cg_matrix4(m: &Mat4) -> Matrix4 {
    Matrix4::from_cols(
        m.x_axis.vector4(),
        m.y_axis.vector4(),
        m.z_axis.vector4(),
        m.w_axis.vector4(),
    )
}
