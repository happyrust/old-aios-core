use crate::prim_geo::category::CateBrepShape;
use crate::prim_geo::cylinder::SCylinder;
use crate::prim_geo::sbox::SBox;
use crate::prim_geo::tubing::TubiSize::{BoreSize, BoxSize};
use crate::shape::pdms_shape::ANGLE_RAD_TOL;
use crate::shape::pdms_shape::BrepShapeTrait;
use crate::tool::dir_tool::parse_ori_str_to_quat;
use crate::tool::direction_parse::parse_rotation_struct;
use crate::tool::math_tool::{quat_to_pdms_ori_xyz_str, to_pdms_ori_str, to_pdms_vec_str};
use crate::types::named_attvalue::NamedAttrValue;
use crate::types::*;
use approx::abs_diff_eq;
use bevy_math::prelude::Quat;
use bevy_transform::prelude::Transform;
use glam::Mat3;
use glam::Vec3;
use serde::{Deserialize, Serialize};
use serde_with::DisplayFromStr;
use serde_with::serde_as;

#[serde_as]
#[derive(
    Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,
)]
pub struct PdmsTubing {
    #[serde(rename = "_key")]
    pub leave_refno: RefnoEnum,
    pub arrive_refno: RefnoEnum,
    pub start_pt: Vec3,
    pub end_pt: Vec3,
    pub desire_leave_dir: Vec3,
    pub leave_ref_dir: Option<Vec3>,
    pub desire_arrive_dir: Vec3,
    pub tubi_size: TubiSize,
    pub index: usize,
}

// remove the _from and _to
// 存放在图数据库的 tubi 的数据
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct TubiEdge {
    pub _key: String,
    pub _from: String,
    pub _to: String,
    pub start_pt: Vec3,
    pub end_pt: Vec3,
    pub att_type: String,
    pub extra_type: String,
    pub tubi_size: TubiSize,
    pub bran_name: String,
}

impl TubiEdge {
    pub fn new_from_edge() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn get_enco_header() -> Vec<String> {
        vec![
            "_key".to_string(),
            "_from".to_string(),
            "_to".to_string(),
            "start_pt".to_string(),
            "end_pt".to_string(),
            "att_type".to_string(),
            "extra_type".to_string(),
            "tubi_size".to_string(),
            "bran_name".to_string(),
            "distance".to_string(),
        ]
    }

    pub fn into_enso_value(self) -> Vec<NamedAttrValue> {
        let distance = self.start_pt.distance(self.end_pt);
        let tubi_size = match self.tubi_size {
            TubiSize::None => NamedAttrValue::F32VecType(vec![0.0]),
            BoreSize(d) => NamedAttrValue::F32VecType(vec![d]),
            BoxSize((x, y)) => NamedAttrValue::F32VecType(vec![x, y]),
        };
        vec![
            NamedAttrValue::StringType(self._key),
            NamedAttrValue::StringType(self._from),
            NamedAttrValue::StringType(self._to),
            NamedAttrValue::F32VecType(vec![self.start_pt.x, self.start_pt.y, self.start_pt.z]),
            NamedAttrValue::F32VecType(vec![self.end_pt.x, self.end_pt.y, self.end_pt.z]),
            NamedAttrValue::StringType(self.att_type),
            NamedAttrValue::StringType(self.extra_type),
            tubi_size,
            NamedAttrValue::StringType(self.bran_name),
            NamedAttrValue::F32Type(distance),
        ]
    }
}

#[serde_as]
#[derive(
    PartialEq,
    Default,
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub enum TubiSize {
    #[default]
    None,
    BoreSize(f32),
    BoxSize((f32, f32)),
}

impl ToString for TubiSize {
    fn to_string(&self) -> String {
        serde_json::to_string(&self.to_vec()).unwrap_or_default()
    }
}

impl TubiSize {
    pub fn to_vec(&self) -> Vec<f32> {
        match self {
            TubiSize::None => vec![],
            TubiSize::BoreSize(d) => vec![*d],
            TubiSize::BoxSize((w, h)) => vec![*w, *h],
        }
    }
}

