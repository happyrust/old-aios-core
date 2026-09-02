//! 曲面原语怎么分段 —— libgm 的权威规则里**进身份键**的那一半，全库唯一一份。
//!
//! 逆向来源是 libgeom 导出的两个自由函数（libgm 只是导入方）：
//! `d2_numberOfSegmentsForCircle`（3.1 libgeom `0x1002BA70`）与
//! `d2_numberOfSegmentsForPartRev`（`0x1002BB20`）。全文与常量见
//! `plant-4/libgm-boolean-algorithm.md` §7.9。
//!
//! 为什么住在这个 crate：可复用的单位网格按 `hash_unit_mesh_params()` 寻址，而段数是
//! 那个键的一部分（gen-model `specs/009-retire-occ` T041 / ADR-044 决策 2）——单位行的
//! 半径恒为 1，真实尺寸只在实例变换的 `scale` 里，所以段数**必须在归一化之前、在原件
//! 上**算好，随单位参数一起落库。算键的是本 crate 的 `prim_geo::*`，规则只能住在它
//! 够得着的地方。gen-model 的 `fast_model::libgm_discretise` 原样重导出这里的每一项，
//! 轮廓 / 截面那一半（`span_*` / `profile_*`，不进键）留在那边。
//!
//! 为什么要照抄而不是"差不多就行"：`cancelFacets` **只消全等重叠**（同文 §6.11）。
//! 共面的两层侧壁段数差一段，共面抵消就整个放弃，结果里留一层内壁。段数规则因此
//! 是布尔能不能收敛的前置条件，不是画质旋钮。
//!
//! 容差从哪来：libgm 是一个全局 `GM_User::arctol_`（初值 0.1，`gm_SetDefaultFacetTolerance`
//! 改；Core3D 主初始化那一处传 **0.5**，另有一条粗路径传 10.0），创建原语时读一次
//! 烤进对象。本仓对齐成 [`FACET_TOL_MM`]，也是全局一个绝对量（gen-model T042）。

/// 曲面离散的弦高容差（mm），**绝对量，全库唯一一份**。
///
/// 对齐 libgm：`GM_User::arctol_` 是一个全局绝对量，Core3D 主初始化那一处传的就是
/// 0.5（另有一条粗路径传 10.0；`arctol_` 自身初值是 0.1）。见
/// `plant-4/libgm-boolean-algorithm.md` §7.9。
///
/// 为什么不能沿用 `BrepShapeTrait::tol()`：那些是**按自身尺度给的比例容差**
/// （挤出/回转是千分之一截面半径，`SweepSolid` 是百分之一轮廓外接球半径），`tol/R`
/// 于是恒定，段数与尺寸无关——同一个圆在挤出侧和回转侧只要包围盒不同就会分成不同
/// 段数。`=24381/36945` 那颗穹顶正是这样：正体圆柱 60 段、负体圆柱 84 段，同一道墙
/// 两个多边形，差集在赤道上留下一圈毫米级残料。改成绝对量之后两侧拿到同一个段数，
/// 而 libgm「段数取到 4 的倍数」保证两者的顶点相位也一致（都落在 0/90/180/270 上），
/// 残料才真正消失。
///
/// **它进了身份键。** 五类复用曲面原语的 `hash_unit_mesh_params()` 混入按它算出的段数，
/// 所以它今天是常量、不进键；**将来若接成配置项，改容差 = 改身份 = 整库重建**
/// （gen-model specs/009 T041 C2）。要做成可配之前，别在别处再写第二个容差来源。
pub const FACET_TOL_MM: f64 = 0.5;

