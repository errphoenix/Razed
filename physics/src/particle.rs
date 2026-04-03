use janus::context::DeltaTime;

#[derive(Clone, Copy, Debug)]
pub struct ParticleOptions {
    gravity: f32,
    ground_level: Option<f32>,
    step_multiplier: f32,
    damping: f32,
    restitution: f32,
}

pub const DEFAULT_GRAVITY: f32 = 9.807;
pub const DEFAULT_DAMPING: f32 = 0.85;
pub const DEFAULT_RESTITUTION: f32 = 0.225;

const INTERNAL_STEP_MULT: f32 = 3.2;

impl Default for ParticleOptions {
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

impl ParticleOptions {
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
pub struct ParticleSolver {
    options: ParticleOptions,
}

impl Default for ParticleSolver {
    fn default() -> Self {
        Self {
            options: Default::default(),
        }
    }
}

impl ParticleSolver {
    pub const fn new(options: ParticleOptions) -> Self {
        Self { options }
    }

    pub fn pre_pass_gravity(&self, forces: &mut [glam::Vec3]) {
        forces.iter_mut().for_each(|f| f.y -= self.options.gravity);
    }

    pub fn step(
        &self,
        positions: &mut [glam::Vec3],
        velocities: &mut [glam::Vec3],
        forces: &mut [glam::Vec3],
        masses: &[f32],
        delta: DeltaTime,
    ) {
        let h = delta.as_f32() * self.options.step_multiplier * INTERNAL_STEP_MULT;
        let h2 = h * h;

        velocities
            .iter_mut()
            .zip(forces.iter_mut())
            .zip(masses)
            .for_each(|((v, f), w)| {
                let force = std::mem::take(f);
                *v += h2 * force * w;
            });

        positions
            .iter_mut()
            .zip(velocities.iter())
            .for_each(|(p, v)| {
                *p += v * h;
            });

        let dh = self.options.damping * h;
        let dh2 = dh * dh;
        let dh2e = (-dh2).exp();
        velocities.iter_mut().for_each(|v| *v *= dh2e);

        if let Some(ground_level) = self.options.ground_level {
            positions.iter_mut().zip(velocities).for_each(|(p, v)| {
                if p.y < ground_level {
                    p.y = 0.0;
                    *v *= -self.options.restitution;
                }
            });
        }
    }
}
