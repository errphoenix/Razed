use janus::context::DeltaTime;

#[derive(Clone, Copy, Debug)]
pub struct ParticleOptions {
    gravity: f32,
    ground_level: Option<f32>,
    step_multiplier: f32,
    damping: f32,
}

pub const DEFAULT_GRAVITY: f32 = 9.807;
pub const DEFAULT_DAMPING: f32 = 0.996;

impl Default for ParticleOptions {
    fn default() -> Self {
        Self {
            gravity: DEFAULT_GRAVITY,
            ground_level: Some(0.0),
            step_multiplier: 1.0,
            damping: DEFAULT_DAMPING,
        }
    }
}

impl ParticleOptions {
    pub fn new(
        gravity: f32,
        ground_level: Option<f32>,
        step_multiplier: f32,
        damping: f32,
    ) -> Self {
        Self {
            gravity,
            ground_level,
            step_multiplier,
            damping,
        }
    }

    pub fn with_gravity(self, gravity: f32) -> Self {
        Self {
            gravity,
            ground_level: self.ground_level,
            step_multiplier: self.step_multiplier,
            damping: self.damping,
        }
    }

    pub fn with_ground_level(self, ground_level: Option<f32>) -> Self {
        Self {
            ground_level,
            gravity: self.gravity,
            step_multiplier: self.step_multiplier,
            damping: self.damping,
        }
    }

    pub fn with_step_multiplier(self, step_multiplier: f32) -> Self {
        Self {
            step_multiplier,
            gravity: self.gravity,
            ground_level: self.ground_level,
            damping: self.damping,
        }
    }

    pub fn with_damping(self, damping: f32) -> Self {
        Self {
            damping,
            gravity: self.gravity,
            ground_level: self.ground_level,
            step_multiplier: self.step_multiplier,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParticleSolver {
    options: ParticleOptions,
    substeps: u32,

    h: f32,
    h2: f32,
}

pub const DEFAULT_SOLVE_ITERATIONS: u32 = 4;

impl Default for ParticleSolver {
    fn default() -> Self {
        Self {
            options: Default::default(),
            substeps: DEFAULT_SOLVE_ITERATIONS,
            h: 0.0,
            h2: 0.0,
        }
    }
}

impl ParticleSolver {
    pub fn new(options: ParticleOptions, substeps: u32) -> Self {
        Self {
            options,
            substeps,
            ..Default::default()
        }
    }

    #[inline]
    pub fn set_step_time(&mut self, delta: DeltaTime) {
        self.h = delta.as_f32() / self.substeps as f32;
        self.h2 = self.h * self.h;
    }

    pub fn pre_pass_gravity(&self, forces: &mut [glam::Vec3]) {
        forces.iter_mut().for_each(|f| *f += self.options.gravity);
    }

    pub fn step(
        &self,
        positions: &mut [glam::Vec3],
        velocities: &mut [glam::Vec3],
        forces: &mut [glam::Vec3],
        masses: &[f32],
    ) {
        let h = self.h * self.options.step_multiplier;
        let h2 = self.h2 * self.options.step_multiplier;

        velocities
            .iter_mut()
            .zip(forces.iter_mut())
            .zip(masses)
            .for_each(|((v, f), w)| {
                let force = std::mem::take(f);
                *v += h2 * force * w;
            });

        for _ in 0..self.substeps {
            positions
                .iter_mut()
                .zip(velocities.iter())
                .for_each(|(p, v)| {
                    *p += v * h;
                });

            velocities
                .iter_mut()
                .for_each(|v| *v *= self.options.damping);
        }

        if let Some(ground_level) = self.options.ground_level {
            positions.iter_mut().for_each(|p| {
                if p.y < ground_level {
                    p.y = 0.0;
                }
            });
        }
    }
}
