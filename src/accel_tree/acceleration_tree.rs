use crate::geometry::PlantGeoData;
use crate::shape::pdms_shape::PlantMesh;
use crate::{types::*, GeomInstQuery, SUL_DB};
use approx::{abs_diff_ne, assert_abs_diff_eq, AbsDiffEq};
use bevy_ecs::prelude::Resource;
use dashmap::mapref::one::Ref;
use dashmap::DashMap;
use glam::{Mat4, Vec3};
use parry3d::bounding_volume::Aabb;
use parry3d::query::{Ray, RayCast};
use parry3d::shape::TriMesh;
use parry3d::shape::TriMeshFlags;
use rstar::Envelope;
use serde_derive::{Deserialize, Serialize};
use serde_with::{serde_as, As, FromInto};
use smallvec::SmallVec;
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

pub struct QueryRay {
    pub ray: Ray,
    pub filter_nouns: HashSet<String>,
    //需要使用当前的房间的距离，这样可以缩小范围
    pub toi: f32,
    pub solid: bool,
    pub min_dist: Cell<f32>,
    pub min_refnos: HashSet<RefnoEnum>,
}

impl QueryRay {
    pub fn new(ray: Ray, filter_nouns: HashSet<String>, solid: bool) -> Self {
        Self {
            ray,
            filter_nouns,
            toi: 10_0000.0,
            // found: Cell::new(false),
            min_dist: Cell::new(f32::MAX),
            min_refnos: HashSet::default(),
            solid,
        }
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
            //每次查找的距离应该比这个小，否则跳过
            let inter = bbox.aabb.cast_ray_and_get_normal(
                &Isometry::identity(),
                &self.ray,
                self.toi,
                self.solid,
            );
            // dbg!(&inter);
            if let Some(ray_inter) = inter
                && ray_inter.time_of_impact <= self.min_dist.get()
            {
                self.min_dist.set(ray_inter.time_of_impact);

                //找到更近的，清空之前的
                // if abs_diff_ne!(ray_inter.time_of_impact, self.toi) {
                //     self.min_refnos.borrow_mut().clear()
                // }
                // self.min_refnos.borrow_mut().insert(bbox.refno.into());
                // println!("found: {}", bbox.refno);
                // dbg!(ray_inter.toi);
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
        self.indexed_tree_len = self.tree.size();
        removed
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
        let geom_insts = crate::query_insts(&[refno], true).await.ok()?;
        // dbg!(geom_insts.len());
        let mut meshes = vec![];
        for g in geom_insts {
            // dbg!(&g);
            for inst in &g.insts {
                let Ok(mesh) =
                    PlantMesh::des_mesh_file(&format!("assets/meshes/{}.mesh", inst.geo_hash))
                else {
                    continue;
                };
                // dbg!(mesh.vertices.len());
                if mesh.vertices.is_empty() {
                    continue;
                }
                let trans = g.world_trans * inst.transform;
                let Some(tri_mesh) = mesh.get_tri_mesh(trans.compute_matrix()) else {
                    continue;
                };
                meshes.push(tri_mesh);
            }
        }
        self.mesh_cache.insert(refno, meshes);
        return self.mesh_cache.get(&refno);
    }

    /// Returns the refno of the nearest object to `query_point`
    /// 也可以检查墙等等，需要判断mesh
    pub async fn query_nearest_by_ray(
        &self,
        ray: QueryRay,
    ) -> anyhow::Result<Option<(RefnoEnum, f32)>> {
        let _ = self
            .tree
            .locate_with_selection_function(&ray)
            .collect::<Vec<_>>();
        let refnos = &ray.min_refnos;
        if refnos.is_empty() {
            return Ok(None);
        }
        //检查是否真的和ray相交, 根据 profile 判断吗？
        //根据 param 判断是否相交吗？
        //直接检查是否在表面上即可
        for &refno in refnos.iter() {
            let Some(tri_meshes) = self.get_tri_mesh(refno).await else {
                continue;
            };
            for mesh in tri_meshes.value() {
                let intersection_flag =
                    match mesh.cast_local_ray_and_get_normal(&ray.ray, 10_0000.0, ray.solid) {
                        // Some(intersection) => tri_mesh.is_backface(intersection.feature),
                        Some(intersection) => {
                            // dbg!(&intersection);
                            true
                        }
                        None => false,
                    };
                if intersection_flag {
                    return Ok(Some((refno, ray.min_dist.get())));
                }
            }
        }
        return Ok(None);
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
        assert_eq!(stale[0].aabb, bbox(1, 0.0, 10.0).aabb, "返回值必须是被替换的旧盒");
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

    #[test]
    fn refno_index_keeps_legacy_bincode_compatible() {
        let current = AccelerationTree::load(vec![
            bbox(1, 0.0, 10.0),
            bbox(2, 20.0, 30.0),
        ]);
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
}
