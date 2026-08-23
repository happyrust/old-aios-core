#[cfg(all(feature = "gen_model", feature = "manifold"))]
use crate::csg::manifold::*;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::prim_geo::wire::*;
#[cfg(feature = "truck")]
use crate::shape::pdms_shape::BrepMathTrait;
use crate::shape::pdms_shape::{BrepShapeTrait, PlantMesh, RsVec3, TRI_TOL, VerifiedShape};
use crate::tool::float_tool::{f32_round_3, hash_f32, hash_vec3};
use approx::AbsDiffEq;
use approx::abs_diff_eq;
use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
#[cfg(feature = "truck")]
use truck_meshalgo::prelude::*;
#[cfg(feature = "truck")]
use truck_modeling::{Surface, builder};
#[cfg(feature = "truck")]
use truck_stepio::out;

#[derive(
    Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,
)]
pub struct Revolution {
    pub verts: Vec<Vec<Vec3>>,
    pub angle: f32,
    pub rot_dir: Vec3,
    pub rot_pt: Vec3,
}

impl Default for Revolution {
    fn default() -> Self {
        Self {
            verts: vec![],
            angle: 90.0,
            rot_dir: Vec3::X,   //默认绕X轴旋转
            rot_pt: Vec3::ZERO, //默认旋转点
        }
    }
}

impl VerifiedShape for Revolution {
    fn check_valid(&self) -> bool {
        //add some other restrictions
        true
        // self.angle.abs() > std::f32::EPSILON
    }
}

impl BrepShapeTrait for Revolution {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    ///revolve 有些问题，暂时用manifold来代替
    ///如果是沿自己的一条边旋转，需要弄清楚为啥三角化出来的不对
    #[cfg(feature = "truck")]
    fn gen_brep_shell(&self) -> Option<truck_modeling::Shell> {
        if !self.check_valid() {
            return None;
        }
        let wire = gen_wire(&self.verts, &self.fradius_vec).unwrap();

        //如果截面包含了原点，就考虑用分成两块的办法
        // let contains_origin = polygon.contains(&point!{ x: 0.0, y: 0.0 });
        if let Ok(mut face) = builder::try_attach_plane(&[wire]) {
            if let Surface::Plane(plane) = face.surface() {
                let mut rot_dir = self.rot_dir.normalize().vector3();
                let rot_pt = self.rot_pt.point3();
                //避免精度的误差
                let mut angle = (f32_round_3(self.angle) as f64).to_radians();
                let mut axis_reversed = false;
                if angle < 0.0 {
                    angle = -angle;
                    rot_dir = -rot_dir;
                    axis_reversed = true;
                }
                let z_flag = plane.normal().z > 0.0;
                // //如果两者一致，就不需要reverse
                // if z_flag && axis_reversed {
                face = face.inverse();
                // }

                //check if exist any point on axis
                let axis_on_edge = self.verts.iter().any(|x| {
                    x.y.abs().abs_diff_eq(&0.0, 0.01) && x.z.abs().abs_diff_eq(&0.0, 0.01)
                });
                {
                    let s = builder::rsweep(&face, rot_pt, rot_dir, Rad(angle as f64));
                    let output_step_file = "revo.stp";
                    let step_string = out::CompleteStepDisplay::new(
                        out::StepModel::from(&s.compress()),
                        out::StepHeaderDescriptor {
                            organization_system: "shape-to-step".to_owned(),
                            ..Default::default()
                        },
                    )
                    .to_string();
                    let mut step_file = std::fs::File::create(&output_step_file).unwrap();
                    std::io::Write::write_all(&mut step_file, step_string.as_ref()).unwrap();

                    let new_s = builder::transformed(&s, Matrix4::from_scale(0.01));
                    let json = serde_json::to_vec_pretty(&new_s).unwrap();
                    std::fs::write("revo.json", json).unwrap();

                    let shell = s.into_boundaries().pop();
                    if shell.is_none() {
                        dbg!(&self);
                    }

                    return shell;
                }
            }
        }
        None
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.verts.iter().flatten().for_each(|v| {
            hash_vec3::<DefaultHasher>(v, &mut hasher);
        });
        "Revolution".hash(&mut hasher);
        hash_f32(self.angle, &mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    #[inline]
    fn tol(&self) -> f32 {
        use parry2d::bounding_volume::Aabb;
        let pts = self
            .verts
            .iter()
            .flatten()
            .map(|x| nalgebra::Point2::from(nalgebra::Vector2::from(x.truncate())))
            .collect::<Vec<_>>();
        let profile_aabb = Aabb::from_points(&pts);
        0.001 * profile_aabb.bounding_sphere().radius.max(1.0)
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimRevolution(self.clone()))
    }

    ///使用manifold生成拉身体的mesh
    #[cfg(all(feature = "gen_model", feature = "truck"))]
    fn gen_csg_mesh(&self) -> Option<PlantMesh> {
        #[cfg(feature = "truck")]
        use truck_meshalgo::prelude::*;
        #[cfg(feature = "truck")]
        use truck_modeling::{Shell, Surface, Wire, builder};
        if !self.check_valid() {
            return None;
        }
        let mut wire = gen_wire(&self.verts, &self.fradius_vec).ok()?;
        if let Ok(face) = builder::try_attach_plane(&[wire.clone()]) {
            if let Surface::Plane(plane) = face.surface() {
                let extrude_dir = Vector3::new(0.0, 0.0, 1.0);
                if plane.normal().dot(extrude_dir) < 0.0 {
                    wire = wire.inverse();
                }
                let e_len = wire.len();
                let pts = wire
                    .edge_iter()
                    .enumerate()
                    .map(|(i, e)| {
                        let curve = e.oriented_curve();
                        let polyline =
                            PolylineCurve::from_curve(&curve, curve.range_tuple(), self.tol() as _);
                        let mut v = polyline
                            .iter()
                            .map(|x| Vec2::new(x.x as _, x.y as _))
                            .collect::<Vec<_>>();
                        if !v.is_empty() && i != (e_len - 1) {
                            v.pop();
                        }
                        v
                    })
                    .flatten()
                    .collect::<Vec<Vec2>>();
                // dbg!(&pts);
                unsafe {
                    let mut cross_section = ManifoldCrossSectionRust::from_points(&pts);
                    let manifold = cross_section.extrude_rotate(300, 360.0);
                    dbg!(manifold.num_tri(), manifold.get_properties());
                    //直接保存到文件，下次要做负实体计算时，直接读取
                    return Some(PlantMesh::from(manifold));
                }
            }
        }
        None
    }

    #[inline]
    fn need_use_csg(&self) -> bool {
        false
    }
}
