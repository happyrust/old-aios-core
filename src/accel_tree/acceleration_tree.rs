use crate::geometry::PlantGeoData;
use crate::shape::pdms_shape::PlantMesh;
use crate::{GeomInstQuery, SUL_DB, types::*};
use approx::{AbsDiffEq, abs_diff_ne, assert_abs_diff_eq};
use bevy_ecs::prelude::Resource;
use dashmap::DashMap;
use dashmap::mapref::one::Ref;
use glam::{Mat4, Vec3};
use parry3d::bounding_volume::{Aabb, BoundingVolume};
use parry3d::query::{Ray, RayCast, RayIntersection};
use parry3d::shape::TriMesh;
use parry3d::shape::TriMeshFlags;
use rstar::Envelope;
use serde_derive::{Deserialize, Serialize};
use serde_with::{As, FromInto, serde_as};
use smallvec::SmallVec;
use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Write};
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RStarBoundingBox {
    pub aabb: Aabb,
    #[serde(serialize_with = "RefU64::serialize_as_u64")]
    #[serde(deserialize_with = "RefU64::deserialize_from_u64")]
    pub refno: RefU64,
    //方便过滤
    pub noun: String,
}

impl RStarBoundingBox {
    pub fn new(aabb: Aabb, refno: RefnoEnum, noun: String) -> Self {
        Self {
            aabb,
            refno: refno.refno(),
            noun,
        }
    }

    pub fn from_aabb(aabb: Aabb, refno: RefnoEnum) -> Self {
        Self {
            aabb,
            refno: refno.refno(),
            noun: "UNSET".to_string(),
        }
    }

    pub fn from_min_max(min: Vec3, max: Vec3, transform: Mat4, refno: RefnoEnum) -> Self {
        let min = transform.transform_point3(min);
        let max = transform.transform_point3(max);

        Self {
            aabb: Aabb::new(min.into(), max.into()),
            refno: refno.refno(),
            noun: "UNSET".to_string(),
        }
    }
}

impl rstar::RTreeObject for RStarBoundingBox {
    type Envelope = rstar::AABB<[f32; 3]>;

    fn envelope(&self) -> Self::Envelope {
        rstar::AABB::from_corners(self.aabb.mins.into(), self.aabb.maxs.into())
    }
}

impl rstar::PointDistance for RStarBoundingBox {
    fn distance_2(&self, point: &[f32; 3]) -> f32 {
        let aabb = rstar::AABB::from_corners(self.aabb.mins.into(), self.aabb.maxs.into());
        aabb.distance_2(point)
    }
}

#[serde_as]
#[derive(Clone, Default, Serialize, Deserialize, Resource)]
pub struct AccelerationTree {
    pub tree: rstar::RTree<RStarBoundingBox>,
    //用来检查是否插入到了 Tree，如果遇到重复的，需要跳过
    #[serde_as(as = "HashSet<FromInto<u64>>")]
    ids: HashSet<RefU64>,
    /// `rstar` 只按空间位置建索引，按 refno 替换时需要这张反向表定位旧盒。
    ///
    /// 它是纯内存派生数据，不能进入 `accel_tree.bin`，否则会破坏旧缓存兼容性。
    #[serde(skip)]
    refno_index: HashMap<RefU64, SmallVec<[RStarBoundingBox; 1]>>,
    #[serde(skip)]
    indexed_tree_len: usize,
    #[serde(skip)]
    mesh_cache: DashMap<RefnoEnum, Vec<TriMesh>>,
}

impl Deref for AccelerationTree {
    type Target = rstar::RTree<RStarBoundingBox>;

    fn deref(&self) -> &Self::Target {
        &self.tree
    }
}

impl DerefMut for AccelerationTree {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // 通过 DerefMut 修改 RTree 的调用方绕过了反向索引，下一次按 refno 操作前重建。
        self.indexed_tree_len = usize::MAX;
        &mut self.tree
    }
}

/// [`AccelerationTree::query_nearest_by_ray`] 打射线前把起点沿反方向退的距离（模型单位 mm）。
pub const RAY_BACKOFF_MM: f32 = 1.0;

pub struct QueryRay {
    pub ray: Ray,
    pub filter_nouns: HashSet<String>,
    //需要使用当前的房间的距离，这样可以缩小范围
    pub toi: f32,
    pub solid: bool,
    pub min_dist: Cell<f32>,
    /// `should_unpack_leaf` 只拿得到 `&self`，收集结果必须走内部可变性。
    ///
    /// 这里曾经写成 `HashSet`，而填充那两行用的是 `borrow_mut()`——那是对
    /// `RefCell` 的写法，套在 `HashSet` 上编译不过，于是整段被注释掉，
    /// `min_refnos` 从此恒为空。`query_nearest_by_ray` 开头的
    /// `if refnos.is_empty() { return Ok(None) }` 因此每次都命中，
    /// **射线查询永远返回 None**——树搜索和几何求交都正常，只是结果没被存下来。
    pub min_refnos: RefCell<HashSet<RefnoEnum>>,
    /// 候选的合并盒，随收集一并记下。
    ///
    /// 精筛阶段的嵌入判定要用候选自己的盒，但**不能**回头查 `entries_of`：快照装载
    /// （`bincode::deserialize`）后 `refno_index` 是空的（`#[serde(skip)]`），
    /// `entries_of` 会退化成遍历整棵树。1165 台设备 × 几十个候选 × 31 万条目，
    /// 真库实测直接把全量重算从几十秒拖到 15 分钟以上。
    ///
    /// 收集阶段本来就把每条命中的 `RStarBoundingBox` 拿在手上，顺手合并成本为零。
    pub min_aabbs: RefCell<HashMap<RefnoEnum, Aabb>>,
}

impl QueryRay {
    pub fn new(ray: Ray, filter_nouns: HashSet<String>, solid: bool) -> Self {
        Self {
            ray,
            filter_nouns,
            toi: 10_0000.0,
            // found: Cell::new(false),
            min_dist: Cell::new(f32::MAX),
            min_refnos: RefCell::new(HashSet::default()),
            min_aabbs: RefCell::new(HashMap::default()),
            solid,
        }
    }
}

/// 沿 `dir` 从 `origin` 到 AABB 出射面的距离（射线方向上的剩余盒厚）。`dir` 为 0 的轴不参与。
fn aabb_exit_distance(aabb: &Aabb, origin: glam::Vec3, dir: glam::Vec3) -> f32 {
    let mut exit = f32::MAX;
    let axes = [
        (origin.x, dir.x, aabb.mins.x, aabb.maxs.x),
        (origin.y, dir.y, aabb.mins.y, aabb.maxs.y),
        (origin.z, dir.z, aabb.mins.z, aabb.maxs.z),
    ];
    for (o, d, min, max) in axes {
        if d > 0.0 {
            exit = exit.min((max - o) / d);
        } else if d < 0.0 {
            exit = exit.min((min - o) / d);
        }
    }
    exit
}

/// 命中距离修正：起点落在候选构件 AABB 内、且命中点就在该盒的**出射面**上时，说明射线
/// 是穿过实体打到远端的（设备支腿嵌进楼板、或底面正压在板厚范围里），算出来的 `toi` 是
/// 「盒厚 − 嵌入深度」，不是净距——按贴面 0 算。
///
/// `RAY_BACKOFF_MM = 1.0` 只吸收 ≤1mm 的微嵌：嵌入更深时退 1mm 后起点仍在板内，`-Z`
/// 射线会命中楼板**底面**，算出「板厚 − 嵌入深度」。真库这类支腿插进楼板的设备会连到
/// 明显偏大的假距离上。
///
/// 判据用「命中点 ≈ AABB 出射面」而不是命中面法向：parry 的
/// `cast_local_ray_and_get_normal` 返回的法向已被翻到**面向射线来向**（`normal·dir < 0`
/// 恒成立），拿它判外法向恒为假。出射面判据同时挡住另一种误判——斜置楼板的 AABB 会
/// 膨胀，起点可能落在膨胀盒内却仍在实体外，那时 `toi` 明显小于出射距离，保留真实净距。
///
/// `origin` 必须是**未后退**的原始起点：退过之后起点可能已经挪进盒内，会把「本来贴在
/// 板面上方 0.5mm」也当成内部穿越。`backoff` 单独给出，只用于把退出去的那段减回来。
fn corrected_toi(
    intersection: &RayIntersection,
    origin: glam::Vec3,
    aabb: &Aabb,
    dir: glam::Vec3,
    backoff: f32,
) -> f32 {
    let raw = (intersection.time_of_impact - backoff).max(0.0);
    let inside = origin.x >= aabb.mins.x
        && origin.x <= aabb.maxs.x
        && origin.y >= aabb.mins.y
        && origin.y <= aabb.maxs.y
        && origin.z >= aabb.mins.z
        && origin.z <= aabb.maxs.z;
    if !inside {
        return raw;
    }
    let exit = aabb_exit_distance(aabb, origin, dir);
    // 容差与回退量同量级：出射面判定本身有浮点误差，但远小于任何真实净距。
    if raw + RAY_BACKOFF_MM >= exit {
        0.0
    } else {
        raw
    }
}


