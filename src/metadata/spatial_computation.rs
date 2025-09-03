use crate::{RefnoEnum, SUL_DB};
use glam::Vec3;

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
    // 获取panel的点集
    let Some(points) = get_panel_points(refno).await? else {
        return Ok([0.0, 0.0]);
    };
    let size = rectangle_dimensions(points, 0.1).unwrap_or((0.0, 0.0));
    Ok([size.0, size.1])
}

// https://gitee.com/happydpc/rs-server/issues/ICVZO1
/// 支架与预埋板相对定位
pub async fn get_position_with_panel(refno: RefnoEnum) -> anyhow::Result<Vec3> {
    todo!()
}

/// 计算向量
fn vec_from(p1: Vec3, p2: Vec3) -> (f32, f32, f32) {
    (p2.x - p1.x, p2.y - p1.y, p2.z - p1.z)
}

/// 向量点积
fn dot(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    a.0 * b.0 + a.1 * b.1 + a.2 * b.2
}

/// 向量长度
fn length(v: (f32, f32, f32)) -> f32 {
    (v.0 * v.0 + v.1 * v.1 + v.2 * v.2).sqrt()
}

/// 判断两个向量是否近似垂直
fn is_perpendicular(a: (f32, f32, f32), b: (f32, f32, f32), tol: f32) -> bool {
    dot(a, b).abs() < tol
}

/// 检查四个点按顺序是否为矩形
fn is_rectangle_ordered(points: &[Vec3; 4], tol: f32) -> Option<(f32, f32)> {
    let v1 = vec_from(points[0], points[1]);
    let v2 = vec_from(points[1], points[2]);
    let v3 = vec_from(points[2], points[3]);
    let v4 = vec_from(points[3], points[0]);

    let l1 = length(v1);
    let l2 = length(v2);
    let l3 = length(v3);
    let l4 = length(v4);

    // 判断相邻边是否垂直
    if !(is_perpendicular(v1, v2, tol)
        && is_perpendicular(v2, v3, tol)
        && is_perpendicular(v3, v4, tol)
        && is_perpendicular(v4, v1, tol))
    {
        return None;
    }

    // 对边长度相等
    if (l1 - l3).abs() > tol || (l2 - l4).abs() > tol {
        return None;
    }

    Some((l1, l2)) // 返回长和宽
}

/// 主函数：支持点的顺序是乱的
fn rectangle_dimensions(mut points: [Vec3; 4], tol: f32) -> Option<(f32, f32)> {
    let indices = [0, 1, 2, 3];
    let mut perm = indices.to_vec();

    // 全排列 4! = 24
    fn next_permutation(arr: &mut Vec<usize>) -> bool {
        let n = arr.len();
        let mut i = n - 2;
        while i != usize::MAX && arr[i] >= arr[i + 1] {
            if i == 0 {
                break;
            }
            i -= 1;
        }
        if arr[i] >= arr[i + 1] && i == 0 {
            return false;
        }
        let mut j = n - 1;
        while arr[j] <= arr[i] {
            j -= 1;
        }
        arr.swap(i, j);
        arr[i + 1..].reverse();
        true
    }
    loop {
        let perm_points = [
            points[perm[0]],
            points[perm[1]],
            points[perm[2]],
            points[perm[3]],
        ];
        if let Some((w, h)) = is_rectangle_ordered(&perm_points, tol) {
            return Some((w, h));
        }
        if !next_permutation(&mut perm) {
            break;
        }
    }
    None
}

/// 获取panel的四个点集
async fn get_panel_points(refno: RefnoEnum) -> anyhow::Result<Option<[Vec3; 4]>> {
    // 查询坐标
    let sql = format!("(select value in.id.refno.POS from ( select value in.id from {}<-pe_owner where in.noun == 'PLOO' ) <-pe_owner)[0]",refno.to_pe_key());
    let mut resp = SUL_DB.query(&sql).await?;
    let r: Vec<Vec<f32>> = resp.take(0)?;
    // 分组
    if r.len() != 4 {
        return Ok(None);
    }
    let mut arr: [Vec3; 4] = [Vec3::ZERO; 4];
    for (i, p) in r.into_iter().enumerate() {
        if p.len() != 3 {
            return Ok(None);
        }
        arr[i] = Vec3::new(p[0], p[1], p[2]);
    }
    Ok(Some(arr))
}
