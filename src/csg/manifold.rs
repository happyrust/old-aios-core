use std::alloc::{Layout, alloc};
use std::{mem, panic};

use crate::shape::pdms_shape::PlantMesh;
use crate::tool::float_tool::*;
use derive_more::{Deref, DerefMut};
use glam::{DMat4, Mat4, Vec2, Vec3};
use itertools::Itertools;
use manifold_sys::bindings::*;
use parry3d::bounding_volume::Aabb;

#[derive(Clone, Deref, DerefMut)]
pub struct ManifoldSimplePolygonRust {
    pub ptr: *mut ManifoldSimplePolygon,
}

impl ManifoldSimplePolygonRust {
    pub fn new() -> Self {
        unsafe {
            let sz = manifold_simple_polygon_size();
            let layout = Layout::from_size_align(sz, 32).unwrap();
            Self {
                ptr: alloc(layout) as _,
            }
        }
    }

    ///根据2d的点生成
    pub fn from_points(pts: &[Vec2]) -> Self {
        unsafe {
            let mut polygon = Self::new();
            let ptr = manifold_simple_polygon(polygon.ptr as _, pts.as_ptr() as _, pts.len() as _);
            Self { ptr }
        }
    }
}

#[derive(Clone, Deref, DerefMut)]
pub struct ManifoldCrossSectionRust {
    pub ptr: *mut ManifoldCrossSection,
}

impl ManifoldCrossSectionRust {
    pub fn new() -> Self {
        unsafe {
            let sz = manifold_cross_section_size();
            let layout = Layout::from_size_align(sz, 32).unwrap();
            Self {
                ptr: alloc(layout) as _,
            }
        }
    }

    ///根据2d的点生成 ManifoldCrossSectionRust
    pub fn from_points(pts: &[Vec2]) -> Self {
        unsafe {
            let mut cross_section = Self::new();
            let mut polygon = ManifoldSimplePolygonRust::from_points(pts);
            manifold_cross_section_of_simple_polygon(
                cross_section.ptr as _,
                polygon.ptr as _,
                ManifoldFillRule_MANIFOLD_FILL_RULE_POSITIVE,
            );
            cross_section
        }
    }

    ///拉伸成manifold
    pub fn extrude(&mut self, height: f32, slices: u32) -> ManifoldRust {
        unsafe {
            let mut manifold = ManifoldRust::new();
            manifold_extrude(
                manifold.ptr as _,
                self.ptr as _,
                height,
                slices as _,
                0.0,
                1.0,
                1.0,
            );

            manifold
        }
    }

    //旋转成manifold
    pub fn extrude_rotate(&mut self, segments: i32, angle: f32) -> ManifoldRust {
        unsafe {
            let mut manifold = ManifoldRust::new();
            manifold_revolve(manifold.ptr as _, self.ptr as _, segments, angle);
            manifold
        }
    }
}

#[derive(Clone, Deref, DerefMut)]
pub struct ManifoldRust {
    pub ptr: *mut ManifoldManifold,
}

unsafe impl Send for ManifoldRust {}

impl ManifoldRust {
    pub fn new() -> Self {
        unsafe {
            let sz = manifold_manifold_size();
            let layout = Layout::from_size_align(sz, 32).unwrap();
            let ptr = manifold_empty(alloc(layout) as _);
            Self { ptr }
        }
    }

    pub fn convert_to_manifold(plant_mesh: PlantMesh, mat4: DMat4, more_precsion: bool) -> Self {
        Self::from_mesh(&ManifoldMeshRust::convert_to_manifold_mesh(
            plant_mesh,
            mat4,
            more_precsion,
        ))
    }

    pub fn from_mesh(m: &ManifoldMeshRust) -> Self {
        unsafe {
            let mut manifold = Self::new();
            let ptr = manifold_of_meshgl(manifold.ptr as _, m.ptr);
            manifold
        }
    }

    pub fn get_mesh(&self) -> ManifoldMeshRust {
        unsafe {
            let mesh = ManifoldMeshRust::new();
            manifold_get_meshgl(mesh.ptr as _, self.ptr);
            mesh
        }
    }

    pub fn num_tri(&self) -> u32 {
        unsafe { manifold_num_tri(self.ptr) as _ }
    }

    pub fn get_properties(&self) -> ManifoldProperties {
        unsafe { manifold_get_properties(self.ptr) }
    }