impl rstar::SelectionFunction<RStarBoundingBox> for &QueryRay {
    //如果找到了最近的，应该停止继续搜索
    fn should_unpack_parent(&self, envelope: &rstar::AABB<[f32; 3]>) -> bool {
        use parry3d::{math::*, query::*};
        //如果已经找到了，就可以返回 false
        // if self.found.get() {
        //     return false;
        // }
        let bbox = Aabb::new(envelope.lower().into(), envelope.upper().into());
        // dbg!(&bbox);
        bbox.intersects_ray(&Isometry::identity(), &self.ray, self.toi)
    }

    fn should_unpack_leaf(&self, bbox: &RStarBoundingBox) -> bool {
        use parry3d::{math::*, query::*};
        if !self.filter_nouns.is_empty() && self.filter_nouns.contains(&bbox.noun) {
            let inter = bbox.aabb.cast_ray_and_get_normal(
                &Isometry::identity(),
                &self.ray,
                self.toi,
                self.solid,
            );
            // 2026-11-01 修：这里原来有 `&& ray_inter.time_of_impact <= self.min_dist.get()`
            // 并用 `min_dist` 做「更近就替换」的裁剪，两处都是错的：
            //
            // 1. AABB 是外接盒，它的 `time_of_impact` 是真实网格 toi 的**下界**。
            //    拿它当「最近」比较，会把 AABB 靠前但网格在该 (x,y) 处镂空的构件
            //    判为赢家（楼板有孔洞、开口、异形轮廓都算）。
            // 2. 裁剪条件让后续候选连进候选集的机会都没有。真库实测
            //    （起点 (-9276.84, 10547.79, 1370) 向 -Z 打）：AABB 命中 8 个 FLOOR，
            //    toi 依次 370 / 430 / 1390 / 1390 / 4570 / 4570 / 8870 / 8870。
            //    toi=370 的 24381_39652（z=[940,1000]）网格没命中，而 toi=430 的
            //    17496_98660（z=[-400,940]）才是真正该命中的那个——它被 370 剪掉了，
            //    整条查询返回 None。往下 2000mm 重新打才命中，正是同一批里的候选。
            //
            // 语义改成「AABB 阶段命中的全部候选」，最近由 `query_nearest_by_ray`
            // 的三角网格精筛决定。候选数量由 `toi` 上限控制，实测同一射线 8 个量级。
            if let Some(ray_inter) = inter {
                let toi = ray_inter.time_of_impact;
                if toi < self.min_dist.get() {
                    self.min_dist.set(toi);
                }
                let refno: RefnoEnum = bbox.refno.into();
                self.min_refnos.borrow_mut().insert(refno);
                self.min_aabbs
                    .borrow_mut()
                    .entry(refno)
                    .and_modify(|merged| merged.merge(&bbox.aabb))
                    .or_insert(bbox.aabb);
                return true;
            }
        }
        return false;
    }
}

impl AccelerationTree {
    #[inline]
    pub fn size(&self) -> usize {
        self.tree.size()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.tree.size() == 0
    }

    /// 加载包围盒
    pub fn load(bounding_boxes: Vec<RStarBoundingBox>) -> Self {
        let mut loaded = Self {
            tree: rstar::RTree::bulk_load(bounding_boxes),
            ..Default::default()
        };
        loaded.rebuild_refno_index();
        loaded
    }

    fn rebuild_refno_index(&mut self) {
        let mut refno_index: HashMap<RefU64, SmallVec<[RStarBoundingBox; 1]>> =
            HashMap::with_capacity(self.tree.size());
        let mut ids = HashSet::with_capacity(self.tree.size());
        for bbox in self.tree.iter() {
            ids.insert(bbox.refno);
            refno_index
                .entry(bbox.refno)
                .or_default()
                .push(bbox.clone());
        }
        self.ids = ids;
        self.refno_index = refno_index;
        self.indexed_tree_len = self.tree.size();
    }

    #[inline]
    fn ensure_refno_index(&mut self) {
        // ponytail: `tree` 暂时保持 public 以兼容下游；长度变化和 DerefMut 都会触发重建。
        // 若出现绕过 API 的等量替换，再把字段收私有并只暴露只读访问。
        if self.indexed_tree_len != self.tree.size() {
            self.rebuild_refno_index();
        }
    }

    /// 用一批新包围盒同步树，返回被摘掉的旧条目。
    ///
    /// 语义是「这些 refno 的现状就是这批新盒子」：先一次遍历收集这些 refno 现存的
    /// **全部**旧条目（`rstar` 的 `remove` 要求整值相等，只有先拿到旧值才删得中），
    /// 逐条摘除后再插入新条目，`ids` 同步维护。历史上 [`Self::update_aabbs`] 的
    /// 去重条件写反（`ids.insert` 返回 true 是「首次见到」，却被当成「已存在」去
    /// remove，而且拿的是新值，永远删不中），同一 refno 会在树里堆叠历史包围盒；
    /// 这里把删旧插新收成一个正确的原语。
    ///
    /// 返回值是新旧比对的依据：调用方拿它判断「包围盒是否真的变了」（房间增量的
    /// 触发源）。没有旧条目的 refno 不在返回值里，即「树上首次见到」。
    pub fn sync_refnos(&mut self, bboxes: Vec<RStarBoundingBox>) -> Vec<RStarBoundingBox> {
        if bboxes.is_empty() {
            return Vec::new();
        }
        self.ensure_refno_index();
        let refnos: HashSet<RefU64> = bboxes.iter().map(|bbox| bbox.refno).collect();
        let mut stale = Vec::new();
        for refno in &refnos {
            if let Some(old_boxes) = self.refno_index.remove(refno) {
                for old in &old_boxes {
                    self.tree.remove(old);
                }
                stale.extend(old_boxes);
            }
            self.ids.remove(refno);
        }
        self.evict_meshes(&refnos);
        for bbox in bboxes {
            self.ids.insert(bbox.refno);
            self.tree.insert(bbox.clone());
            self.refno_index.entry(bbox.refno).or_default().push(bbox);
        }
        self.indexed_tree_len = self.tree.size();
        stale
    }

    /// 新加数据（不关心旧值的调用方用这个；需要新旧比对用 [`Self::sync_refnos`]）。
    pub fn update_aabbs(&mut self, bboxes: Vec<RStarBoundingBox>) {
        let _ = self.sync_refnos(bboxes);
    }

    pub fn replace(&mut self, bounding_boxes: Vec<RStarBoundingBox>) {
        self.tree = rstar::RTree::bulk_load(bounding_boxes);
        self.rebuild_refno_index();
    }

    /// 按 refno 移除条目，返回实际移除的条数。
    ///
    /// `rstar` 的 `remove` 要求按整值相等匹配，也就是调用方得先拿到那条**旧的**包围盒。
    /// 删除路径拿不到：元素连同它的 `inst_relate.aabb` 一起没了。留在树里的话，
    /// `locate_intersecting_bounds` 会继续把它当候选返回，房间归属就会把一个已经不存在
    /// 的构件算进某间房。
    ///
    /// `ids` 也要一并清掉，否则这个 refno 之后再次入树时会被 [`Self::update_aabbs`]
    /// 的去重分支当成「已存在」。
    pub fn remove_by_refnos(&mut self, refnos: &HashSet<RefU64>) -> usize {
        if refnos.is_empty() {
            return 0;
        }
        self.ensure_refno_index();
        let mut removed = 0;
        for refno in refnos {
            if let Some(old_boxes) = self.refno_index.remove(refno) {
                removed += old_boxes.len();
                for bbox in old_boxes {
                    self.tree.remove(&bbox);
                }
            }
            self.ids.remove(refno);
        }
        self.evict_meshes(refnos);
        self.indexed_tree_len = self.tree.size();
        removed
    }

