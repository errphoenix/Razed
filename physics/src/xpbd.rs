use ethel::state::data::table::QuatViewMut;
use janus::context::DeltaTime;

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct PredictedPosition(glam::Vec3);

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Position(glam::Vec3);

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Mass(f32);

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct InverseMass(f32);

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct ExternalForces(glam::Vec3);

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Velocity(glam::Vec3);

ethel::table_spec! {
    struct Nodes {
        next_positions: PredictedPosition;
        positions: Position;
        masses: Mass;
        inverse_masses: InverseMass;
        external_forces: ExternalForces;
        velocities: Velocity;
    }
}

pub struct XpbdSolver {
    substeps: u32,
    h: f32,
}

impl XpbdSolver {
    pub fn new(substeps: u32) -> Self {
        Self { substeps, h: 0.0 }
    }

    pub fn step_time(&mut self, delta: DeltaTime) {
        self.h = delta.as_f32() / self.substeps as f32;
    }

    pub fn predict_positions<'data>(
        &self,
        data: &'data mut QuatViewMut<
            'data,
            NodesTableDef,
            PredictedPosition,
            Position,
            ExternalForces,
            Velocity,
        >,
    ) {
        let h = self.h;
        data.iter_mut().for_each(|((np, p), (f, v))| {
            np.0 = p.0 + h * v.0 + h * h * f.0;
        });
    }
}