/// 收到的弦高容差能不能用。不能用的**必须报错**，不许兜一个默认值。
///
/// 折线化那几处原先写的是 `let tol = if chord_tol > 0.0 { chord_tol } else { 1.0 };`，
/// 三份拷贝各带一个 1.0mm。它们今天不可达（生产路径喂的都是 [`FACET_TOL_MM`]），
/// 可一旦有人把容差接成配置项或按构件算，非正值就会**静默**变成 1.0mm：段数比
/// 0.5mm 少一半，而 `cancelFacets` 只消全等重叠——共面处留一层壁，现场只看得到
/// 布尔结果里多一层内壁，没有任何一行日志指向容差。
pub fn chord_tol_is_usable(chord_tol: f64) -> bool {
    chord_tol.is_finite() && chord_tol > 0.0
}

/// 段数上限，对齐 libgm 的 1000（同文 §7.9.1）。
///
/// 这个上限**不在** libgeom 的公式里：`d2_numberOfSegmentsForCircle` 自己不封顶，是每个
/// `GM_*::calcFacets` 各自 `if (n > 1000) n = 1000`，同时打一条
/// 「facet tolerance too small for radius, adjusted」。所以它跟段数规则一样是复刻项，
/// 不是我们的画质旋钮——取别的数，撞顶的那些圆就跟 E3D 逐面对不上。
///
/// **只对曲面原语是这个形状。** 轮廓（`GM_Profile`）那条路上封顶也存在，但不是逐段
/// 截断：`polygonForFacet` 先按整条轮廓求实际点数，超过 1000 就清空步数数组、把容差
/// 放大 `((total − nSpans) / (1000 − nSpans))²` 再整条重算（同文 §7.9.2）。那一套在
/// gen-model 的 `libgm_discretise::profile_steps`。
///
/// 按 `facet_tol = 0.5mm`，R=23400（RM13 穹顶）要 484 段，离顶还远；把容差调到 0.05
/// 同一个圆是 1532 段，那才会撞顶。撞到说明容差给得过细，应当去调容差。
pub const MAX_SEGMENTS: i32 = 1000;

/// `d2_numberOfSegmentsForCircle(radius, tol)`：整圆分几段。
///
/// ```text
/// step = 2·acos(1 − |tol/R|)      弦高 R(1 − cos(step/2)) ≤ tol 的最大圆心角
/// step 封顶 45°                   ⇒ 整圆最少 8 段
/// n = ceil(360/step)，再向上取到 4 的倍数
/// ```
///
/// 那个「4 的倍数」不是凑整：它保证 0/90/180/270 四个象限点落在网格上。少了它，
/// 段数会跟 E3D 差 1~3 段。
pub fn circle_segments(radius: f64, chord_tol: f64) -> i32 {
    circle_segments_uncapped(radius, chord_tol).min(MAX_SEGMENTS)
}

/// `d2_numberOfSegmentsForCircle` 本身，不封顶。
///
/// 封顶是各 `GM_*::calcFacets` 各封各的（§7.9.1），**挤出截面那条路上没有**：
/// `GM_Extrusion::calcFacets`（3.1 libgm `0x10056F10`）直接把 `arctol_` 交给
/// `D2_Span::getApproxPolyLine`，中间不经过任何 `if (n > 1000)`。所以挤出截面弧要用
/// 这一支，曲面原语用上面那支。
///
/// **回转 / collar 的截面是第三种，两支都不是。** `GM_Revolution` 与 `GM_Collar` 走
/// `GM_Profile::polygonForFacet` → `setNSteps`（§7.9.2）：段数取「自身半径与配对 span
/// 半径的大者」、与已存步数单调取大，整条轮廓的实际点数超过 1000 时放大容差重算。
/// 那一支在 gen-model 的 `libgm_discretise::profile_steps`。
pub fn circle_segments_uncapped(radius: f64, chord_tol: f64) -> i32 {
    if !(radius > 0.0) {
        return 1;
    }
    let x = (1.0 - (chord_tol / radius).abs()).max(0.0);
    let mut step_deg = x.acos().to_degrees() * 2.0;
    if !(step_deg > 0.0) || step_deg > 45.0 {
        step_deg = 45.0;
    }
    let n = (360.0 / step_deg).ceil() as i32;
    (n + 3) & !3
}

