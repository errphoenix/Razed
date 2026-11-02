use ethel::{
    Column, IndirectIndex,
    state::data::{ParallelIndexArrayColumn, column::IterColumn},
};
use janus::DeltaTime;

#[derive(Clone, Copy, Debug)]
pub struct PersistentForce {
    force: glam::Vec3,
    falloff: fn(f32) -> f32,
    progress: f32,
    duration_seconds: f32,
}
impl Default for PersistentForce {
    fn default() -> Self {
        Self {
            force: glam::Vec3::ZERO,
            falloff: |f| f,
            progress: 0f32,
            duration_seconds: 0f32,
        }
    }
}
impl PersistentForce {
    /// Persistent force with linear falloff.
    pub const fn new(force: glam::Vec3, duration_seconds: f32) -> Self {
        Self {
            force,
            duration_seconds,
            falloff: |t| 1.0 - t, // linear falloff
            progress: 0f32,
        }
    }

    /// Persistent force with custom falloff.
    pub const fn with_falloff(
        force: glam::Vec3,
        duration_seconds: f32,
        falloff: fn(f32) -> f32,
    ) -> Self {
        Self {
            force,
            duration_seconds,
            falloff,
            progress: 0f32,
        }
    }

    pub const fn tick(&mut self, delta: DeltaTime) {
        self.progress += delta.as_f32();
    }

    pub fn get(&self) -> glam::Vec3 {
        if !self.progress_inbound() {
            return glam::Vec3::ZERO;
        }
        let coeff = (self.falloff)(self.progress);
        self.force * coeff
    }

    pub const fn base_force(&self) -> glam::Vec3 {
        self.force
    }

    pub const fn duration_seconds(&self) -> f32 {
        self.duration_seconds
    }

    pub const fn progress(&self) -> f32 {
        self.progress
    }

    pub const fn falloff(&self) -> fn(f32) -> f32 {
        self.falloff
    }

    const fn progress_inbound(&self) -> bool {
        self.progress >= 0.0 && self.progress <= 1.0
    }
}

#[derive(Debug, Default)]
pub struct ForceTarget {
    pub force: IndirectIndex,
    pub entity: IndirectIndex,
}

#[derive(Debug, Default)]
pub struct ForcesRegistry {
    forces: ParallelIndexArrayColumn<PersistentForce>,
    targets_particle: Vec<ForceTarget>,
    targets_body: Vec<ForceTarget>,
    targets_lattice: Vec<ForceTarget>,
    clear_buf: Vec<IndirectIndex>,
}
impl ForcesRegistry {
    pub fn new() -> Self {
        Self {
            forces: ParallelIndexArrayColumn::new(),
            targets_particle: Vec::new(),
            targets_body: Vec::new(),
            targets_lattice: Vec::new(),
            clear_buf: Vec::new(),
        }
    }

    pub fn record(&mut self, pf: PersistentForce) -> IndirectIndex {
        self.forces.insert(pf)
    }

    pub fn add_target_particle(&mut self, fid: IndirectIndex, eid: IndirectIndex) {
        self.targets_particle.push(ForceTarget {
            force: fid,
            entity: eid,
        });
    }

    pub fn add_target_body(&mut self, fid: IndirectIndex, eid: IndirectIndex) {
        self.targets_body.push(ForceTarget {
            force: fid,
            entity: eid,
        });
    }

    pub fn add_target_lattice(&mut self, fid: IndirectIndex, eid: IndirectIndex) {
        self.targets_lattice.push(ForceTarget {
            force: fid,
            entity: eid,
        });
    }

    pub fn tick(&mut self, delta: DeltaTime) {
        self.forces.iter_mut().for_each(|e| e.tick(delta));
    }

    fn clear_targets(
        forces: &ParallelIndexArrayColumn<PersistentForce>,
        targets: &mut Vec<ForceTarget>,
    ) {
        targets.retain(|t| {
            let fid = t.force;
            if let Some(fdid) = forces.solve_indirect(fid) {
                let force = forces.contiguous()[fdid.as_index()];
                force.progress_inbound()
            } else {
                false
            }
        });
    }

    pub fn clear(&mut self) {
        for (force, &id) in self.forces.iter().zip(self.forces.handles().iter().skip(1)) {
            if !force.progress_inbound() {
                self.clear_buf.push(id);
            }
        }

        Self::clear_targets(&self.forces, &mut self.targets_particle);
        Self::clear_targets(&self.forces, &mut self.targets_body);
        Self::clear_targets(&self.forces, &mut self.targets_lattice);

        self.forces.free_many(&self.clear_buf);
        self.clear_buf.clear();
    }

    pub const fn forces_column(&self) -> &ParallelIndexArrayColumn<PersistentForce> {
        &self.forces
    }

    pub const fn forces_column_mut(&mut self) -> &mut ParallelIndexArrayColumn<PersistentForce> {
        &mut self.forces
    }

    pub fn forces(&self) -> &[PersistentForce] {
        self.forces.contiguous()
    }

    pub fn forces_mut(&mut self) -> &mut [PersistentForce] {
        self.forces.contiguous_mut()
    }

    pub fn targets_particle(&self) -> &[ForceTarget] {
        &self.targets_particle
    }

    pub fn targets_body(&self) -> &[ForceTarget] {
        &self.targets_body
    }

    pub fn targets_lattice(&self) -> &[ForceTarget] {
        &self.targets_lattice
    }
}