    /// 摘掉这些 refno 的世界坐标网格缓存。
    ///
    /// `mesh_cache` 按 refno 缓存 [`Self::get_tri_mesh`] 读出的网格。增量刷新空间树只走
    /// [`Self::sync_refnos`] / [`Self::remove_by_refnos`]，这两处以前不清缓存：进程里重生成过
    /// 的构件（楼板被改），射线就一直打在旧网格上，增量重算的最近楼板仍是旧结果。
    ///
    /// 缓存键是 `RefnoEnum`（可能带会话号），树只认 `RefU64`，所以按 `.refno()` 匹配全部变体。
    fn evict_meshes(&self, refnos: &HashSet<RefU64>) {
        if refnos.is_empty() || self.mesh_cache.is_empty() {
            return;
        }
        self.mesh_cache
            .retain(|key, _| !refnos.contains(&key.refno()));
    }

    /// 某个 refno 在树上的全部条目：正常恰好一条，历史堆叠的残留可能多条，没有则空。
    ///
    /// 走 `refno_index` 的只读查询（`&self`，不触发懒重建）。索引在 [`Self::load`] /
    /// [`Self::replace`] / [`Self::sync_refnos`] / [`Self::remove_by_refnos`] /
    /// [`Self::deserialize_from_bin_file`] 之后恒新鲜；两种情况下它是过期的——直接
    /// `bincode::deserialize` 出来的树（`#[serde(skip)]` 让索引落成空表，下游的快照装载
    /// 正是这条路）与经 `DerefMut` / pub `tree` 绕过 API 的改动。过期时退化为整树线性扫描：
    /// 结果仍然正确，只是 O(n)。这里**不** `debug_assert!`——快照复用启动后到第一次写树
    /// 之前索引一直是过期的，断言会让 debug 构建的服务在第一条按 refno 的查询上崩掉；
    /// 写路径下一次 [`Self::ensure_refno_index`] 会把它治愈。
    pub fn entries_of(&self, refno: RefU64) -> Cow<'_, [RStarBoundingBox]> {
        if self.indexed_tree_len == self.tree.size() {
            return match self.refno_index.get(&refno) {
                Some(entries) => Cow::Borrowed(entries.as_slice()),
                None => Cow::Borrowed(&[]),
            };
        }
        Cow::Owned(
            self.tree
                .iter()
                .filter(|bbox| bbox.refno == refno)
                .cloned()
                .collect(),
        )
    }

    pub fn query_within_distance<'a>(
        &'a self,
        loc: Vec3,
        distance: f32,
    ) -> impl Iterator<Item = (RefnoEnum, Aabb)> + 'a {
        self.tree
            .locate_within_distance([loc.x, loc.y, loc.z], distance.powi(2))
            .map(|bb| (bb.refno.into(), bb.aabb))
    }

    pub fn locate_intersecting_bounds<'a>(
        &'a self,
        bounds: &Aabb,
    ) -> impl Iterator<Item = &RStarBoundingBox> + 'a {
        self.tree
            .locate_in_envelope_intersecting(&rstar::AABB::from_corners(
                [bounds.mins[0], bounds.mins[1], bounds.mins[2]],
                [bounds.maxs[0], bounds.maxs[1], bounds.maxs[2]],
            ))
            .map(|bb| bb)
    }

    /// 检查是否包含包围盒
    pub fn locate_contain_bounds<'a>(
        &'a self,
        bounds: &Aabb,
    ) -> impl Iterator<Item = &RStarBoundingBox> + 'a {
        self.tree
            .locate_in_envelope(&rstar::AABB::from_corners(
                [bounds.mins[0], bounds.mins[1], bounds.mins[2]],
                [bounds.maxs[0], bounds.maxs[1], bounds.maxs[2]],
            ))
            .map(|bb| bb)
    }

    //实现使用bincode序列化
    /// 先写临时文件再原子 rename 覆盖：这个文件由空闲轮反复重写（17MB 量级），
    /// 原地 `File::create` 重写意味着每次落盘都有一个「写半截崩溃 → 文件损坏」的
    /// 窗口，而损坏文件会让下次启动的加载失败。std 的 `rename` 在 Windows 上带
    /// REPLACE_EXISTING 语义，读者要么看到旧文件要么看到新文件，没有半截。
    #[cfg(not(target_arch = "wasm32"))]
    pub fn serialize_to_bin_file(&self) -> anyhow::Result<bool> {
        let tmp_path = "accel_tree.bin.tmp";
        let serialized = bincode::serialize(&self)?;
        {
            let mut file = File::create(tmp_path)?;
            file.write_all(serialized.as_slice())?;
            file.sync_all()?;
        }
        std::fs::rename(tmp_path, "accel_tree.bin")?;
        Ok(true)
    }

    /// 使用bincode反序列化
    ///
    /// 损坏的文件（历史版本原地重写留下的半截）必须走 `Err` 让调用方降级重建，
    /// 不能 panic——这里在启动路径上，panic 等于 crash loop 到有人手删文件为止。
    #[cfg(not(target_arch = "wasm32"))]
    pub fn deserialize_from_bin_file() -> anyhow::Result<Self> {
        let mut file = File::open("accel_tree.bin")?;
        let mut buf: Vec<u8> = Vec::new();
        let _ = file.read_to_end(&mut buf)?;
        let mut r: Self = bincode::deserialize(&buf)?;
        r.rebuild_refno_index();
        Ok(r)
    }

    /// 获取一个refno的mesh
    /// 如果mesh_cache中没有，则从数据库中加载
    /// 如果数据库中也没有，则返回None
    pub async fn get_tri_mesh(&self, refno: RefnoEnum) -> Option<Ref<RefnoEnum, Vec<TriMesh>>> {
        if let Some(r) = self.mesh_cache.get(&refno) {
            return Some(r);
        }
        let geom_insts = match crate::query_insts(&[refno], true).await {
            Ok(v) => v,
            Err(error) => {
                log::warn!("射线精筛：查询几何实例失败 refno={refno:?}: {error}");
                return None;
            }
        };
        let mut meshes = vec![];
        for g in geom_insts {
            for inst in &g.insts {
                let path = format!("assets/meshes/{}.mesh", inst.geo_hash);
                let mesh = match PlantMesh::des_mesh_file(&path) {
                    Ok(mesh) => mesh,
                    Err(error) => {
                        log::warn!("射线精筛：网格文件读取失败 refno={refno:?} path={path}: {error}");
                        continue;
                    }
                };
                if mesh.vertices.is_empty() {
                    log::debug!("射线精筛：网格无顶点 refno={refno:?} path={path}");
                    continue;
                }
                let trans = g.world_trans * inst.transform;
                let Some(tri_mesh) = mesh.get_tri_mesh(trans.compute_matrix()) else {
                    log::warn!("射线精筛：三角网格构建失败（索引不足 3） refno={refno:?} path={path}");
                    continue;
                };
                meshes.push(tri_mesh);
            }
        }
        // 空结果不进缓存：读失败 / 几何尚未落盘时缓存空列表，这个构件在本进程里
        // 就再也不会被重试，射线永远打不中它。
        if meshes.is_empty() {
            return None;
        }
        self.mesh_cache.insert(refno, meshes);
        self.mesh_cache.get(&refno)
    }

    /// Returns the refno of the nearest object to `query_point`
    /// 也可以检查墙等等，需要判断mesh
    ///
    /// 查询分两阶段：AABB 阶段把射线穿过的候选**全部**收进 `min_refnos`，不做
    /// 距离裁剪；谁最近由下面的三角网格精筛逐个求交决定。AABB 的命中 toi 只是
    /// 真实网格 toi 的下界（楼板在射线 (x,y) 处可能是孔洞），不能拿它在收集
    /// 阶段排序取舍。`QueryRay` 上的 `min_dist` 只是诊断用的记录值，不参与任何
    /// 拒绝判断；父节点要不要下钻用的是 `QueryRay::new` 定死的 `toi` 常量，不会
    /// 被 `min_dist` 收窄——即不存在「先命中的 AABB 剪掉后来的候选」这条路径，
    /// 不要再往收集阶段加距离裁剪。
    ///
    /// 由此，返回 `None` 的语义是「AABB 候选全部进了精筛但网格无一命中」，而非
    /// 查询漏掉候选。真库 2026-11-02 实测：阀门 `pe:24381_177593` 正下方射线收进
    /// 3 个楼板候选，492 / 696 个三角形无一命中（该 (x,y) 处网格全镂空），返回
    /// `None` 是正确结果。
    pub async fn query_nearest_by_ray(
        &self,
        mut ray: QueryRay,
    ) -> anyhow::Result<Option<(RefnoEnum, f32)>> {
        // 2026-11-01 修：起点先沿反方向退 `RAY_BACKOFF_MM` 再打，命中距离再减回去。
        //
        // 设备底面常常正好落在楼板面上，或者略微嵌进楼板（真库 `24381_102435`：底面
        // z=-3320，正是楼板 `24381_38154` z=[-3320,-3200] 的底面）。起点贴在三角面上时
        // parry 的射线求交不算命中；`solid=true` 也救不了——TriMesh 用默认 flag 建，判断
        // 不了「起点在实体内」（探针实测 solid / 非 solid 结果处处相同）。退 1mm 后从面外
        // 打进来就能命中，距离减回去后是 0（贴面）或真实间距。
        let dir_len = ray.ray.dir.norm();
        let backoff = if dir_len > 0.0 {
            RAY_BACKOFF_MM / dir_len
        } else {
            0.0
        };
        // 嵌入判定要用**未后退**的起点：退过之后起点可能已经挪进楼板盒内，
        // 会把「本来贴在板面上方 0.5mm」也当成内部穿越。
        let query_origin = ray.ray.origin;
        ray.ray.origin -= ray.ray.dir * backoff;
        let _ = self
            .tree
            .locate_with_selection_function(&ray)
            .collect::<Vec<_>>();
        // 借出来一份快照再放开：下面 `get_tri_mesh` 是 async，不能跨 await 持着借用。
        let refnos: Vec<RefnoEnum> = ray.min_refnos.borrow().iter().copied().collect();
        if refnos.is_empty() {
            return Ok(None);
        }
        // 候选盒也一并借出来：嵌入判定要用，回头查 `entries_of` 在快照装载的进程里
        // 会退化成遍历整棵树（见 `min_aabbs` 的说明）。
        let candidate_aabbs: HashMap<RefnoEnum, Aabb> = ray.min_aabbs.borrow().clone();
        //检查是否真的和ray相交, 根据 profile 判断吗？
        //根据 param 判断是否相交吗？
        //直接检查是否在表面上即可
        //
        // 2026-11-01 修：候选集现在是「AABB 阶段命中的全部」，所以这里必须真的比出
        // 最近的网格，不能遇到第一个命中的就返回——那等于又退回按遍历顺序取，而
        // rstar 的遍历顺序不保证由近到远。
        //
        // 2026-11-02 修：距离相等时按 refno 定序。`min_refnos` 是 `HashSet`，迭代顺序
        // 逐次运行都不同；同 z 的多块楼板（同一层被切成的几块）距离常常恰好相等，
        // 不定序时同一台设备两次重算会连到不同的楼板上（真库实测
        // `24381_100725` 在 `24381_38695` 与 `24381_39170` 之间来回跳）。
        let mut refnos = refnos;
        refnos.sort_unstable();
        let mut best: Option<(RefnoEnum, f32)> = None;
        for &refno in refnos.iter() {
            let Some(tri_meshes) = self.get_tri_mesh(refno).await else {
                continue;
            };
            // 嵌入判定要拿构件自己的盒：起点在盒内、命中点是出射面时，说明起点已经在
            // 实体里（支腿插进楼板），算出来的 toi 是「盒厚 − 嵌入深度」。
            let aabb = candidate_aabbs.get(&refno).copied();
            let origin = glam::Vec3::new(query_origin.x, query_origin.y, query_origin.z);
            let dir = glam::Vec3::new(ray.ray.dir.x, ray.ray.dir.y, ray.ray.dir.z);
            for mesh in tri_meshes.value() {
                let Some(intersection) =
                    mesh.cast_local_ray_and_get_normal(&ray.ray, 10_0000.0, ray.solid)
                else {
                    continue;
                };
                // 减回退出去的那一段；起点本来就在面后不到 1mm 时会略小于 0，按贴面算。
                let toi = match aabb {
                    Some(ref aabb) => corrected_toi(&intersection, origin, aabb, dir, backoff),
                    None => (intersection.time_of_impact - backoff).max(0.0),
                };
                if best.is_none_or(|(winner, d)| toi < d || (toi == d && refno < winner)) {
                    best = Some((refno, toi));
                }
            }
        }
        return Ok(best);
    }

    /// 一批点同时向 `-Z` 打射线，一次包络粗筛后逐点精筛，返回每点各自的最近目标。
    ///
    /// 设备底面探测要打几十到几百条同向射线（4 角 + 底面中心，逐子件）。逐条调
    /// [`Self::query_nearest_by_ray`] 意味着每条一次 R-tree 遍历；这里改成一次
    /// `locate_in_envelope_intersecting`：xy 取全部点的包围盒、z 从世界底到最高点，
    /// 该包络是所有 `-Z` 射线的超集（`-Z` 射线的 xy 恒定），不会漏候选。
    ///
    /// 返回与 `points` 等长的向量，每格是该点最近的 `(refno, 距离)`；`None` 表示该点
    /// 正下方没有网格命中（脚下确实没目标，或网格在该 (x,y) 处镂空）。
    pub async fn query_nearest_below(
        &self,
        points: &[glam::Vec3],
        filter_nouns: &HashSet<String>,
    ) -> anyhow::Result<Vec<Option<(RefnoEnum, f32)>>> {
        self.query_nearest_below_bounded(points, filter_nouns, None)
            .await
    }

    /// [`Self::query_nearest_below`] 带初始上界：只报**严格优于** `bound` 的命中。
    ///
    /// `bound` 是调用方已经拿到的候选距离（例如合并盒四角的结果）。传进来有两个作用：
    ///
    /// 1. **剪枝**：每个点一开始就有 `best = bound`，下界剪枝立刻生效，不必先扫完一圈
    ///    才建立最优值。设备底面探测里绝大多数子件点比合并盒那一级更远，这一层能把
    ///    无效网格求交砍掉大半。
    /// 2. **语义**：返回 `Some` 就是「这一点比调用方已有的更近」，返回 `None` 就是
    ///    「没有改进」，调用方不必再自己比一遍。
    ///
    /// 精筛有三层剪枝，缺一层都会让设备全量重算从几十秒涨到几分钟：
    ///
    /// 1. **xy 剪枝**：`-Z` 射线的 xy 恒定，候选 AABB 不含该 (x,y) 就必然打不中。
    /// 2. **上下剪枝**：候选整体在点之上（`mins.z > pt.z`）打不中；候选顶面已在
    ///    「当前最优距离」之下（`maxs.z <= pt.z - best`）也不可能更近。候选按顶面 z
    ///    降序排好，后一条一旦成立，后面的全部更低，直接 `break`。
    /// 3. **同距定序**：距离相等时取 refno 小的，保证同一台设备重跑连到同一块楼板。
    pub async fn query_nearest_below_bounded(
        &self,
        points: &[glam::Vec3],
        filter_nouns: &HashSet<String>,
        bound: Option<(RefnoEnum, f32)>,
    ) -> anyhow::Result<Vec<Option<(RefnoEnum, f32)>>> {
        let mut hits = vec![None; points.len()];
        if points.is_empty() || filter_nouns.is_empty() {
            return Ok(hits);
        }
        let mut lo = glam::Vec3::splat(f32::MAX);
        let mut hi = glam::Vec3::splat(f32::MIN);
        for pt in points {
            lo = lo.min(*pt);
            hi = hi.max(*pt);
        }
        let envelope = Aabb::new(
            parry3d::math::Point::new(lo.x, lo.y, f32::MIN),
            parry3d::math::Point::new(hi.x, hi.y, hi.z),
        );
        // 候选按顶面 z 降序：精筛的下界剪枝依赖这个顺序。
        let mut candidates: Vec<(RefnoEnum, Aabb)> = Vec::new();
        let mut merged: HashMap<RefnoEnum, Aabb> = HashMap::new();
        for entry in self.locate_intersecting_bounds(&envelope) {
            if !filter_nouns.contains(&entry.noun) {
                continue;
            }
            let refno = RefnoEnum::from(entry.refno);
            // 同一 refno 的多条条目（历史堆叠）合并成一个盒。
            merged
                .entry(refno)
                .and_modify(|box_| box_.merge(&entry.aabb))
                .or_insert(entry.aabb);
        }
        for (refno, aabb) in merged {
            candidates.push((refno, aabb));
        }
        if candidates.is_empty() {
            return Ok(hits);
        }
        candidates.sort_by(|a, b| b.1.maxs.z.total_cmp(&a.1.maxs.z));

        for (index, pt) in points.iter().enumerate() {
            let dir = glam::Vec3::NEG_Z;
            // 与 `query_nearest_by_ray` 同口径：起点先沿反方向退 `RAY_BACKOFF_MM`，
            // 命中距离再减回去，贴面才算得到 0。
            let origin = *pt - dir * RAY_BACKOFF_MM;
            let ray = Ray::new(origin.into(), dir.into());
            let mut best = bound;
            for (refno, aabb) in &candidates {
                // 整体在点之上：-Z 射线打不中。
                if aabb.mins.z > pt.z {
                    continue;
                }
                if let Some((winner, d)) = best {
                    // 顶面已在「当前最优」之下：不可能更近，后面的候选只会更低。
                    // 严格不等号保住了同距时按 refno 定序的那条。
                    if aabb.maxs.z < pt.z - d || (aabb.maxs.z == pt.z - d && *refno > winner) {
                        break;
                    }
                }
                // xy 不含该点：竖直射线必然打不中，连网格都不用取。
                if pt.x < aabb.mins.x
                    || pt.x > aabb.maxs.x
                    || pt.y < aabb.mins.y
                    || pt.y > aabb.maxs.y
                {
                    continue;
                }
                let Some(tri_meshes) = self.get_tri_mesh(*refno).await else {
                    continue;
                };
                for mesh in tri_meshes.value() {
                    let Some(intersection) =
                        mesh.cast_local_ray_and_get_normal(&ray, 10_0000.0, true)
                    else {
                        continue;
                    };
                    let toi = corrected_toi(&intersection, origin, aabb, dir, RAY_BACKOFF_MM);
                    if best.is_none_or(|(winner, d)| toi < d || (toi == d && *refno < winner)) {
                        best = Some((*refno, toi));
                    }
                }
            }
            // 只报改进：等于上界的不算（调用方已经有了）。
            hits[index] = match (best, bound) {
                (Some(found), Some(initial)) => {
                    (found.1 < initial.1 || (found.1 == initial.1 && found.0 < initial.0))
                        .then_some(found)
                }
                (found, None) => found,
                (None, Some(_)) => None,
            };
        }
        Ok(hits)
    }

    /// 某个 refno 在树上的全部条目的合并盒；不在树上时为 `None`。
    ///
    /// **不要在热路径里用**：快照装载（`bincode::deserialize`）后 `refno_index` 是空的，
    /// `entries_of` 会退化成遍历整棵树。射线与批量查询都改从收集阶段顺手取盒
    /// （`QueryRay::min_aabbs` / `locate_intersecting_bounds`），不再走这里。
    #[allow(dead_code)]
    fn merged_aabb(&self, refno: RefnoEnum) -> Option<Aabb> {
        let entries = self.entries_of(refno.refno());
        if entries.is_empty() {
            return None;
        }
        let mut merged = Aabb::new_invalid();
        for entry in entries.iter() {
            merged.merge(&entry.aabb);
        }
        Some(merged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[serde_as]
    #[derive(Serialize)]
    struct LegacyAccelerationTree {
        tree: rstar::RTree<RStarBoundingBox>,
        #[serde_as(as = "HashSet<FromInto<u64>>")]
        ids: HashSet<RefU64>,
    }

    fn bbox(refno_seq: u64, min: f32, max: f32) -> RStarBoundingBox {
        RStarBoundingBox {
            aabb: Aabb::new(
                parry3d::math::Point::new(min, min, min),
                parry3d::math::Point::new(max, max, max),
            ),
            refno: RefU64((4000000001u64 << 32) | refno_seq),
            noun: "BOX".to_string(),
        }
    }

    /// 指定 noun 和任意轴向范围的包围盒，用于射线相关用例。
    fn bbox_at(noun: &str, refno_seq: u64, mins: [f32; 3], maxs: [f32; 3]) -> RStarBoundingBox {
        RStarBoundingBox {
            aabb: Aabb::new(
                parry3d::math::Point::new(mins[0], mins[1], mins[2]),
                parry3d::math::Point::new(maxs[0], maxs[1], maxs[2]),
            ),
            refno: RefU64((4000000001u64 << 32) | refno_seq),
            noun: noun.to_string(),
        }
    }

    fn repeated_sync_elapsed(tree_size: u64, iterations: u64) -> std::time::Duration {
        let mut tree = AccelerationTree::load(
            (1..=tree_size)
                .map(|seq| bbox(seq, seq as f32, seq as f32 + 1.0))
                .collect(),
        );
        let target = tree_size / 2;
        let started = std::time::Instant::now();
        for step in 0..iterations {
            let min = (tree_size + step + 1) as f32;
            let stale = tree.sync_refnos(vec![bbox(target, min, min + 1.0)]);
            assert_eq!(stale.len(), 1);
            std::hint::black_box(stale);
        }
        started.elapsed()
    }

    /// 单个 refno 的刷新成本不应随整棵树规模线性增长。
    #[test]
    fn sync_refnos_cost_is_not_proportional_to_tree_size() {
        let small = repeated_sync_elapsed(2_000, 100);
        let large = repeated_sync_elapsed(100_000, 100);

        assert!(
            large <= small.saturating_mul(12) + std::time::Duration::from_millis(5),
            "单 refno 同步仍疑似扫描整树: small={small:?}, large={large:?}"
        );
    }

    /// 同一 refno 再次同步必须替换而不是堆叠：旧的 update_aabbs 去重条件写反
    /// （首次见到才 remove、拿新值删旧值），重复刷新会让树里留下历史包围盒，
    /// 房间重算就会把一个构件同时算进新旧两个位置的房间。
    #[test]
    fn sync_refnos_replaces_instead_of_stacking() {
        let mut tree = AccelerationTree::default();
        assert!(tree.sync_refnos(vec![bbox(1, 0.0, 10.0)]).is_empty());
        let stale = tree.sync_refnos(vec![bbox(1, 100.0, 110.0)]);

        assert_eq!(tree.size(), 1, "同一 refno 不允许在树里占两条");
        assert_eq!(stale.len(), 1);
        assert_eq!(
            stale[0].aabb,
            bbox(1, 0.0, 10.0).aabb,
            "返回值必须是被替换的旧盒"
        );
    }

    /// 旧值可能有多条（历史堆叠的残留），同步必须一次清干净。
    #[test]
    fn sync_refnos_clears_historic_duplicates() {
        let mut tree = AccelerationTree::default();
        tree.tree.insert(bbox(1, 0.0, 10.0));
        tree.tree.insert(bbox(1, 50.0, 60.0));

        let stale = tree.sync_refnos(vec![bbox(1, 100.0, 110.0)]);
        assert_eq!(stale.len(), 2, "历史重复条目必须全部被摘掉");
        assert_eq!(tree.size(), 1);
    }

    /// update_aabbs 是 sync_refnos 的忽略返回值版本，行为必须一致。
    #[test]
    fn update_aabbs_no_longer_duplicates() {
        let mut tree = AccelerationTree::default();
        tree.update_aabbs(vec![bbox(1, 0.0, 10.0)]);
        tree.update_aabbs(vec![bbox(1, 100.0, 110.0)]);
        assert_eq!(tree.size(), 1);
    }

    #[test]
    fn remove_by_refnos_removes_all_target_entries_only() {
        let mut tree = AccelerationTree::load(vec![
            bbox(1, 0.0, 10.0),
            bbox(1, 20.0, 30.0),
            bbox(2, 40.0, 50.0),
        ]);

        let removed = tree.remove_by_refnos(&HashSet::from([bbox(1, 0.0, 10.0).refno]));

        assert_eq!(removed, 2);
        assert_eq!(tree.size(), 1);
        assert_eq!(tree.iter().next().unwrap().refno, bbox(2, 40.0, 50.0).refno);
    }

    /// 只读按 refno 取盒：同一 refno 的全部条目（含历史堆叠）都要给，别的 refno 一条不混进来；
    /// 写路径之后索引必须仍然准确。
    #[test]
    fn entries_of_returns_all_boxes_of_a_refno_and_nothing_else() {
        let mut tree = AccelerationTree::load(vec![
            bbox(1, 0.0, 10.0),
            bbox(1, 20.0, 30.0),
            bbox(2, 40.0, 50.0),
        ]);

        let mut ones = tree.entries_of(bbox(1, 0.0, 10.0).refno).into_owned();
        ones.sort_by(|a, b| a.aabb.mins.x.total_cmp(&b.aabb.mins.x));
        assert_eq!(ones, vec![bbox(1, 0.0, 10.0), bbox(1, 20.0, 30.0)]);
        assert_eq!(
            tree.entries_of(bbox(2, 40.0, 50.0).refno).as_ref(),
            &[bbox(2, 40.0, 50.0)]
        );
        assert!(tree.entries_of(bbox(3, 0.0, 1.0).refno).is_empty());

        tree.sync_refnos(vec![bbox(1, 100.0, 110.0)]);
        assert_eq!(
            tree.entries_of(bbox(1, 0.0, 10.0).refno).as_ref(),
            &[bbox(1, 100.0, 110.0)],
            "同步之后只剩新盒"
        );
        tree.remove_by_refnos(&HashSet::from([bbox(2, 40.0, 50.0).refno]));
        assert!(tree.entries_of(bbox(2, 40.0, 50.0).refno).is_empty());
        assert_eq!(tree.entries_of(bbox(1, 0.0, 10.0).refno).len(), 1);
    }

    /// 索引过期（直接 bincode 反序列化、绕过 API 直插 `tree`）时 `entries_of` 退化为线性扫描，
    /// 结果仍要正确且不能 panic；写路径治愈索引后又回到索引查找。
    #[test]
    fn entries_of_falls_back_to_a_linear_scan_while_the_index_is_stale() {
        let loaded = AccelerationTree::load(vec![bbox(1, 0.0, 10.0), bbox(2, 20.0, 30.0)]);
        let bytes = bincode::serialize(&loaded).unwrap();
        let mut restored: AccelerationTree = bincode::deserialize(&bytes).unwrap();
        assert_ne!(
            restored.indexed_tree_len,
            restored.tree.size(),
            "直接反序列化的树索引应当是过期的（前提不成立则本用例没在测回退路径）"
        );
        assert_eq!(
            restored.entries_of(bbox(2, 20.0, 30.0).refno).as_ref(),
            &[bbox(2, 20.0, 30.0)]
        );
        assert!(restored.entries_of(bbox(9, 0.0, 1.0).refno).is_empty());

        restored.tree.insert(bbox(3, 40.0, 50.0));
        assert_eq!(
            restored.entries_of(bbox(3, 40.0, 50.0).refno).as_ref(),
            &[bbox(3, 40.0, 50.0)],
            "绕过 API 直插的条目也要在扫描里找到"
        );

        restored.sync_refnos(vec![bbox(1, 100.0, 110.0)]);
        assert_eq!(restored.indexed_tree_len, restored.tree.size(), "写路径治愈索引");
        assert_eq!(
            restored.entries_of(bbox(3, 40.0, 50.0).refno).as_ref(),
            &[bbox(3, 40.0, 50.0)]
        );
        assert_eq!(
            restored.entries_of(bbox(1, 0.0, 10.0).refno).as_ref(),
            &[bbox(1, 100.0, 110.0)]
        );
    }

    #[test]
    fn refno_index_keeps_legacy_bincode_compatible() {
        let current = AccelerationTree::load(vec![bbox(1, 0.0, 10.0), bbox(2, 20.0, 30.0)]);
        let legacy = LegacyAccelerationTree {
            tree: current.tree.clone(),
            ids: current.ids.clone(),
        };
        let legacy_bytes = bincode::serialize(&legacy).unwrap();

        assert_eq!(bincode::serialize(&current).unwrap(), legacy_bytes);

        let mut restored: AccelerationTree = bincode::deserialize(&legacy_bytes).unwrap();
        let stale = restored.sync_refnos(vec![bbox(1, 100.0, 110.0)]);
        assert_eq!(stale.len(), 1);
        assert_eq!(restored.size(), 2);
    }

    /// replace 换掉整棵树时 ids 必须跟着换，否则后续同步的去重判断全部失真。
    #[test]
    fn replace_resets_the_id_set() {
        let mut tree = AccelerationTree::default();
        tree.update_aabbs(vec![bbox(1, 0.0, 10.0)]);
        tree.replace(vec![bbox(2, 0.0, 10.0)]);

        assert_eq!(tree.size(), 1);
        assert!(tree.ids.contains(&bbox(2, 0.0, 10.0).refno));
        assert!(
            !tree.ids.contains(&bbox(1, 0.0, 10.0).refno),
            "replace 之后旧 refno 不该还挂在 ids 里"
        );
    }

    // ---- 网格缓存失效 ----
    //
    // 增量刷新只走 sync_refnos / remove_by_refnos。它们不清 mesh_cache 时，进程里
    // 重生成过的楼板，射线一直打在旧网格上。

    fn cached(tree: &AccelerationTree, refno_seq: u64) -> bool {
        let key: RefnoEnum = bbox(refno_seq, 0.0, 1.0).refno.into();
        tree.mesh_cache.contains_key(&key)
    }

    fn fill_cache(tree: &AccelerationTree, refno_seq: u64) {
        let key: RefnoEnum = bbox(refno_seq, 0.0, 1.0).refno.into();
        tree.mesh_cache.insert(key, Vec::new());
    }

    #[test]
    fn sync_refnos_evicts_the_mesh_cache_of_synced_refnos_only() {
        let mut tree = AccelerationTree::load(vec![bbox(1, 0.0, 10.0), bbox(2, 20.0, 30.0)]);
        fill_cache(&tree, 1);
        fill_cache(&tree, 2);

        tree.sync_refnos(vec![bbox(1, 100.0, 110.0)]);

        assert!(!cached(&tree, 1), "同步过的 refno 必须丢掉旧网格，下次射线重新读");
        assert!(cached(&tree, 2), "没同步的 refno 缓存不能被误清");
    }

    #[test]
    fn remove_by_refnos_evicts_the_mesh_cache() {
        let mut tree = AccelerationTree::load(vec![bbox(1, 0.0, 10.0), bbox(2, 20.0, 30.0)]);
        fill_cache(&tree, 1);
        fill_cache(&tree, 2);

        tree.remove_by_refnos(&HashSet::from([bbox(1, 0.0, 10.0).refno]));

        assert!(!cached(&tree, 1), "删掉的 refno 不能留着网格缓存");
        assert!(cached(&tree, 2));
    }

    // ---- 射线起点贴面 ----

    /// z=0 平面上一块 20x20 的方板（两片三角形），放进树和网格缓存，不碰库。
    fn tree_with_plate_at_z0(refno_seq: u64, aabb_z: [f32; 2]) -> (AccelerationTree, RefnoEnum) {
        let plate = bbox_at(
            "FLOOR",
            refno_seq,
            [-10.0, -10.0, aabb_z[0]],
            [10.0, 10.0, aabb_z[1]],
        );
        let key: RefnoEnum = plate.refno.into();
        let tree = AccelerationTree::load(vec![plate]);
        let p = |x: f32, y: f32| parry3d::math::Point::new(x, y, 0.0);
        let mesh = TriMesh::with_flags(
            vec![p(-10.0, -10.0), p(10.0, -10.0), p(10.0, 10.0), p(-10.0, 10.0)],
            vec![[0, 1, 2], [0, 2, 3]],
            TriMeshFlags::default(),
        );
        tree.mesh_cache.insert(key, vec![mesh]);
        (tree, key)
    }

    /// 起点正好落在楼板面上（设备底面 = 楼板底面，真库 `24381_102435`）：退一点再打
    /// 才命中，距离减回去是 0。
    #[test]
    fn ray_starting_on_a_floor_face_hits_with_zero_distance() {
        let (tree, key) = tree_with_plate_at_z0(1, [0.0, 120.0]);
        let hit = futures::executor::block_on(tree.query_nearest_by_ray(down_ray(0.0, 0.0, 0.0)))
            .unwrap();
        let (refno, dist) = hit.expect("起点贴面必须命中");
        assert_eq!(refno, key);
        assert!(dist.abs() < 1e-3, "贴面距离应为 0，实际 {dist}");
    }

    /// 起点离面有间距时，退出去的那段要减干净，距离不能多出 `RAY_BACKOFF_MM`。
    #[test]
    fn ray_backoff_is_subtracted_from_the_distance() {
        let (tree, key) = tree_with_plate_at_z0(1, [-100.0, 0.0]);
        let hit = futures::executor::block_on(tree.query_nearest_by_ray(down_ray(0.0, 0.0, 100.0)))
            .unwrap();
        let (refno, dist) = hit.expect("正下方 100 处有楼板");
        assert_eq!(refno, key);
        assert!((dist - 100.0).abs() < 1e-3, "距离应为 100，实际 {dist}");
    }

    /// 同距离的两块楼板必须稳定选 refno 小的那个：`min_refnos` 是 `HashSet`，迭代顺序
    /// 逐次运行不同，不定序时同一台设备两次重算会连到不同的楼板上（真库实测
    /// `24381_100725` 在 `24381_38695` 与 `24381_39170` 之间来回跳）。
    #[test]
    fn equidistant_floors_break_the_tie_by_refno() {
        // 两块同 z、同 xy 的楼板：谁先进候选集取决于 HashSet 顺序。
        let lower = bbox_at("FLOOR", 2, [-10.0, -10.0, -100.0], [10.0, 10.0, 0.0]);
        let upper = bbox_at("FLOOR", 3, [-10.0, -10.0, -100.0], [10.0, 10.0, 0.0]);
        let key_lower: RefnoEnum = lower.refno.into();
        let key_upper: RefnoEnum = upper.refno.into();
        let mut tree = AccelerationTree::load(vec![upper.clone(), lower.clone()]);
        let p = |x: f32, y: f32| parry3d::math::Point::new(x, y, 0.0);
        let mesh = TriMesh::with_flags(
            vec![p(-10.0, -10.0), p(10.0, -10.0), p(10.0, 10.0), p(-10.0, 10.0)],
            vec![[0, 1, 2], [0, 2, 3]],
            TriMeshFlags::default(),
        );
        tree.mesh_cache.insert(key_lower, vec![mesh.clone()]);
        tree.mesh_cache.insert(key_upper, vec![mesh]);

        for _ in 0..8 {
            let hit = futures::executor::block_on(
                tree.query_nearest_by_ray(down_ray(0.0, 0.0, 100.0)),
            )
            .unwrap()
            .expect("两块楼板都在正下方");
            assert_eq!(
                hit.0, key_lower,
                "距离相等时必须稳定选 refno 小的那块，不能随 HashSet 顺序变"
            );
        }
    }

    // ---- 射线候选收集 ----
    //
    // `should_unpack_leaf` 只收集候选，真正的最近由 `query_nearest_by_ray` 的
    // 三角网格精筛决定。下面几条钉住收集阶段的语义，防止有人再把它改回
    // 「只留 AABB 最近的那一个」——那会让网格镂空的构件挤掉真实命中。

    /// 沿 -Z 打的一条竖直射线，从 z=100 往下。
    fn down_ray(x: f32, y: f32, z: f32) -> QueryRay {
        let mut filter = HashSet::new();
        filter.insert("FLOOR".to_string());
        QueryRay::new(
            Ray::new(
                parry3d::math::Point::new(x, y, z),
                parry3d::math::Vector::new(0.0, 0.0, -1.0),
            ),
            filter,
            true,
        )
    }

    /// 收集阶段必须把所有 AABB 命中的候选都留下，而不是只留最近的一个。
    ///
    /// 真库对应场景：起点 `(-9276.84, 10547.79, 1370)` 向 -Z，AABB 命中 8 个
    /// FLOOR，最近那个（`24381_39652`）网格在该 (x,y) 镂空。旧实现只留它，
    /// 精筛失败后整条查询返回 None。
    #[test]
    fn ray_collects_every_aabb_candidate_not_just_the_nearest() {
        let mut tree = AccelerationTree::default();
        // 三层楼板，z 从高到低：越靠上 toi 越小。
        tree.update_aabbs(vec![
            bbox_at("FLOOR", 1, [-10.0, -10.0, 900.0], [10.0, 10.0, 1000.0]),
            bbox_at("FLOOR", 2, [-10.0, -10.0, -400.0], [10.0, 10.0, 940.0]),
            bbox_at("FLOOR", 3, [-10.0, -10.0, -1700.0], [10.0, 10.0, -20.0]),
        ]);

        let qr = down_ray(0.0, 0.0, 1370.0);
        let hits = tree
            .tree
            .locate_with_selection_function(&qr)
            .collect::<Vec<_>>();
        let cands: HashSet<RefnoEnum> = qr.min_refnos.borrow().iter().copied().collect();

        assert_eq!(hits.len(), 3, "三层楼板都该被 AABB 命中");
        assert_eq!(
            cands.len(),
            3,
            "收集阶段必须保留全部 3 个候选，实际只留下 {} 个：{cands:?}",
            cands.len()
        );
    }

    /// 只命中一个时仍然要收进来（防止「不再 clear」改成「干脆不收」）。
    #[test]
    fn ray_collects_the_single_candidate() {
        let mut tree = AccelerationTree::default();
        tree.update_aabbs(vec![bbox_at(
            "FLOOR",
            1,
            [-10.0, -10.0, -100.0],
            [10.0, 10.0, 0.0],
        )]);

        let qr = down_ray(0.0, 0.0, 100.0);
        let _ = tree
            .tree
            .locate_with_selection_function(&qr)
            .collect::<Vec<_>>();
        let cands = qr.min_refnos.borrow().len();
        assert_eq!(cands, 1, "单个候选也必须被收集");
    }

    /// noun 不在 filter 里的构件不能被收集。
    #[test]
    fn ray_skips_nouns_outside_the_filter() {
        let mut tree = AccelerationTree::default();
        tree.update_aabbs(vec![
            bbox_at("WALL", 1, [-10.0, -10.0, -100.0], [10.0, 10.0, 0.0]),
            bbox_at("EQUI", 2, [-10.0, -10.0, -200.0], [10.0, 10.0, -150.0]),
        ]);

        let qr = down_ray(0.0, 0.0, 100.0);
        let _ = tree
            .tree
            .locate_with_selection_function(&qr)
            .collect::<Vec<_>>();
        let cands = qr.min_refnos.borrow().len();
        assert_eq!(cands, 0, "filter 只留 FLOOR，WALL/EQUI 不该进候选集");
    }

    /// 射线背对构件时不收集。
    #[test]
    fn ray_pointing_away_collects_nothing() {
        let mut tree = AccelerationTree::default();
        tree.update_aabbs(vec![bbox_at(
            "FLOOR",
            1,
            [-10.0, -10.0, -100.0],
            [10.0, 10.0, 0.0],
        )]);

        // 楼板在 z=[-100,0]，起点在 z=500（楼板上方）朝 +Z 打——背对楼板。
        let mut filter = HashSet::new();
        filter.insert("FLOOR".to_string());
        let qr = QueryRay::new(
            Ray::new(
                parry3d::math::Point::new(0.0, 0.0, 500.0),
                parry3d::math::Vector::new(0.0, 0.0, 1.0),
            ),
            filter,
            true,
        );
        let _ = tree
            .tree
            .locate_with_selection_function(&qr)
            .collect::<Vec<_>>();
        assert_eq!(
            qr.min_refnos.borrow().len(),
            0,
            "背对楼板时不该收集到候选"
        );
    }

    // ---- 嵌入 >1mm 的防穿板 ----
    //
    // `RAY_BACKOFF_MM = 1.0` 只吸收 ≤1mm 的微嵌。支腿插进楼板 >1mm 时，退 1mm 后起点
    // 仍在板内，`-Z` 射线命中楼板**底面**（外法向与射线同向），算出来是「板厚 − 嵌入
    // 深度」，一个与真实净距无关的假值。`corrected_toi` 用「起点在盒内 + 法向与射线
    // 同向」把这种情况归零。

    /// z=[-200,0] 的实心块（上下两个面 + 四周），供嵌入用例打穿。
    fn tree_with_solid_block(refno_seq: u64) -> (AccelerationTree, RefnoEnum) {
        let plate = bbox_at("FLOOR", refno_seq, [-10.0, -10.0, -200.0], [10.0, 10.0, 0.0]);
        let key: RefnoEnum = plate.refno.into();
        let tree = AccelerationTree::load(vec![plate]);
        let p = |x: f32, y: f32, z: f32| parry3d::math::Point::new(x, y, z);
        let mesh = TriMesh::with_flags(
            vec![
                p(-10.0, -10.0, 0.0),
                p(10.0, -10.0, 0.0),
                p(10.0, 10.0, 0.0),
                p(-10.0, 10.0, 0.0),
                p(-10.0, -10.0, -200.0),
                p(10.0, -10.0, -200.0),
                p(10.0, 10.0, -200.0),
                p(-10.0, 10.0, -200.0),
            ],
            vec![
                // 顶面，法向 +Z
                [0, 1, 2],
                [0, 2, 3],
                // 底面，法向 -Z
                [4, 6, 5],
                [4, 7, 6],
            ],
            TriMeshFlags::default(),
        );
        tree.mesh_cache.insert(key, vec![mesh]);
        (tree, key)
    }

    /// 起点嵌进楼板 5mm（> RAY_BACKOFF_MM）：净距按贴面算 0，不能返回「板厚 − 嵌入」。
    #[test]
    fn deeply_embedded_origin_is_treated_as_flush() {
        let (tree, key) = tree_with_solid_block(1);
        let hit = futures::executor::block_on(tree.query_nearest_by_ray(down_ray(0.0, 0.0, -5.0)))
            .unwrap()
            .expect("起点在板内，射线必须命中");
        assert_eq!(hit.0, key);
        assert!(
            hit.1.abs() < 1e-3,
            "嵌入 5mm 应按贴面 0 算，实际 {}（旧实现会返回约 195）",
            hit.1
        );
    }

    /// 起点在楼板**下方**、射线朝下（背对楼板）：不该命中，更不能因为法向判定被算成 0。
    #[test]
    fn origin_below_the_slab_does_not_hit_it() {
        let (tree, _) = tree_with_solid_block(1);
        let hit = futures::executor::block_on(tree.query_nearest_by_ray(down_ray(0.0, 0.0, -300.0)))
            .unwrap();
        assert!(hit.is_none(), "楼板在上方，-Z 射线不该命中它");
    }

    /// 起点在板面上方 100：正常返回 100，嵌入修正不能把外部命中误归零。
    #[test]
    fn origin_above_the_slab_keeps_the_real_gap() {
        let (tree, key) = tree_with_solid_block(1);
        let hit = futures::executor::block_on(tree.query_nearest_by_ray(down_ray(0.0, 0.0, 100.0)))
            .unwrap()
            .expect("正上方 100 处有楼板");
        assert_eq!(hit.0, key);
        assert!(
            (hit.1 - 100.0).abs() < 1e-3,
            "外部命中距离必须是 100，实际 {}",
            hit.1
        );
    }

    /// 起点在 AABB 内但**不在实体里**（斜置 / 异形构件让 AABB 膨胀，或盒内是空的）时，
    /// 命中点是近面而不是出射面，必须保留真实净距，不能被嵌入修正误归零。
    ///
    /// 真库对应场景：斜置楼板的 AABB 比实体大得多，设备底面落在膨胀盒内却仍在板外。
    #[test]
    fn origin_inside_the_aabb_but_outside_the_solid_keeps_the_gap() {
        // 大盒 z=[-1000, 0]，里面只有一片 z=-900 的水平面（盒内上方是空的）。
        let plate = bbox_at("FLOOR", 1, [-10.0, -10.0, -1000.0], [10.0, 10.0, 0.0]);
        let key: RefnoEnum = plate.refno.into();
        let tree = AccelerationTree::load(vec![plate]);
        let p = |x: f32, y: f32, z: f32| parry3d::math::Point::new(x, y, z);
        let mesh = TriMesh::with_flags(
            vec![
                p(-10.0, -10.0, -900.0),
                p(10.0, -10.0, -900.0),
                p(10.0, 10.0, -900.0),
                p(-10.0, 10.0, -900.0),
            ],
            vec![[0, 1, 2], [0, 2, 3]],
            TriMeshFlags::default(),
        );
        tree.mesh_cache.insert(key, vec![mesh]);

        // 起点在盒内（z=-100 落在 [-1000, 0]）、在实体上方 800。
        let hit = futures::executor::block_on(tree.query_nearest_by_ray(down_ray(0.0, 0.0, -100.0)))
            .unwrap()
            .expect("正下方有三角面");
        assert_eq!(hit.0, key);
        assert!(
            (hit.1 - 800.0).abs() < 1e-2,
            "盒内但在实体外，真实净距 800 必须保留，实际 {}",
            hit.1
        );
    }

    // ---- 多点批量最近楼板 ----

    /// 一批点共用一次 R-tree 遍历：每点各取自己正下方的最近楼板，互不串台。
    #[test]
    fn batch_query_returns_one_hit_per_point() {
        let (tree, key) = tree_with_plate_at_z0(1, [-200.0, 0.0]);
        let points = [
            glam::Vec3::new(0.0, 0.0, 100.0),
            glam::Vec3::new(5.0, 5.0, 300.0),
            // 板外 30mm：板只有 20x20（x/y 各 [-10,10]），这里必然落空。
            glam::Vec3::new(30.0, 0.0, 100.0),
        ];
        let mut filter = HashSet::new();
        filter.insert("FLOOR".to_string());
        let hits = futures::executor::block_on(tree.query_nearest_below(&points, &filter)).unwrap();

        assert_eq!(hits.len(), 3, "每点一格结果");
        assert_eq!(hits[0].map(|h| h.0), Some(key));
        assert!((hits[0].unwrap().1 - 100.0).abs() < 1e-3, "{:?}", hits[0]);
        assert_eq!(hits[1].map(|h| h.0), Some(key));
        assert!((hits[1].unwrap().1 - 300.0).abs() < 1e-3, "{:?}", hits[1]);
        assert!(hits[2].is_none(), "板外的点不该命中: {:?}", hits[2]);
    }

    /// 多点批量必须和单点逐条查询给出同样的结果（同一点集、同一棵树）。
    #[test]
    fn batch_query_agrees_with_single_point_queries() {
        let (tree, _) = tree_with_plate_at_z0(1, [-200.0, 0.0]);
        let points: Vec<glam::Vec3> = (0..5)
            .map(|i| glam::Vec3::new(i as f32 * 2.0 - 4.0, 1.0, 50.0 + i as f32 * 40.0))
            .collect();
        let mut filter = HashSet::new();
        filter.insert("FLOOR".to_string());

        let batch = futures::executor::block_on(tree.query_nearest_below(&points, &filter)).unwrap();
        for (index, pt) in points.iter().enumerate() {
            let single = futures::executor::block_on(
                tree.query_nearest_by_ray(down_ray(pt.x, pt.y, pt.z)),
            )
            .unwrap();
            assert_eq!(
                batch[index].map(|h| h.0),
                single.map(|h| h.0),
                "第 {index} 点的批量结果与单点结果不一致"
            );
            if let (Some(b), Some(s)) = (batch[index], single) {
                assert!(
                    (b.1 - s.1).abs() < 1e-3,
                    "第 {index} 点距离不一致：批量 {} 单点 {}",
                    b.1,
                    s.1
                );
            }
        }
    }

    /// 空点集 / 空 filter 必须直接返回空结果，不碰树。
    #[test]
    fn batch_query_handles_empty_inputs() {
        let (tree, _) = tree_with_plate_at_z0(1, [-200.0, 0.0]);
        let mut filter = HashSet::new();
        filter.insert("FLOOR".to_string());
        assert!(futures::executor::block_on(tree.query_nearest_below(&[], &filter))
            .unwrap()
            .is_empty());
        assert!(
            futures::executor::block_on(tree.query_nearest_below(
                &[glam::Vec3::new(0.0, 0.0, 100.0)],
                &HashSet::new()
            ))
            .unwrap()
            .iter()
            .all(Option::is_none)
        );
    }

    /// 多点批量也要按 z 升序剪枝：下方没有候选的点不能被上方候选算成命中。
    #[test]
    fn batch_query_skips_candidates_above_the_point() {
        // 楼板 z=[-200,0]；点在 z=-500（楼板下方）朝 -Z 打，应落空。
        let (tree, _) = tree_with_plate_at_z0(1, [-200.0, 0.0]);
        let mut filter = HashSet::new();
        filter.insert("FLOOR".to_string());
        let hits = futures::executor::block_on(
            tree.query_nearest_below(&[glam::Vec3::new(0.0, 0.0, -500.0)], &filter),
        )
        .unwrap();
        assert!(hits[0].is_none(), "楼板上方的候选不能算成命中: {:?}", hits[0]);
    }
}
