use crate::parsed_data::CateAxisParam;
use glam::Vec3;
use std::ops::Neg;

impl Neg for CateAxisParam {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            // dir: Vec3::new(-self.dir[0].clone(), -self.dir[1].clone(), -self.dir[2].clone()),
            dir: self.dir.map(|x| -x),
            dir_flag: -1.0,
            ..self.clone()
        }
    }
}

impl CateAxisParam {
    pub fn zero() -> Self {
        Self {
            pt: Vec3::ZERO,
            dir: None,
            pconnect: "".to_string(),
            ..Default::default()
        }
    }

    pub fn x() -> Self {
        Self {
            pt: Vec3::ZERO,
            dir: Some(Vec3::X),
            pconnect: "".to_string(),
            ..Default::default()
        }
    }

    pub fn y() -> Self {
        Self {
            pt: Vec3::ZERO,
            dir: Some(Vec3::Y),
            pconnect: "".to_string(),
            ..Default::default()
        }
    }

    pub fn z() -> Self {
        Self {
            pt: Vec3::ZERO,
            dir: Some(Vec3::Z),
            pconnect: "".to_string(),
            ..Default::default()
        }
    }
}
