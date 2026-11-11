use ethel::{Cell, Column, FxLsSpatialHash, FxSpatialHash, SpatialHash};
use glam::FloatExt;
use physics::force::{ForcesRegistry, PersistentForce};

use crate::simulation::{debris::DebrisSystem, fragment::FragmentSystem};

#[derive(Clone, Copy, Debug)]
pub struct DamageOverlay {
    pub visible: bool,
    pub position: glam::Vec3,
    pub radius: f32,
}
impl Default for DamageOverlay {
    fn default() -> Self {
        Self {
            radius: 0f32,
            visible: false,
            position: glam::Vec3::ZERO,
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
    pub radius: f32,
    pub falloff: DamageFalloffModel,
    pub strength: f32,
}
impl DamageCloud {
    pub const fn new(
        position: glam::Vec3,
        amount: f32,
        radius: f32,
        falloff: DamageFalloffModel,
        strength: f32,
    ) -> Self {
        Self {
            position,
            amount,
            radius,
            falloff,
            strength,
        }
    }

    pub const fn as_aabb_volume(&self) -> (glam::Vec3, glam::Vec3) {
        let half_extents = self.radius;
        (
            glam::vec3(
                self.position.x - half_extents,
                self.position.y - half_extents,
                self.position.z - half_extents,
            ),
            glam::vec3(
                self.position.x + half_extents,
                self.position.y + half_extents,
                self.position.z + half_extents,
            ),
        )
    }

    pub fn test_aabb(&self, aabb: (glam::Vec3, glam::Vec3)) -> bool {
        let aabb = physics::Aabb {
            min: aabb.0,
            max: aabb.1,
        };
        physics::Sphere::new(self.radius).intersects_aabb(self.position, aabb)
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
        fragment_sys: &mut FragmentSystem,
        debris_sys: &mut DebrisSystem,
        forces: &mut ForcesRegistry,
    ) {
        let fragments = &mut fragment_sys.data;
        let h_frags = &mut fragment_sys.hashed;
        let h_debris = &mut debris_sys.hashed;

        self.queue.drain(..).for_each(|cloud| {
            h_frags.get_damage_area(&cloud, &mut self.lookup_buffer);
            h_debris.get_damage_area(&cloud, &mut self.lookup_buffer);

            let origin = cloud.position;
            let falloff = cloud.falloff;
            let dmg_amount = cloud.amount;
            let strength = cloud.strength;
            let radius = cloud.radius;
            self.lookup_buffer.drain(..).for_each(|cell| {
                let b_frags = h_frags.get(cell);
                let b_debris = h_debris.get(cell);
                if b_frags.is_some_and(|v| !v.is_empty()) || b_debris.is_some_and(|v| !v.is_empty())
                {
                    let point = h_frags.approx_point_at(cell);
                    let to_origin = origin - point;

                    let normalized_distance = (to_origin.length() / radius).clamp(0.0, 1.0);
                    let vicinity = 1.0 - normalized_distance;

                    let coefficient = falloff * vicinity;
                    let damage = coefficient * dmg_amount;

                    let impulse_dir = -to_origin.normalize();
                    let f_impulse = coefficient * impulse_dir * strength;

                    if f_impulse.length_squared() > 0.01 {
                        const DURATION_COEFF: f32 = 0.25;
                        //const FALLOFF: fn(f32) -> f32 = |t| 1.0 - t * t; // quadratic curve
                        //const FALLOFF: fn(f32) -> f32 = |t| (-t).exp(); // exponential curve
                        const FALLOFF: fn(f32) -> f32 = |t| (-t * 3f32).exp(); // exponential curve alt

                        let impulse = f_impulse * radius;
                        let duration = radius * DURATION_COEFF;
                        let p_force = PersistentForce::with_falloff(impulse, duration, FALLOFF);
                        let fid = forces.record(p_force);

                        if let Some(structure) = b_frags {
                            for &id in structure {
                                let did = fragments.solve_indirect(id).unwrap();
                                let index = did.as_index();
                                let mut integrity = fragments.integrity[index];
                                integrity -= damage;
                                fragments.integrity[index] = integrity.saturate();
                                fragments.held_force[index] += f_impulse;
                            }
                        }
                        if let Some(debris) = b_debris {
                            for &id in debris {
                                forces.add_target_body(fid, id);
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
