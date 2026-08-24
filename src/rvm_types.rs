use crate::geometry::PlantGeoData;
use crate::types::*;
use serde_derive::{Deserialize, Serialize};
use serde_with::DisplayFromStr;
use serde_with::serde_as;

/// rvm 格式类型
#[derive(Debug, Clone)]
pub enum RvmShapeTypeData {
    /// 0: bottom width, 1: bottom length , 2:top width, 3:top length ,4:x offset, 5: y offset, 6: height
    Pyramid([f32; 7]),
    /// 长 宽 高
    Box([f32; 3]),
    /// 0:弧长半径, 1:矩形的宽, 2: 矩形的长 3: 角度: π/n
    RectangularTorus([f32; 4]),
    /// 0:弧长半径, 1: 圆半径 2: 角度: π/n
    CircularTorus([f32; 3]),
    /// 0:radius 1: height
    EllipticalDish([f32; 2]),
    /// 半径 高
    SphericalDish([f32; 2]),
    /// 0: bottom radius 1 : top radius 2: height 3: offset
    Snout([f32; 9]),
    /// 半径 高
    Cylinder([f32; 2]),
    /// 球体
    Sphere,
    /// 0: 1: 长度(mm)
    Line([f32; 2]),
    /// 多面体
    FacetGroup,
}

impl RvmShapeTypeData {
    /// 获得 ShapeType在 Prim种代表的数字
    pub fn get_shape_number(&self) -> u8 {
        match self {
            RvmShapeTypeData::Pyramid(_) => 1,
            RvmShapeTypeData::Box(_) => 2,
            RvmShapeTypeData::RectangularTorus(_) => 3,
            RvmShapeTypeData::CircularTorus(_) => 4,
            RvmShapeTypeData::EllipticalDish(_) => 5,
            RvmShapeTypeData::SphericalDish(_) => 6,
            RvmShapeTypeData::Snout(_) => 7,
            RvmShapeTypeData::Cylinder(_) => 8,
            RvmShapeTypeData::Sphere => 9,
            RvmShapeTypeData::Line(_) => 10,
            RvmShapeTypeData::FacetGroup => 11,
        }
    }
    pub fn convert_shape_type_to_bytes(&self) -> Vec<u8> {
        let mut data = vec![];
        match &self {
            RvmShapeTypeData::Pyramid(array) => {
                data.append(
                    &mut format!(
                        "     {:.7}     {:.7}     {:.7}     {:.7}\r\n",
                        array[0], array[1], array[2], array[3]
                    )
                    .into_bytes(),
                );
                data.append(
                    &mut format!(
                        "     {:.7}     {:.7}     {:.7}\r\n",
                        array[4], array[5], array[6]
                    )
                    .into_bytes(),
                );
            }
            RvmShapeTypeData::Box(array) => {
                data.append(
                    &mut format!(
                        "     {:.7}     {:.7}     {:.7}\r\n",
                        array[0], array[1], array[2]
                    )
                    .into_bytes(),
                );
            }
            RvmShapeTypeData::RectangularTorus(array) => {
                data.append(
                    &mut format!(
                        "     {:.7}     {:.7}     {:.7}     {:.7}\r\n",
                        array[0], array[1], array[2], array[3]
                    )
                    .into_bytes(),
                );
            }
            RvmShapeTypeData::CircularTorus(array) => {
                data.append(
                    &mut format!(
                        "     {:.7}     {:.7}     {:.7}\r\n",
                        array[0], array[1], array[2]
                    )
                    .into_bytes(),
                );
            }
            RvmShapeTypeData::EllipticalDish(array) => {
                data.append(
                    &mut format!("     {:.7}     {:.7}\r\n", array[0], array[1]).into_bytes(),
                );
            }
            RvmShapeTypeData::SphericalDish(arr) => {
                data.append(&mut format!("     {:.7}     {:.7}\r\n", arr[0], arr[1]).into_bytes());
            }
            RvmShapeTypeData::Snout(array) => {
                data.append(
                    &mut format!(
                        "     {:.7}     {:.7}     {:.7}     {:.7}     {:.7}\r\n",
                        array[0], array[1], array[2], array[3], array[4]
                    )
                    .into_bytes(),
                );
                data.append(
                    &mut format!(
                        "     {:.7}     {:.7}     {:.7}     {:.7}\r\n",
                        array[5], array[6], array[7], array[8]
                    )
                    .into_bytes(),
                );
            }
            RvmShapeTypeData::Cylinder(array) => {
                data.append(
                    &mut format!("     {:.7}     {:.7}\r\n", array[0], array[1]).into_bytes(),
                );
            }
            RvmShapeTypeData::Line(arr) => {
                data.append(&mut format!("     {:.7}     {:.7}\r\n", arr[0], arr[1]).into_bytes());
            }
            _ => {}
        }
        data
    }
}

#[serde_as]
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RvmPlatMesh {
    #[serde_as(as = "DisplayFromStr")]
    pub refno: RefU64,
    pub mesh: PlantGeoData,
}
