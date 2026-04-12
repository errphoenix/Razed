use ethel::state::data::DirectIndex;

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Default)]
pub struct Sphere {
    pub radius: f32,
}

impl Sphere {
    pub const UNIT: Self = Self::new(1.0);
    pub const HALF_UNIT: Self = Self::new(0.5);

    pub const fn new(radius: f32) -> Self {
        Self { radius }
    }

    pub const fn intersects(&self, other: Sphere, distance_squared: f32) -> bool {
        let r1 = self.radius;
        let r2 = other.radius;
        let rr2 = (r1 + r2) * (r1 + r2);
        distance_squared < rr2
    }

    /// Returns the normalized intersection direction and the depth of the penetration.
    pub fn peneration_with(
        &self,
        other: Sphere,
        origin: glam::Vec3,
        other_center: glam::Vec3,
        distance_squared: f32,
    ) -> (glam::Vec3, f32) {
        let d = distance_squared.sqrt();
        let dir = origin - other_center;
        let n = dir.normalize_or_zero();
        let depth = self.radius + other.radius - d;
        (n, depth)
    }
}

/// A very basic collision.
///
/// Stores the indices of the bodies involved, and contact data (direction
/// and depth).
///
/// The indices of the bodies depend on the ID map passed to the collision
/// detection function, they are intended to be global indices.
#[derive(Debug, Clone, Copy)]
pub struct LightCollision {
    pub normal: glam::Vec3,
    pub depth: f32,
    pub index_a: DirectIndex,
    pub index_b: DirectIndex,
}

/// Detect collisions between all bodies, given their `positions` and
/// `volumes`.
pub fn detect_n2(
    positions: &[glam::Vec3],
    volumes: &[Sphere],
    direct_indices: &[DirectIndex],
    results: &mut Vec<LightCollision>,
) {
    let len = positions.len();
    assert_eq!(len, volumes.len());
    assert_eq!(len, direct_indices.len());

    for i in 0..len {
        for j in (i + 1)..len {
            let p0 = positions[i];
            let p1 = positions[j];
            let v0 = volumes[i];
            let v1 = volumes[j];

            let d_sq = p0.distance_squared(p1);
            if v0.intersects(v1, d_sq) && d_sq > 0.01 {
                let (n, depth) = v0.peneration_with(v1, p0, p1, d_sq);

                let id0 = direct_indices[i];
                let id1 = direct_indices[j];

                results.push(LightCollision {
                    normal: n,
                    depth,
                    index_a: id0,
                    index_b: id1,
                });
            }
        }
    }
}
