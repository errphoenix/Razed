use ethel::state::data::{
    Column, DirectIndex, IndirectIndex, SparseSlot,
    hash::{FxLsSpatialHash, SpatialResolution},
};
use janus::context::DeltaTime;

use crate::collision::{self, LightCollision, Sphere};

#[derive(Clone, Copy, Debug)]
pub struct RigidBodyOptions {
    gravity: f32,
    ground_level: Option<f32>,
    step_multiplier: f32,
    damping: f32,
    restitution: f32,
    friction: f32,
    static_volumes_hash_resolution: SpatialResolution,
}

pub const DEFAULT_GRAVITY: f32 = 9.807;
pub const DEFAULT_DAMPING: f32 = 0.725;
pub const DEFAULT_RESTITUTION: f32 = 0.001;
pub const DEFAULT_FRICTION: f32 = 0.00215;
pub const DEFAULT_STATIC_VOLUMES_HASH_RESOLUTION: SpatialResolution = SpatialResolution::new(2.0);

const INTERNAL_STEP_MULT: f32 = 1.0;

impl Default for RigidBodyOptions {
    fn default() -> Self {
        Self {
            gravity: DEFAULT_GRAVITY,
            ground_level: Some(0.0),
            step_multiplier: 1.0,
            damping: DEFAULT_DAMPING,
            restitution: DEFAULT_RESTITUTION,
            friction: DEFAULT_FRICTION,
            static_volumes_hash_resolution: DEFAULT_STATIC_VOLUMES_HASH_RESOLUTION,
        }
    }
}

impl RigidBodyOptions {
    pub fn new(
        gravity: f32,
        ground_level: Option<f32>,
        step_multiplier: f32,
        damping: f32,
        restitution: f32,
        friction: f32,
        static_volumes_hash_resolution: SpatialResolution,
    ) -> Self {
        Self {
            gravity,
            ground_level,
            step_multiplier,
            damping,
            restitution,
            friction,
            static_volumes_hash_resolution,
        }
    }

    pub fn with_gravity(self, gravity: f32) -> Self {
        Self {
            gravity,
            ground_level: self.ground_level,
            step_multiplier: self.step_multiplier,
            damping: self.damping,
            restitution: self.restitution,
            friction: self.friction,
            static_volumes_hash_resolution: self.static_volumes_hash_resolution,
        }
    }

    pub fn with_ground_level(self, ground_level: Option<f32>) -> Self {
        Self {
            ground_level,
            gravity: self.gravity,
            step_multiplier: self.step_multiplier,
            damping: self.damping,
            restitution: self.restitution,
            friction: self.friction,
            static_volumes_hash_resolution: self.static_volumes_hash_resolution,
        }
    }

    pub fn with_step_multiplier(self, step_multiplier: f32) -> Self {
        Self {
            step_multiplier,
            gravity: self.gravity,
            ground_level: self.ground_level,
            damping: self.damping,
            restitution: self.restitution,
            friction: self.friction,
            static_volumes_hash_resolution: self.static_volumes_hash_resolution,
        }
    }

    pub fn with_damping(self, damping: f32) -> Self {
        Self {
            damping,
            gravity: self.gravity,
            ground_level: self.ground_level,
            step_multiplier: self.step_multiplier,
            restitution: self.restitution,
            friction: self.friction,
            static_volumes_hash_resolution: self.static_volumes_hash_resolution,
        }
    }

    pub fn with_restitution(self, restitution: f32) -> Self {
        Self {
            restitution,
            gravity: self.gravity,
            ground_level: self.ground_level,
            step_multiplier: self.step_multiplier,
            damping: self.damping,
            friction: self.friction,
            static_volumes_hash_resolution: self.static_volumes_hash_resolution,
        }
    }

    pub fn with_friction(self, friction: f32) -> Self {
        Self {
            friction,
            gravity: self.gravity,
            ground_level: self.ground_level,
            step_multiplier: self.step_multiplier,
            damping: self.damping,
            restitution: self.restitution,
            static_volumes_hash_resolution: self.static_volumes_hash_resolution,
        }
    }

    pub fn with_static_volumes_hash_resolution(
        self,
        static_volumes_hash_resolution: SpatialResolution,
    ) -> Self {
        Self {
            static_volumes_hash_resolution,
            gravity: self.gravity,
            ground_level: self.ground_level,
            step_multiplier: self.step_multiplier,
            damping: self.damping,
            restitution: self.restitution,
            friction: self.friction,
        }
    }
}