    ///不支持subtact
    pub fn batch_boolean(batch: &[Self], op: ManifoldOpType) -> Self {
        unsafe {
            let sz = manifold_manifold_vec_size();
            let layout = Layout::from_size_align(sz, 32).unwrap();
            let m_vec = manifold_manifold_vec(alloc(layout) as _, batch.len());
            for b in batch {
                manifold_manifold_vec_push_back(m_vec, b.ptr);
            }
            let mut result = Self::new();
            manifold_batch_boolean(result.ptr as _, m_vec, op);
            //todo how to release memory
            // manifold_delete_manifold_vec(m_vec);
            result
        }
    }

    pub fn batch_boolean_subtract(&self, negs: &[Self]) -> Self {
        unsafe {
            let mut result = Self::new();
            if negs.len() == 0 {
                return self.clone();
            }
            let mut src = self.clone();
            for (i, b) in negs.iter().enumerate() {
                manifold_difference(result.ptr as _, src.ptr, b.ptr);
                src.ptr = result.ptr;
            }
            manifold_as_original(result.ptr as _, src.ptr);
            result
        }
    }

    pub fn destroy(&self) {
        unsafe {
            manifold_delete_manifold(self.ptr);
        }
    }
}

#[derive(Clone, Deref, DerefMut)]
pub struct ManifoldMeshRust {
    pub ptr: *mut ManifoldMeshGL,
}

impl ManifoldMeshRust {
    pub fn new() -> Self {
        unsafe {
            let sz = manifold_meshgl_size();
            let layout = Layout::from_size_align(sz, 32).unwrap();
            Self {
                ptr: alloc(layout) as _,
            }
        }
    }
    pub fn num_tri(&self) -> u32 {
        unsafe { manifold_meshgl_num_tri(self.ptr) as _ }
    }

    pub fn merge(&mut self) -> bool {
        unsafe {
            // manifold_meshgl_merge(self.ptr) != 0
            true
        }
    }

    pub fn convert_to_manifold_mesh(
        mut plant_mesh: PlantMesh,
        mat4: DMat4,
        ceil_or_trunc: bool,
    ) -> Self {
        unsafe {
            let mesh = ManifoldMeshRust::new();
            let len = plant_mesh.vertices.len();
            let mut verts: Vec<f32> = Vec::with_capacity(len * 3);
            for v in mem::take(&mut plant_mesh.vertices) {
                let pt = mat4.transform_point3(glam::DVec3::from(v));
                if ceil_or_trunc {
                    verts.push((num_traits::signum(pt[0]) * f64_round_2(pt[0].abs())) as f32);
                    verts.push((num_traits::signum(pt[1]) * f64_round_2(pt[1].abs())) as f32);
                    verts.push((num_traits::signum(pt[2]) * f64_round_2(pt[2].abs())) as f32);
                } else {
                    verts.push(f64_trunc_1(pt[0]) as _);
                    verts.push(f64_trunc_1(pt[1]) as _);
                    verts.push(f64_trunc_1(pt[2]) as _);
                }
            }
            manifold_meshgl(
                mesh.ptr as _,
                verts.as_ptr() as _,
                len,
                3,
                plant_mesh.indices.as_ptr() as _,
                plant_mesh.indices.len() / 3,
            );
            mesh
        }
    }
}
//负实体的模型应该更大一些
//正实体的模型更小一些

// impl From<(&PlantMesh, &DMat4)> for ManifoldMeshRust {
//     fn from(c: (&PlantMesh, &DMat4)) -> Self {
//         let m = c.0;
//         let t = c.1;
//         unsafe {
//             let mesh = ManifoldMeshRust::new();
//             let mut verts: Vec<f32> = Vec::with_capacity(m.vertices.len() * 3);
//             for v in m.vertices.clone() {
//                 let pt = t.transform_point3(glam::DVec3::from(v));
//                 // verts.push(f64_round_3(pt[0]) as _);
//                 // verts.push(f64_round_3(pt[1]) as _);
//                 // verts.push(f64_round_3(pt[2]) as _);
//
//                 verts.push(f64_round_1(pt[0]) as _);
//                 verts.push(f64_round_1(pt[1]) as _);
//                 verts.push(f64_round_1(pt[2]) as _);
//             }
//             manifold_meshgl(mesh.ptr as _,
//                             verts.as_ptr() as _, m.vertices.len(), 3,
//                             m.indices.as_ptr() as _, m.indices.len() / 3);
//             mesh
//         }
//     }
// }

