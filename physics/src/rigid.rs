use ethel::state::data::IndirectIndex;
use janus::context::DeltaTime;

use crate::collision::{self, LightCollision, Sphere};

#[derive(Clone, Copy, Debug)]
pub struct RigidBodyOptions {
    gravity: f32,
    ground_level: Option<f32>,
    step_multiplier: f32,
    damping: f32,
    restitution: f32,
}

pub const DEFAULT_GRAVITY: f32 = 9.807;
pub const DEFAULT_DAMPING: f32 = 0.85;
pub const DEFAULT_RESTITUTION: f32 = 0.125;

const INTERNAL_STEP_MULT: f32 = 1.0;

impl Default for RigidBodyOptions {
    fn default() -> Self {
        Self {
            gravity: DEFAULT_GRAVITY,
            ground_level: Some(0.0),
            step_multiplier: 1.0,
            damping: DEFAULT_DAMPING,
            restitution: DEFAULT_RESTITUTION,
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
    ) -> Self {
        Self {
            gravity,
            ground_level,
            step_multiplier,
            damping,
            restitution,
        }
    }

    pub fn with_gravity(self, gravity: f32) -> Self {
        Self {
            gravity,
            ground_level: self.ground_level,
            step_multiplier: self.step_multiplier,
            damping: self.damping,
            restitution: self.restitution,
        }
    }

    pub fn with_ground_level(self, ground_level: Option<f32>) -> Self {
        Self {
            ground_level,
            gravity: self.gravity,
            step_multiplier: self.step_multiplier,
            damping: self.damping,
            restitution: self.restitution,
        }
    }

    pub fn with_step_multiplier(self, step_multiplier: f32) -> Self {
        Self {
            step_multiplier,
            gravity: self.gravity,
            ground_level: self.ground_level,
            damping: self.damping,
            restitution: self.restitution,
        }
    }

    pub fn with_damping(self, damping: f32) -> Self {
        Self {
            damping,
            gravity: self.gravity,
            ground_level: self.ground_level,
            step_multiplier: self.step_multiplier,
            restitution: self.restitution,
        }
    }

    pub fn with_restitution(self, restitution: f32) -> Self {
        Self {
            restitution,
            gravity: self.gravity,
            ground_level: self.ground_level,
            step_multiplier: self.step_multiplier,
            damping: self.damping,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RigidBodySolver {
    options: RigidBodyOptions,

    collision_buffer: Vec<LightCollision>,
}

impl Default for RigidBodySolver {
    fn default() -> Self {
        Self {
            options: Default::default(),
            collision_buffer: Vec::default(),
        }
    }
}

impl RigidBodySolver {
    pub const fn new(options: RigidBodyOptions) -> Self {
        Self {
            options,
            collision_buffer: Vec::new(),
        }
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

                ang_velocities[id0] *= 0.8;
                ang_velocities[id1] *= 0.8;
            }
        });
    }

    pub fn pre_pass_inertia(
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

    pub fn pre_pass_gravity(&self, forces: &mut [glam::Vec3], torques: &mut [glam::Vec3]) {
        forces.iter_mut().for_each(|f| f.y -= self.options.gravity);
        torques.iter_mut().for_each(|t| {
            t.x += self.options.gravity;
            t.z += self.options.gravity;
        });
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
                *v += h * i.mul_vec3(t) + f32::EPSILON;
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

    pub fn post_damp_velocities(
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

    pub fn post_ground_constraint(
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
                        *v *= -self.options.restitution; //todo: friction
                        *a_v *= -self.options.restitution * 2.0;
                    }
                });
        }
    }
}
