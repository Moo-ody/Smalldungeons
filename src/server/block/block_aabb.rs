use crate::server::utils::dvec3::DVec3;

/// Represents a single AABB in local block coordinates [0.0, 1.0]
#[derive(Debug, Clone)]
pub struct BlockAABB {
    pub min: DVec3,
    pub max: DVec3,
}

impl BlockAABB {
    pub fn new(min_x: f64, min_y: f64, min_z: f64, max_x: f64, max_y: f64, max_z: f64) -> Self {
        Self {
            min: DVec3::new(min_x, min_y, min_z),
            max: DVec3::new(max_x, max_y, max_z),
        }
    }

    /// Full cube AABB
    pub fn full_cube() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0, 1.0, 1.0)
    }

    /// Check if this AABB intersects with a line segment
    pub fn intersects_segment(&self, start: &DVec3, end: &DVec3) -> Option<f64> {
        // Ray-AABB intersection using slab method
        let dir = *end - *start;
        let inv_dir = DVec3::new(
            if dir.x != 0.0 { 1.0 / dir.x } else { f64::INFINITY },
            if dir.y != 0.0 { 1.0 / dir.y } else { f64::INFINITY },
            if dir.z != 0.0 { 1.0 / dir.z } else { f64::INFINITY },
        );

        let t1 = (self.min.x - start.x) * inv_dir.x;
        let t2 = (self.max.x - start.x) * inv_dir.x;
        let t3 = (self.min.y - start.y) * inv_dir.y;
        let t4 = (self.max.y - start.y) * inv_dir.y;
        let t5 = (self.min.z - start.z) * inv_dir.z;
        let t6 = (self.max.z - start.z) * inv_dir.z;

        let tmin = t1.min(t2).max(t3.min(t4)).max(t5.min(t6));
        let tmax = t1.max(t2).min(t3.max(t4)).min(t5.max(t6));

        if tmax < 0.0 || tmin > tmax {
            None
        } else {
            Some(tmin)
        }
    }

    /// Get the face normal at the intersection point
    pub fn get_face_normal(&self, intersection: &DVec3) -> (char, f64) {
        // Determine which face was hit by checking which coordinate is closest to min/max
        let local_pos = *intersection;
        
        let dx_min = (local_pos.x - self.min.x).abs();
        let dx_max = (local_pos.x - self.max.x).abs();
        let dy_min = (local_pos.y - self.min.y).abs();
        let dy_max = (local_pos.y - self.max.y).abs();
        let dz_min = (local_pos.z - self.min.z).abs();
        let dz_max = (local_pos.z - self.max.z).abs();

        let min_dist = dx_min.min(dx_max).min(dy_min.min(dy_max)).min(dz_min.min(dz_max));

        if dx_min == min_dist {
            ('x', if local_pos.x < 0.5 { -1.0 } else { 1.0 })
        } else if dy_min == min_dist {
            ('y', if local_pos.y < 0.5 { -1.0 } else { 1.0 })
        } else if dz_min == min_dist {
            ('z', if local_pos.z < 0.5 { -1.0 } else { 1.0 })
        } else if dx_max == min_dist {
            ('x', if local_pos.x < 0.5 { -1.0 } else { 1.0 })
        } else if dy_max == min_dist {
            ('y', if local_pos.y < 0.5 { -1.0 } else { 1.0 })
        } else {
            ('z', if local_pos.z < 0.5 { -1.0 } else { 1.0 })
        }
    }
}

/// Collection of AABBs for a single block
#[derive(Debug, Clone)]
pub struct BlockShape {
    pub aabbs: Vec<BlockAABB>,
}

impl BlockShape {
    pub fn new(aabbs: Vec<BlockAABB>) -> Self {
        Self { aabbs }
    }

    /// Full cube block
    pub fn full_cube() -> Self {
        Self::new(vec![BlockAABB::full_cube()])
    }

    /// Top slab
    pub fn top_slab() -> Self {
        Self::new(vec![BlockAABB::new(0.0, 0.5, 0.0, 1.0, 1.0, 1.0)])
    }