impl From<&PlantMesh> for ManifoldMeshRust {
    fn from(m: &PlantMesh) -> Self {
        unsafe {
            let mesh = ManifoldMeshRust::new();
            let mut verts = Vec::with_capacity(m.vertices.len() * 3);
            //todo 是否要根据包围盒的大小来判断用哪个等级的round
            for v in m.vertices.clone() {
                // verts.push(f32_round_3(v[0]));
                // verts.push(f32_round_3(v[1]));
                // verts.push(f32_round_3(v[2]));

                verts.push(f32_round_1(v[0]));
                verts.push(f32_round_1(v[1]));
                verts.push(f32_round_1(v[2]));
            }
            manifold_meshgl(
                mesh.ptr as _,
                verts.as_ptr() as _,
                m.vertices.len(),
                3,
                m.indices.as_ptr() as _,
                m.indices.len() / 3,
            );
            mesh
        }
    }
}

impl From<&PlantMesh> for ManifoldRust {
    fn from(m: &PlantMesh) -> Self {
        let mesh: ManifoldMeshRust = m.into();
        Self::from_mesh(&mesh)
    }
}

impl From<PlantMesh> for ManifoldRust {
    fn from(m: PlantMesh) -> Self {
        let mesh: ManifoldMeshRust = (&m).into();
        Self::from_mesh(&mesh)
    }
}

// impl From<(&PlantMesh, &DMat4)> for ManifoldRust {
//     fn from(m: (&PlantMesh, &DMat4)) -> Self {
//         unsafe {
//             let mesh: ManifoldMeshRust = m.into();
//             Self::from_mesh(&mesh)
//         }
//     }
// }
// impl From<ManifoldRust> for PlantMesh {
//     fn from(m: ManifoldRust) -> Self {
//         (&m).into()
//     }
// }

impl From<&ManifoldRust> for PlantMesh {
    fn from(m: &ManifoldRust) -> Self {
        unsafe {
            let mesh = ManifoldMeshRust::new();
            let mut aabb = Aabb::new_invalid();
            manifold_get_meshgl(mesh.ptr as _, m.ptr);
            let len = manifold_meshgl_tri_length(mesh.ptr as _);
            if len == 0 {
                return Self::default();
            }
            // dbg!(len);

            let prop_num = manifold_meshgl_num_prop(mesh.ptr) as usize;
            // dbg!(prop_num);
            let vert_num = manifold_meshgl_num_vert(mesh.ptr) as usize;
            // dbg!(vert_num);
            let tri_num = manifold_meshgl_num_tri(mesh.ptr) as usize;
            // dbg!(tri_num);

            let mut p: Vec<f32> = Vec::with_capacity(vert_num * prop_num);
            p.resize(vert_num * prop_num, 0.0);
            let mut old_indices: Vec<u32> = Vec::with_capacity(tri_num * 3);
            old_indices.resize(tri_num * 3, 0);

            let mut vert = Vec::with_capacity(vert_num);
            manifold_meshgl_vert_properties(p.as_mut_ptr() as _, mesh.ptr);
            manifold_meshgl_tri_verts(old_indices.as_mut_ptr() as _, mesh.ptr);

            for i in 0..vert_num {
                vert.push([
                    p[prop_num * i + 0],
                    p[prop_num * i + 1],
                    p[prop_num * i + 2],
                ]);
            }

            let index_num = tri_num * 3;
            let mut indices = Vec::with_capacity(index_num);
            let mut normals = Vec::with_capacity(index_num);
            let mut vertices = Vec::with_capacity(index_num);

            for (i, c) in old_indices.chunks(3).enumerate() {
                let a: Vec3 = Vec3::from(vert[c[0] as usize].clone());
                let b: Vec3 = Vec3::from(vert[c[1] as usize].clone());
                let c: Vec3 = Vec3::from(vert[c[2] as usize].clone());

                let normal = ((b - a).cross(c - a)).normalize();

                vertices.push(a.into());
                vertices.push(b.into());
                vertices.push(c.into());

                normals.push(normal);
                normals.push(normal);
                normals.push(normal);
                let i = i as u32;
                indices.push(i * 3 + 0);
                indices.push(i * 3 + 1);
                indices.push(i * 3 + 2);
            }

            // m.destroy();

            Self {
                indices,
                vertices,
                normals,
                wire_vertices: vec![],
                aabb: None,
            }
        }
    }
}