/// `d2_numberOfSegmentsForPartRev` 的返回：段数、是否整圈、归一化后的起止角（度）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PartRev {
    pub segments: i32,
    pub is_full: bool,
    pub start_deg: f64,
    pub end_deg: f64,
}

/// `d2_numberOfSegmentsForPartRev(radius, tol, &start, &end, &isFull)`：部分回转分几段。
///
/// 先把区间归一化到 `start < end ≤ start + 360`，再**按整圈段数等比例缩**
/// （不是拿扫角直接除步长——那样得到的数跟 E3D 会差一段），最少 2 段。
/// 扫角在 1e-6 度内等于 0 或 360 时判整圈，起止角改写成 0/360。
pub fn part_rev_segments(radius: f64, chord_tol: f64, start_deg: f64, end_deg: f64) -> PartRev {
    let mut start = start_deg;
    let mut end = end_deg;
    // libgm 是 while 循环逐圈加减，角度大到几万度会转很久；这里直接算差几圈，
    // 结果与逐圈推进一致（NaN / 无穷大交给下面的 is_finite 兜底）。
    if start.is_finite() && end.is_finite() {
        if start >= end {
            let turns = ((start - end) / 360.0).floor() + 1.0;
            end += 360.0 * turns;
        }
        if end > start + 360.0 {
            let turns = ((end - start - 360.0) / 360.0).ceil();
            end -= 360.0 * turns;
        }
    }

    let n_full = circle_segments(radius, chord_tol);
    let sweep = end - start;
    if (sweep.abs() <= 1e-6) || ((sweep - 360.0).abs() <= 1e-6) {
        return PartRev {
            segments: n_full,
            is_full: true,
            start_deg: 0.0,
            end_deg: 360.0,
        };
    }
    let n = (n_full as f64 * sweep / 360.0).ceil() as i32;
    PartRev {
        segments: n.max(2),
        is_full: false,
        start_deg: start,
        end_deg: end,
    }
}

/// 扫角以弧度给时的便捷入口（几何量多数是弧度）。起点固定在 0。
pub fn sweep_segments_rad(radius: f64, chord_tol: f64, sweep_rad: f64) -> i32 {
    part_rev_segments(radius, chord_tol, 0.0, sweep_rad.to_degrees()).segments
}

// ─── §7.9.1 调用点表：每个曲面原语把哪个半径喂进去 ──────────────────────────
//
// 公式只是一半，另一半在调用点上。同一个 `d2_numberOfSegmentsForCircle`，各原语喂的
// 半径不是同一个量——喂错了段数就跟 E3D 对不上，`cancelFacets` 只消全等重叠（§6.11），
// 共面抵消随之整个放弃。下面每个函数对应 3.1 libgm 里的一个 `calcFacets` 调用点，
// 括号里是地址。**不要在别处再写第二份**，也不要拿其中一个顶替另一个。
//
// 这一张表同时就是身份键的段数来源：`prim_geo::{LCylinder, SCylinder, Sphere, LSnout,
// CTorus, RTorus, Dish}::hash_unit_mesh_params()` 与它们的 `gen_unit_shape()` 都从这里
// 取数，键与落库值同源（T041 A3）。

/// `GM_Cylinder::calcFacets`（`0x100532F0`）、`GM_SlopeEndCyl`（`0x1009DFC0`）、
/// `GM_Sphere`（`0x100A20F0`）：直接喂自己的半径。
pub fn cylinder_segments(radius: f64, chord_tol: f64) -> i32 {
    circle_segments(radius, chord_tol)
}

/// `GM_Sphere::calcFacetsWithoutSurfaces`（`0x100A20F0`）的经向带数：**恒为绕轴的一半**，
/// 不是独立自由度——顶点 `n·(n/2−1)+2`，两极各一点，角步长两向同一个。所以球的身份键
/// 只混 `n` 一个数（T041 B5），这里只是把那一半算出来给生成器。
pub fn sphere_stacks(around: i32) -> i32 {
    (around / 2).max(1)
}

