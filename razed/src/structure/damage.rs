use ethel::{Cell, FxLsSpatialHash, FxSpatialHash, SpatialHash};

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum DamageVolume {
    Spherical { radius: f32 },
    Cubic { side: f32 },
}
impl DamageVolume {
    pub const fn half_extents(&self) -> glam::Vec3 {
        match self {
            DamageVolume::Spherical { radius } => glam::vec3(*radius, *radius, *radius),
            DamageVolume::Cubic { side } => {
                let half_side = *side * 0.5;
                glam::vec3(half_side, half_side, half_side)
            }
        }
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
pub enum DamageFalloffModel {
    /// `1.0-x`
    Linear,
    /// `exp(-x)*(1.0-x)`
    Exponential,
    #[default]
    /// `1.0-x*x`
    Quadratic,
    /// `1.0-x*x*x`
    Cubic,
}
impl DamageFalloffModel {
    /// Applies the corresponding damage falloff curve.
    ///
    /// The `value` parameter must be the distance from the center from
    /// `0.0` to `1.0`, representing the center and the bounds of the damage
    /// volume, respectively.
    pub fn apply(&self, value: f32) -> f32 {
        match self {
            DamageFalloffModel::Linear => 1.0 - value,
            DamageFalloffModel::Exponential => (-value).exp() * (1.0 - value),
            DamageFalloffModel::Quadratic => 1.0 - (value * value),
            DamageFalloffModel::Cubic => 1.0 - (value * value),
        }
        .clamp(0.0, 1.0)
    }
}
impl std::ops::Mul<f32> for DamageFalloffModel {
    type Output = f32;

    fn mul(self, rhs: f32) -> Self::Output {
        self.apply(rhs)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DamageCloud {
    pub position: glam::Vec3,
    pub amount: f32,
    pub volume: DamageVolume,
    pub falloff: DamageFalloffModel,
}
impl DamageCloud {
    pub const fn new(
        position: glam::Vec3,
        amount: f32,
        volume: DamageVolume,
        falloff: DamageFalloffModel,
    ) -> Self {
        Self {
            position,
            amount,
            volume,
            falloff,
        }
    }

    pub const fn as_aabb_volume(&self) -> (glam::Vec3, glam::Vec3) {
        let half_extents = self.volume.half_extents();
        (
            glam::vec3(
                self.position.x - half_extents.x,
                self.position.y - half_extents.y,
                self.position.z - half_extents.z,
            ),
            glam::vec3(
                self.position.x + half_extents.x,
                self.position.y + half_extents.y,
                self.position.z + half_extents.z,
            ),
        )
    }

    pub fn test_aabb(&self, aabb: (glam::Vec3, glam::Vec3)) -> bool {
        let aabb = physics::Aabb {
            min: aabb.0,
            max: aabb.1,
        };
        match self.volume {
            DamageVolume::Spherical { radius } => {
                physics::Sphere::new(radius).intersects_aabb(self.position, aabb)
            }
            DamageVolume::Cubic { side } => {
                physics::Aabb::new(glam::Vec3::splat(side), self.position).intersects(aabb)
            }
        }
    }
}

pub trait DamageSpatialLookupExt<T: Clone>: SpatialHash<T> {
    fn get_damage_area(&self, cloud: &DamageCloud, out: &mut Vec<Cell>) {
        let aabb = cloud.as_aabb_volume();
        let (min, max) = (self.cell_at(aabb.0), self.cell_at(aabb.1));
        Self::get_aabb_area(min, max, out);
        out.retain(|&cell| {
            let aabb = self.cell_extents(cell);
            cloud.test_aabb(aabb)
        });
    }
}
impl<T: Clone + Copy> DamageSpatialLookupExt<T> for FxSpatialHash<T> {}
impl<T: Clone + Copy> DamageSpatialLookupExt<Vec<T>> for FxLsSpatialHash<T> {}

#[derive(Clone, Debug, Default)]
pub struct DamageSystem {
    damage_queue: Vec<DamageCloud>,
}
impl DamageSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn queue_damage(&mut self, damage_cloud: DamageCloud) {
        self.damage_queue.push(damage_cloud);
    }

    pub fn damage_queue(&self) -> &[DamageCloud] {
        &self.damage_queue
    }

    pub const fn damage_queue_mut(&mut self) -> &Vec<DamageCloud> {
        &mut self.damage_queue
    }
}
