use ethel::state::data::IndirectIndex;

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
    pub index_a: IndirectIndex,
    pub index_b: IndirectIndex,
}

/// Detect collisions between all bodies, given their `positions` and
/// `volumes`.
///
/// The indices in `id_map` are intended to be a mapping of the local indices
/// of the given slices and a global stable ID to recognize them by later.
pub fn detect_n2(
    positions: &[glam::Vec3],
    volumes: &[Sphere],
    id_map: &[IndirectIndex],
    results: &mut Vec<LightCollision>,
) {
    let len = positions.len();
    assert_eq!(len, volumes.len());

    for i in 0..len {
        for j in (i + 1)..len {
            let p0 = positions[i];
            let p1 = positions[j];
            let v0 = volumes[i];
            let v1 = volumes[j];

            let d_sq = p0.distance_squared(p1);
            if v0.intersects(v1, d_sq) && d_sq > 0.01 {
                let d = d_sq.sqrt();
                let dir = p0 - p1;
                let n = dir.normalize_or_zero();
                let depth = v0.radius + v1.radius - d;

                let id0 = id_map[i];
                let id1 = id_map[j];

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