/// `GM_Snout::calcFacets`（`0x1009EA30`）：喂**两端半径的大者**。
///
/// 不是底也不是顶——锥度大时两者差很远，取错哪一个都会让侧壁跟相邻圆柱对不上。
pub fn snout_segments(r_bottom: f64, r_top: f64, chord_tol: f64) -> i32 {
    circle_segments(r_bottom.max(r_top), chord_tol)
}

/// `GM_CircTorus`（`0x10047150`）/ `GM_RectTorus`（`0x100962F0`）的扫掠方向：
/// 喂**外半径**，不是中心线半径，且走部分回转那一支。
///
/// 返回的是**段数**。libgm 在非整圈时还会 `+1`，那一下是段数转顶点数——gen-model 的
/// `mesh_primitives::gen_circular_torus` / `gen_rectangular_torus` 内部已经做了
/// （`ring_count = ring_segments + 1`），所以这里**不要再加一次**。
pub fn torus_ring_segments(r_outside: f64, chord_tol: f64, sweep_deg: f64) -> i32 {
    part_rev_segments(r_outside, chord_tol, 0.0, sweep_deg).segments
}

/// `GM_CircTorus` 的管截面方向：喂 `(rOut − rIns) / 2`。
///
/// 两个方向喂两个不同的半径，这是 §7.9.1 点名「容易照抄错」的一条。
pub fn circular_torus_tube_segments(r_inside: f64, r_outside: f64, chord_tol: f64) -> i32 {
    circle_segments((r_outside - r_inside) * 0.5, chord_tol)
}

/// `GM_SDish::calcFacets`（`0x10099CF0`）解出来的球碟离散参数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SphericalDishFacets {
    /// 球面半径 `R = (a²/h + h) / 2`。
    pub sphere_radius: f64,
    /// 极角 `θ`（弧度）：从顶点量到底面边缘。`h > a` 时超过 90°。
    pub polar_angle: f64,
    /// 绕轴段数。
    pub around: i32,
    /// 经向段数。
    pub meridional: i32,
}

/// `GM_SDish::calcFacets`（`0x10099CF0`）：**两个方向都不是常数**。
///
/// 绕轴先算球半径 `R = (a²/h + h)/2`，再按 `h ≥ a ? R : a` 选——实质是「这个封头上
/// 最大的那个圆」：超过半球时最大圆在赤道而不在底面。
///
/// 经向**不另算容差**，直接沿用绕轴的角步长：`θ = acos(1 − h/R)`
/// （`|h/R| ≤ 1e-6` 时退化成 `sqrt(2h/R)`），段数 `ceil(θ / (2π/n))`。
///
/// 注意 `θ` 要用 `acos(1 − h/R)` 而不是 `asin(a/R)`：两者只在 `h ≤ a`（不超过半球）
/// 时相等，`h > a` 时 `asin` 会把钝角折回锐角，碟顶直接被削平。
pub fn spherical_dish_facets(
    base_radius: f64,
    height: f64,
    chord_tol: f64,
) -> Option<SphericalDishFacets> {
    if !(base_radius > 0.0) || !(height > 0.0) {
        return None;
    }
    let sphere_radius = (base_radius * base_radius / height + height) * 0.5;
    if !(sphere_radius > 0.0) {
        return None;
    }
    let ratio = height / sphere_radius;
    let polar_angle = if ratio.abs() <= 1e-6 {
        (2.0 * ratio).sqrt()
    } else {
        (1.0 - ratio).clamp(-1.0, 1.0).acos()
    };
    let around = circle_segments(
        if height >= base_radius {
            sphere_radius
        } else {
            base_radius
        },
        chord_tol,
    );
    let step = std::f64::consts::TAU / f64::from(around.max(1));
    let meridional = (polar_angle / step).ceil().max(1.0) as i32;
    Some(SphericalDishFacets {
        sphere_radius,
        polar_angle,
        around,
        meridional,
    })
}

