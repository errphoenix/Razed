use ethel::{Cell, Column, FxLsSpatialHash, FxSpatialHash, SpatialHash};
use glam::FloatExt;

use crate::simulation::{
    StructurePartIndex, StructureSpatialHash, fragment::FragmentSystem, lattice::LatticeSystem,
};

#[derive(Clone, Copy, Debug)]
pub struct DamageOverlay {
    pub visible: bool,
    pub position: glam::Vec3,
    pub volume: DamageVolume,
}
impl Default for DamageOverlay {
    fn default() -> Self {
        Self {
            volume: DamageVolume::Spherical { radius: 0f32 },
            visible: false,
            position: glam::Vec3::ZERO,
        }
    }
}

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
    Linear,
    Exponential,
    #[default]
    Quadratic,
    Cubic,
}
impl DamageFalloffModel {
    /// Applies the corresponding damage falloff curve.
    ///
    /// The `value` parameter must correspond to the vicinity to the center,
    /// i.e. the inverse distance.
    pub fn apply(&self, value: f32) -> f32 {
        match self {
            DamageFalloffModel::Linear => value,
            DamageFalloffModel::Exponential => (value).exp() - 1.0,
            DamageFalloffModel::Quadratic => value * value,
            DamageFalloffModel::Cubic => value * value * value,
        }
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
    pub strength: f32,
}
impl DamageCloud {
    pub const fn new(
        position: glam::Vec3,
        amount: f32,
        volume: DamageVolume,
        falloff: DamageFalloffModel,
        strength: f32,
    ) -> Self {
        Self {
            position,
            amount,
            volume,
            falloff,
            strength,
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
    queue: Vec<DamageCloud>,
    lookup_buffer: Vec<Cell>,
}
impl DamageSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn queue_damage(&mut self, damage_cloud: DamageCloud) {
        self.queue.push(damage_cloud);
    }

    pub fn propagate_damage(
        &mut self,
        hashed: &StructureSpatialHash,
        fragment_sys: &mut FragmentSystem,
        lattice_sys: &mut LatticeSystem,
    ) {
        let fragments = &mut fragment_sys.fragments;
        self.queue.drain(..).for_each(|cloud| {
            hashed.get_damage_area(&cloud, &mut self.lookup_buffer);
            let origin = cloud.position;
            let falloff = cloud.falloff;
            let dmg_amount = cloud.amount;
            let strength = cloud.strength;
            self.lookup_buffer.drain(..).for_each(|cell| {
                if let Some(affected) = hashed.get(cell) {
                    if !affected.is_empty() {
                        let point = hashed.approx_point_at(cell);
                        let to_origin = origin - point;
                        let vicinity = to_origin.length();

                        let coefficient = falloff * vicinity;
                        let damage = coefficient * dmg_amount;
                        let impulse_dir = -to_origin;
                        let impulse = coefficient * impulse_dir * strength;

                        for &id in affected {
                            match id {
                                StructurePartIndex::Lattice(id) => {
                                    // todo: persistent impulse
                                    lattice_sys.apply_forces(id, impulse);
                                }
                                StructurePartIndex::Fragment(id) => {
                                    let did = fragments.solve_indirect(id).unwrap();
                                    let index = did.as_index();
                                    let mut integrity = fragments.integrity[index];
                                    integrity -= damage;
                                    fragments.integrity[index] = integrity.saturate();
                                }
                            }
                        }
                    }
                }
            });
        });
    }

    pub fn damage_queue(&self) -> &[DamageCloud] {
        &self.queue
    }

    pub const fn damage_queue_mut(&mut self) -> &Vec<DamageCloud> {
        &mut self.queue
    }
}