    /// Bottom slab
    pub fn bottom_slab() -> Self {
        Self::new(vec![BlockAABB::new(0.0, 0.0, 0.0, 1.0, 0.5, 1.0)])
    }

    /// Stairs (approximate with two-box model per facing)
    pub fn stairs() -> Self {
        Self::new(vec![
            BlockAABB::new(0.0, 0.0, 0.0, 1.0, 0.5, 1.0), // bottom half
            BlockAABB::new(0.0, 0.5, 0.0, 0.5, 1.0, 1.0), // top half (step lip)
        ])
    }

    /// Iron bars (center post + arms)
    pub fn iron_bars() -> Self {
        Self::new(vec![
            BlockAABB::new(0.4375, 0.0, 0.4375, 0.5625, 1.0, 0.5625), // center post
            BlockAABB::new(0.0, 0.0, 0.4375, 0.4375, 1.0, 0.5625),    // west arm
            BlockAABB::new(0.5625, 0.0, 0.4375, 1.0, 1.0, 0.5625),    // east arm
            BlockAABB::new(0.4375, 0.0, 0.0, 0.5625, 1.0, 0.4375),    // north arm
            BlockAABB::new(0.4375, 0.0, 0.5625, 0.5625, 1.0, 1.0),    // south arm
        ])
    }

    /// Glass panes (center post + arms)
    pub fn glass_pane() -> Self {
        Self::new(vec![
            BlockAABB::new(0.4375, 0.0, 0.4375, 0.5625, 1.0, 0.5625), // center post
            BlockAABB::new(0.0, 0.0, 0.4375, 0.4375, 1.0, 0.5625),    // west arm
            BlockAABB::new(0.5625, 0.0, 0.4375, 1.0, 1.0, 0.5625),    // east arm
            BlockAABB::new(0.4375, 0.0, 0.0, 0.5625, 1.0, 0.4375),    // north arm
            BlockAABB::new(0.4375, 0.0, 0.5625, 0.5625, 1.0, 1.0),    // south arm
        ])
    }

    /// Fences (center post + arms)
    pub fn fence() -> Self {
        Self::new(vec![
            BlockAABB::new(0.375, 0.0, 0.375, 0.625, 1.0, 0.625), // center post
            BlockAABB::new(0.0, 0.0, 0.375, 0.375, 1.0, 0.625),    // west arm
            BlockAABB::new(0.625, 0.0, 0.375, 1.0, 1.0, 0.625),    // east arm
            BlockAABB::new(0.375, 0.0, 0.0, 0.625, 1.0, 0.375),    // north arm
            BlockAABB::new(0.375, 0.0, 0.625, 0.625, 1.0, 1.0),    // south arm
        ])
    }

    /// Cobblestone wall (thicker center)
    pub fn cobblestone_wall() -> Self {
        Self::new(vec![
            BlockAABB::new(0.25, 0.0, 0.25, 0.75, 1.0, 0.75), // wall body
        ])
    }

    /// Carpet (thin horizontal)
    pub fn carpet() -> Self {
        Self::new(vec![BlockAABB::new(0.0, 0.0, 0.0, 1.0, 0.0625, 1.0)])
    }

    /// Snow layer (thin horizontal)
    pub fn snow_layer() -> Self {
        Self::new(vec![BlockAABB::new(0.0, 0.0, 0.0, 1.0, 0.125, 1.0)])
    }

    /// Check intersection with all AABBs in this block
    pub fn intersects_segment(&self, start: &DVec3, end: &DVec3) -> Option<(f64, BlockAABB)> {
        let mut closest_intersection = None;
        let mut closest_t = f64::INFINITY;

        for aabb in &self.aabbs {
            if let Some(t) = aabb.intersects_segment(start, end) {
                if t < closest_t {
                    closest_t = t;
                    closest_intersection = Some((t, aabb.clone()));
                }
            }
        }

        closest_intersection
    }
}