/// `GM_EDish::calcFacetsWithoutSurfaces`（`0x10054AB0`）解出来的椭圆碟参数。
///
/// **「椭圆碟」是 PDMS 的叫法，形状不是椭球**：libgm 建的是托里球形封头——一段球冠
/// （半径 `hub_radius`）加一圈与它相切的环面拐角（管半径 `knuckle_radius`）。
/// 前三个字段是母线的形状参数，后三个才是段数；两者一起返回是因为 libgm 也是在
/// 同一个函数里先算形状再按形状分段，拆开只会让两边各存一份公式。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EllipticalDishFacets {
    /// 拐角环的管半径 `r_k`。
    pub knuckle_radius: f64,
    /// 球冠半径 `R_c`（`radiusOfHub`）。球心在轴上 `z = h − R_c`。
    pub hub_radius: f64,
    /// 球冠与拐角的交接角（弧度），从 **+Z 极点**量起。
    pub transition_angle: f64,
    /// 绕轴段数。
    pub around: i32,
    /// 球冠段的经向段数。
    pub hub: i32,
    /// 拐角段的经向段数。
    pub knuckle: i32,
}

/// `GM_EDish::calcFacetsWithoutSurfaces`（`0x10054AB0`）：形状与段数一次解出。
///
/// 记 `a = base_radius`（= DIAM/2）、`h = height`、`s = √(a² + h²)`：
///
/// ```text
/// r_k = h / (1 + (a − h)/s)                       knuckleRadiusToUse  0x100556A0
/// R_c = (a² + h² − 2a·r_k) / (2(h − r_k))         radiusOfHub         0x10055750
/// θ   = acos(1 − (h − r_k)/(R_c − r_k))                               0x10054CCB
///     = acos((R_c − h)/(R_c − r_k)) = atan2(h, a)
/// n_around  = circle(a, tol)                      喂底半径，不是 R_c
/// n_hub     = partRev(R_c, tol, 0°, θ°)
/// n_knuckle = partRev(r_k, tol, θ°, 90°)
/// 2(n_hub + n_knuckle) > 1000 时，4·n > 1000 的那一段各自夹到 250
/// ```
///
/// 三处容易抄错的地方：
///
/// 1. **`RADI` 只是开关。** Core3D 的 `CSG_BasicDIS::getPrimGeom`（`0x10726D10`）读了
///    `ATT_RADI` 却只用它判「椭圆碟还是球碟」，传给 `gm_CreateEllipticalDish` 的第三个
///    实参是上面那条现算的 `r_k`。用户填的拐角半径**不进几何**。
/// 2. **`θ` 不是 `acos((h − r_k)/(R_c − r_k))`。** 那是 Hex-Rays 吞掉 acos 实参之后的
///    伪码假象；反汇编是 `acos(1 − q)`，小 `q` 分支的 `sqrt(2q)` 也只有对 `1 − q` 才是
///    正确的小角展开。抄错的话 a=2 / h=1 会得到 83.9° 而不是 26.6°，碟身留一道折痕。
/// 3. **绕轴喂 `a`。** 喂 `R_c` 会得到另一个数，而 `R_c` 恰好也是个「看着很像半径」的量。
///
/// `isSpherical()`（`|a − h| ≤ 1e-6`）时 `θ` 直接取 45°、`R_c` 保持等于 `r_k`：
/// 半球被拆成 0–45° 与 45–90° 两段同半径的弧。
///
/// **与 libgm 的一处有意分歧**：`R_c == r_k` 时 libgm 打
/// `gm_reportInternalFault(GM_EDish.cxx:171)` 之后仍按 45° 继续出网格；这里回 `None`，
/// 由调用方响亮失败。内部故障之后接着造几何，不是本仓要复刻的行为。
pub fn elliptical_dish_facets(
    base_radius: f64,
    height: f64,
    chord_tol: f64,
) -> Option<EllipticalDishFacets> {
    let (a, h) = (base_radius, height);
    if !(a > 0.0) || !(h > 0.0) || !chord_tol_is_usable(chord_tol) {
        return None;
    }
    let s = (a * a + h * h).sqrt();
    let knuckle_radius = h / ((a - h) / s + 1.0);
    if !(knuckle_radius > 0.0) {
        return None;
    }

    let (hub_radius, transition_angle) = if (a - h).abs() <= 1e-6 {
        (knuckle_radius, std::f64::consts::FRAC_PI_4)
    } else {
        let hub_radius = (a * a + h * h - 2.0 * a * knuckle_radius) / (2.0 * (h - knuckle_radius));
        let den = hub_radius - knuckle_radius;
        if den == 0.0 || !hub_radius.is_finite() {
            return None;
        }
        let q = (h - knuckle_radius) / den;
        let angle = if q.abs() > 1e-6 {
            (1.0 - q).clamp(-1.0, 1.0).acos()
        } else {
            (2.0 * q).max(0.0).sqrt()
        };
        (hub_radius, angle)
    };
    if !(hub_radius > 0.0) || !transition_angle.is_finite() {
        return None;
    }

    let around = circle_segments(a, chord_tol);
    let theta_deg = transition_angle.to_degrees();
    let mut hub = part_rev_segments(hub_radius, chord_tol, 0.0, theta_deg).segments;
    let mut knuckle = part_rev_segments(knuckle_radius, chord_tol, theta_deg, 90.0).segments;
    if 2 * (hub + knuckle) > MAX_SEGMENTS {
        if 4 * knuckle > MAX_SEGMENTS {
            knuckle = 250;
        }
        if 4 * hub > MAX_SEGMENTS {
            hub = 250;
        }
    }

    Some(EllipticalDishFacets {
        knuckle_radius,
        hub_radius,
        transition_angle,
        around,
        hub,
        knuckle,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 按 E3D 主初始化用的 `facet_tol = 0.5mm` 手算的对照表。gen-model 那边的
    /// `libgm_discretise` 单测对同一组函数还有更全的一套（经重导出跑）；这里只钉
    /// 「规则确实住在本 crate、取值没变」这一条，规则搬家时两边一起红。
    #[test]
    fn circle_segments_match_libgm_at_the_core3d_default_tolerance() {
        for (radius, expect) in [
            (1.0, 8),
            (25.0, 16),
            (100.0, 32),
            (250.0, 52),
            (3000.0, 176),
            (23400.0, 484),
        ] {
            assert_eq!(circle_segments(radius, FACET_TOL_MM), expect, "R={radius}");
        }
        assert_eq!(circle_segments(23400.0, 0.05), MAX_SEGMENTS, "曲面原语封在 1000");
    }

    /// 球的经向带数恒为绕轴一半（`0x100A20F0`）——它不是独立自由度，键里也不许出现。
    #[test]
    fn sphere_stacks_are_half_the_around_count() {
        assert_eq!(sphere_stacks(32), 16);
        assert_eq!(sphere_stacks(8), 4);
        assert_eq!(sphere_stacks(1), 1, "退化输入也不许给 0 带");
    }

    #[test]
    fn the_call_site_table_feeds_the_documented_radii() {
        assert_eq!(snout_segments(25.0, 100.0, FACET_TOL_MM), 32);
        assert_eq!(snout_segments(100.0, 25.0, FACET_TOL_MM), 32);
        assert_eq!(torus_ring_segments(250.0, FACET_TOL_MM, 90.0), 13);
        assert_eq!(torus_ring_segments(250.0, FACET_TOL_MM, 360.0), 52);
        assert_eq!(circular_torus_tube_segments(50.0, 250.0, FACET_TOL_MM), 32);

        let f = elliptical_dish_facets(1000.0, 250.0, FACET_TOL_MM).expect("合法尺寸");
        assert_eq!((f.around, f.hub, f.knuckle), (100, 8, 9));
        let s = spherical_dish_facets(100.0, 25.0, FACET_TOL_MM).expect("浅碟合法");
        assert_eq!((s.around, s.meridional), (32, 3));
    }
}
