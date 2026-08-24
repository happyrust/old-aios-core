use crate::types::*;
use parry3d::bounding_volume::Aabb;

pub struct RStarBoundingBox2D {
    aabb: rstar::AABB<[f32; 2]>,
    refno: RefU64,
}

impl RStarBoundingBox2D {
    pub fn from_aabb(bounds: &Aabb, refno: RefU64) -> Self {
        Self {
            aabb: rstar::AABB::from_corners(
                [bounds.mins[0], bounds.mins[1]],
                [bounds.maxs[0], bounds.maxs[1]],
            ),
            refno,
        }
    }
}

impl rstar::RTreeObject for RStarBoundingBox2D {
    type Envelope = rstar::AABB<[f32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.aabb
    }
}

impl rstar::PointDistance for RStarBoundingBox2D {
    fn distance_2(&self, point: &[f32; 2]) -> f32 {
        self.aabb.distance_2(point)
    }
}

#[derive(Default)]
pub struct Acceleration2DTree {
    tree: rstar::RTree<RStarBoundingBox2D>,
}

impl Acceleration2DTree {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.tree.size() == 0
    }

    pub fn load(bounding_boxes: Vec<RStarBoundingBox2D>) -> Self {
        Self {
            tree: rstar::RTree::bulk_load(bounding_boxes),
        }
    }

    pub fn replace(&mut self, bounding_boxes: Vec<RStarBoundingBox2D>) {
        self.tree = rstar::RTree::bulk_load(bounding_boxes);
    }

    // pub fn locate_withing_distance<'a>(&'a self, loc: Vec3, distance: f32) -> impl Iterator<Item = RefU64> + 'a {
    //     self.tree
    //         .locate_within_distance([loc.x, loc.y, loc.z], distance.powi(2))
    //         .map(|bb| bb.refno)
    // }

    pub fn locate_intersecting_bounds<'a>(
        &'a self,
        bounds: &Aabb,
    ) -> impl Iterator<Item = RefU64> + 'a {
        self.tree
            .locate_in_envelope_intersecting(&rstar::AABB::from_corners(
                [bounds.mins[0], bounds.mins[1]],
                [bounds.maxs[0], bounds.maxs[1]],
            ))
            .map(|bb| bb.refno)
    }

    // pub fn keys_intersecting_bounds(&self, bounds: AABB) -> Vec<StrokeKey> {
    //     self.0
    //         .locate_in_envelope_intersecting(&rstar::AABB::from_corners(
    //             [bounds.mins[0], bounds.mins[1]],
    //             [bounds.maxs[0], bounds.maxs[1]],
    //         ))
    //         .map(|object| object.data)
    //         .collect()
    // }

    // pub fn locate<'a>(&'a self, ray: &'a Ray) -> impl Iterator<Item = Entity> + 'a {
    //     self.tree
    //         .locate_with_selection_function(ray)
    //         .map(|bb| bb.entity)
    // }
}