impl PdmsTubing {
    ///获得方向
    #[inline]
    pub fn get_dir(&self) -> Vec3 {
        (self.end_pt - self.start_pt).normalize_or_zero()
    }

    /// 是否方向是ok的
    #[inline]
    pub fn is_dir_ok(&self) -> bool {
        let a = self.desire_leave_dir.normalize_or_zero();
        let b = -self.desire_arrive_dir.normalize_or_zero();
        let c = self.get_dir();
        abs_diff_eq!(a.dot(c), 1.0, epsilon = 0.01) && abs_diff_eq!(b.dot(c), 1.0, epsilon = 0.01)
    }

    /// 获得tubi的transform
    pub fn get_transform(&self) -> Option<Transform> {
        let v = self.end_pt - self.start_pt;
        let len = v.length();
        let is_bore = matches!(self.tubi_size, TubiSize::BoreSize(_));
        let leave_dir = if is_bore {
            v.normalize_or_zero()
        } else {
            self.desire_leave_dir.normalize_or_zero()
        };

        if self.tubi_size == TubiSize::None || leave_dir.length().abs() < f32::EPSILON {
            return None;
        }
        let scale = match self.tubi_size {
            TubiSize::BoreSize(bore) => Vec3::new(bore, bore, len),
            TubiSize::BoxSize((w, h)) => Vec3::new(w, len, h),
            _ => Vec3::ONE,
        };
        //统一都用Z轴为参考轴的方法
        let rotation = if let Some(ref_dir) = self.leave_ref_dir
            && !is_bore
        {
            let ydir = leave_dir;
            let zdir = ref_dir;
            let xdir = ydir.cross(zdir).normalize_or_zero();
            // dbg!((xdir, ydir, zdir));
            let rot = Quat::from_mat3(&Mat3::from_cols(xdir, ydir, zdir));
            // dbg!(quat_to_pdms_ori_xyz_str(&rot));
            rot
            // Quat::IDENTITY
        } else {
            if is_bore {
                //圆柱体是以Z方向做的拉伸，后面如果要和e3d一致，还需要还原到以Y方向拉伸
                Quat::from_rotation_arc(Vec3::Z, leave_dir)
            } else {
                Quat::from_rotation_arc(Vec3::Y, leave_dir)
            }
        };

        let translation = match self.tubi_size {
            TubiSize::BoreSize(_) => self.start_pt,
            TubiSize::BoxSize(_) => self.start_pt + (v * 0.5),
            // TubiSize::BoxSize(_) => self.start_pt,
            _ => self.start_pt,
        };

        Some(Transform {
            rotation,
            translation,
            scale,
        })
    }

    pub fn convert_to_shape(&self) -> Option<CateBrepShape> {
        let dir = (self.end_pt - self.start_pt).normalize();
        let brep_shape: Option<Box<dyn BrepShapeTrait>> = match &self.tubi_size {
            TubiSize::BoreSize(d) => {
                let cylinder = SCylinder {
                    phei: self.start_pt.distance(self.end_pt),
                    pdia: *d,
                    center_in_mid: false,
                    ..Default::default()
                };
                Some(Box::new(cylinder))
            }
            TubiSize::BoxSize((w, h)) => {
                let len = self.start_pt.distance(self.end_pt);
                let size = Vec3::new(*w, *h, len);
                let cube = SBox {
                    center: Default::default(),
                    size,
                };
                Some(Box::new(cube))
            }
            _ => None,
        };
        if brep_shape.is_none() {
            return None;
        }

        Some(CateBrepShape {
            refno: self.leave_refno,
            brep_shape: brep_shape.unwrap(),
            transform: Transform {
                rotation: Quat::from_rotation_arc(Vec3::Z, dir),
                translation: self.start_pt,
                scale: Vec3::ONE,
            },
            visible: true,
            is_tubi: true,
            shape_err: None,
            pts: Default::default(),
            is_ngmr: false,
        })
    }
}
