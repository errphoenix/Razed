use ethel::state::data::DirectIndex;

pub fn intersect_aabb_sphere(aabb: &Aabb, sphere: Sphere, sphere_center: glam::Vec3) -> bool {
    let e = (aabb.min - sphere_center).max(glam::Vec3::ZERO)
        + (sphere_center - aabb.max).max(glam::Vec3::ZERO);
    let r2 = sphere.radius * sphere.radius;
    r2 > e.length_squared()
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Aabb {
    pub min: glam::Vec3,
    pub max: glam::Vec3,
}

impl Aabb {
    pub fn unit(center: glam::Vec3) -> Self {
        Self {
            min: center - 0.5,
            max: center + 0.5,
        }
    }

    pub fn new(extents: glam::Vec3, center: glam::Vec3) -> Self {
        Self {
            min: center - extents * 0.5,
            max: center + extents * 0.5,
        }
    }

    pub fn from_cell(
        cell: ethel::state::data::hash::Cell,
        resolution: ethel::state::data::hash::SpatialResolution,
    ) -> Self {
        Self::new(
            glam::Vec3::splat(resolution.get()),
            resolution.approx_point(cell),
        )
    }

    pub fn with_center(self, new_center: glam::Vec3) -> Self {
        let extents = self.extents();
        Self::new(extents, new_center)
    }

    pub fn intersects(&self, other: Aabb) -> bool {
        for i in 0..3 {
            if self.min[i] > other.max[i] || other.min[i] > self.max[i] {
                return false;
            }
        }
        return true;
    }

    pub fn intersects_sphere(&self, sphere: Sphere, sphere_center: glam::Vec3) -> bool {
        intersect_aabb_sphere(self, sphere, sphere_center)
    }

    pub fn extents(&self) -> glam::Vec3 {
        (self.max - self.min).abs()
    }

    pub fn center(&self) -> glam::Vec3 {
        (self.min + self.max) * 0.5
    }
}

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

    pub fn intersects_aabb(&self, origin: glam::Vec3, aabb: Aabb) -> bool {
        intersect_aabb_sphere(&aabb, *self, origin)
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
#[derive(Debug, Clone, Copy, Default)]
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
            if v0.intersects(v1, d_sq) && d_sq > 0.0001 {
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

#[cfg(test)]
mod tests {
    use ethel::state::data::hash::{Cell, SpatialResolution};

    use super::*;

    #[test]
    fn intersect_aabb_sphere() {
        let aabb = Aabb::new(glam::Vec3::splat(1.0), glam::Vec3::ZERO);
        let sphere = Sphere::new(0.5);
        let sphere_o = glam::Vec3::ZERO;

        assert!(aabb.intersects_sphere(sphere, sphere_o));

        let aabb = Aabb::from_cell(Cell::new(2, 1, 1), SpatialResolution::new(1.0));
        let sphere = Sphere::new(0.5);
        let sphere_o = glam::vec3(1.85, 0.8, 0.75);

        assert!(aabb.intersects_sphere(sphere, sphere_o));

        let aabb = Aabb::from_cell(Cell::new(2, 1, 1), SpatialResolution::new(1.0));
        let sphere = Sphere::new(0.5);
        let sphere_o = glam::vec3(5.85, 0.8, 0.75);

        assert!(!aabb.intersects_sphere(sphere, sphere_o));

        let aabb = Aabb::from_cell(Cell::new(-2, -1, -1), SpatialResolution::new(1.0));
        let sphere = Sphere::new(0.5);
        let sphere_o = glam::vec3(-1.85, -0.8, -0.75);

        assert!(aabb.intersects_sphere(sphere, sphere_o));
    }
}
