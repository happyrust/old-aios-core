use glam::Vec3;
use crate::RefnoEnum;

// https://gitee.com/happydpc/rs-server/issues/IB8S8I
/// 找到支吊架对应的土建预埋板
pub async fn get_supp_panel(refno: RefnoEnum) -> anyhow::Result<String> {
    todo!()
}

// https://gitee.com/happydpc/rs-server/issues/IB8RUF
/// 支吊架下的sctn在空间上找到支撑的bran
pub async fn get_supp_bran(refno: RefnoEnum) -> anyhow::Result<Vec<String>> {
    todo!()
}

// https://gitee.com/happydpc/rs-server/issues/IB8SNG
/// 通过输入支吊架找到支撑的bran，然后找到支吊架旁边两个支架，且着三个支架支撑的都是同一个bran，分别求这个支架与旁边两个支架的距离
pub async fn get_supp_span(refno: RefnoEnum) -> anyhow::Result<[f32; 2]> {
    todo!()
}

// https://gitee.com/happydpc/rs-server/issues/IB9D2S
/// 输入管夹下的PCLA类型，通过管夹找到夹的bran下的管件
pub async fn get_bran_in_pcla(refno: RefnoEnum) -> anyhow::Result<RefnoEnum> {
    todo!()
}

// https://gitee.com/happydpc/rs-server/issues/IB9YKZ
/// 获取panel的长宽
pub async fn get_panel_size(refno: RefnoEnum) -> anyhow::Result<[f32; 2]> {
    todo!()
}

// https://gitee.com/happydpc/rs-server/issues/ICVZO1
/// 支架与预埋板相对定位
pub async fn get_positon_with_panel(refno: RefnoEnum) -> anyhow::Result<Vec3> {
    todo!()
}