ethel::table_spec! {
    struct StaticVolume {
        position: glam::Vec3;
        volume: Sphere;
    }
}

#[derive(Debug)]
pub struct RigidBodySolver {
    options: RigidBodyOptions,

    collision_buffer: Vec<LightCollision>,

    /// Static non-physical bodies that still contribute during the collision
    /// pass for dynamic bodies.
    ///
    /// Currently only supports [`spheres`](Sphere).
    static_volumes: StaticVolumeRowTable,
    static_volumes_hash: FxLsSpatialHash<IndirectIndex>,
}

impl Default for RigidBodySolver {
    fn default() -> Self {
        Self {
            options: Default::default(),
            collision_buffer: Vec::default(),
            static_volumes: StaticVolumeRowTable::default(),
            static_volumes_hash: FxLsSpatialHash::new(DEFAULT_STATIC_VOLUMES_HASH_RESOLUTION),
        }
    }
}

impl RigidBodySolver {
    pub fn new(options: RigidBodyOptions) -> Self {
        Self {
            options,
            collision_buffer: Vec::new(),
            static_volumes: StaticVolumeRowTable::new(),
            static_volumes_hash: FxLsSpatialHash::new(options.static_volumes_hash_resolution),
        }
    }

    pub fn clear_static_volumes(&mut self) {
        self.static_volumes.clear();
        self.static_volumes_hash.clear();
    }

    pub fn add_static_volume(&mut self, position: glam::Vec3, volume: Sphere) -> IndirectIndex {
        let id = self.static_volumes.insert((position, volume));
        let cell = self.static_volumes_hash.cell_at(position);
        self.static_volumes_hash.put(cell, id);
        id
    }

    pub fn remove_static_volume(&mut self, id: IndirectIndex) {
        if let Some(direct) = self.static_volumes.solve_indirect(id) {
            let position = self.static_volumes.position[direct.as_index()];

            let cell = self.static_volumes_hash.cell_at(position);
            let bucket = self.static_volumes_hash.get_mut(cell).unwrap();

            bucket.retain(|&h_id| h_id != id);
            self.static_volumes.free(id);
        }
    }

    pub fn detect_static_collisions(
        &mut self,
        rb_hash: &FxLsSpatialHash<DirectIndex>,
        positions: &[glam::Vec3],
        volumes: &[Sphere],
        handles: &[IndirectIndex],
    ) {
        self.static_volumes_hash.cells().for_each(|&cell| {
            let static_bodies = self.static_volumes_hash.get(cell);
            let dynamic_bodies = rb_hash.get(cell);

            if let Some(static_bodies) = static_bodies
                && !static_bodies.is_empty()
                && let Some(dynamic_bodies) = dynamic_bodies
                && !dynamic_bodies.is_empty()
            {
                static_bodies.iter().for_each(|&static_body| {
                    let static_index = self.static_volumes.solve_indirect(static_body).unwrap();
                    let static_volume = self.static_volumes.volume[static_index.as_index()];
                    let static_position = self.static_volumes.position[static_index.as_index()];

                    dynamic_bodies.iter().for_each(|dynamic_index| {
                        let dynamic_volume = volumes[dynamic_index.as_index()];
                        let dynamic_position = positions[dynamic_index.as_index()];

                        let d_sq = static_position.distance_squared(dynamic_position);
                        if static_volume.intersects(dynamic_volume, d_sq) {
                            let (n, depth) = static_volume.peneration_with(
                                dynamic_volume,
                                static_position,
                                dynamic_position,
                                d_sq,
                            );

                            self.collision_buffer.push(LightCollision {
                                normal: n,
                                depth,
                                index_a: IndirectIndex::default(),
                                index_b: handles[dynamic_index.as_index()],
                            });
                        }
                    });
                });
            }
        });
    }

    pub fn detect_collisions(
        &mut self,
        positions: &[glam::Vec3],
        volumes: &[Sphere],
        id_map: &[IndirectIndex],
    ) {
        collision::detect_n2(positions, volumes, id_map, &mut self.collision_buffer);
    }

    pub fn solve_collisions(
        &mut self,
        positions: &mut [glam::Vec3],
        velocities: &mut [glam::Vec3],
        ang_velocities: &mut [glam::Vec3],
        id_map: &[IndirectIndex],
    ) {
        self.collision_buffer.drain(..).for_each(|collision| {
            let index0 = collision.index_a;
            let index1 = collision.index_b;
            let id0 = id_map[index0.as_index()].as_index();
            let id1 = id_map[index1.as_index()].as_index();

            let correction = collision.depth * collision.normal;
            positions[id0] += correction * 0.5;
            positions[id1] -= correction * 0.5;

            let rel_v = velocities[id0] - velocities[id1];
            let d_v = rel_v.dot(collision.normal);

            if d_v < 0.0 && collision.depth > 0.0 {
                let j = -(1.0 - self.options.restitution) * d_v;
                let impulse = collision.normal * j;

                velocities[id0] += impulse * 0.5;
                velocities[id1] -= impulse * 0.5;

                ang_velocities[id0] *= 0.5;
                ang_velocities[id1] *= 0.5;
            }
        });
    }

    pub fn sync_inertia(
        &self,
        rotations: &[glam::Quat],
        inertia_loc: &[glam::Mat3],
        inertia_abs: &mut [glam::Mat3],
    ) {
        inertia_abs
            .iter_mut()
            .zip(inertia_loc.iter().zip(rotations))
            .for_each(|(i_abs, (i_loc, rot))| {
                let rot = glam::Mat3::from_quat(*rot);
                *i_abs = rot * *i_loc * rot.transpose();
            });
    }

    pub fn apply_gravity(&self, forces: &mut [glam::Vec3]) {
        forces.iter_mut().for_each(|f| f.y -= self.options.gravity);
    }

    pub fn integrate(
        &self,
        velocities: &mut [glam::Vec3],
        ang_velocities: &mut [glam::Vec3],
        forces: &mut [glam::Vec3],
        torques: &mut [glam::Vec3],
        masses: &[f32],
        inertia: &[glam::Mat3],
        delta: DeltaTime,
    ) {
        let h = delta.as_f32() * self.options.step_multiplier * INTERNAL_STEP_MULT;

        velocities
            .iter_mut()
            .zip(forces)
            .zip(masses)
            .for_each(|((v, f), w)| {
                let f = std::mem::take(f);
                *v += h * f * w;
            });
        ang_velocities
            .iter_mut()
            .zip(torques)
            .zip(inertia)
            .for_each(|((v, t), i)| {
                let t = std::mem::take(t);
                *v += h * i.mul_vec3(t);
            });
    }

    pub fn update_bodies(
        &self,
        positions: &mut [glam::Vec3],
        rotations: &mut [glam::Quat],
        velocities: &[glam::Vec3],
        ang_velocities: &[glam::Vec3],
        delta: DeltaTime,
    ) {
        let h = delta.as_f32() * self.options.step_multiplier * INTERNAL_STEP_MULT;
        positions.iter_mut().zip(velocities).for_each(|(p, v)| {
            *p += v * h;
        });
        rotations.iter_mut().zip(ang_velocities).for_each(|(r, v)| {
            let q = {
                let l = v.length() + f32::EPSILON;
                let lh = l * h;
                let a = v / l;
                glam::Quat::from_axis_angle(a, lh)
            };
            *r = q * *r;
            *r = r.normalize();
        });
    }

    pub fn damp_velocity(
        &self,
        velocities: &mut [glam::Vec3],
        ang_velocities: &mut [glam::Vec3],
        delta: DeltaTime,
    ) {
        let h = delta.as_f32() * self.options.step_multiplier * INTERNAL_STEP_MULT;
        let dh = self.options.damping * h;
        let dh2 = dh * dh;
        let dh2e = (-dh2).exp();
        velocities.iter_mut().for_each(|v| *v *= dh2e);
        ang_velocities.iter_mut().for_each(|v| *v *= dh2e);
    }

    pub fn constrain_ground(
        &self,
        positions: &mut [glam::Vec3],
        velocities: &mut [glam::Vec3],
        ang_velocities: &mut [glam::Vec3],
    ) {
        if let Some(ground_level) = self.options.ground_level {
            positions
                .iter_mut()
                .zip(velocities)
                .zip(ang_velocities)
                .for_each(|((p, v), a_v)| {
                    if p.y < ground_level {
                        p.y = ground_level;
                        v.y *= -self.options.restitution;
                        v.x *= self.options.friction;
                        v.z *= self.options.friction;
                        *a_v *= -self.options.restitution * 2.0;
                    }
                });
        }
    }
}
