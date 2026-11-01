use ethel::{
    Column, IndirectIndex,
    state::data::{ParallelIndexArrayColumn, column::IterColumn},
};
use janus::DeltaTime;

#[derive(Clone, Copy, Debug)]
pub struct PersistentImpulse {
    force: glam::Vec3,
    falloff: fn(f32) -> f32,
    progress: f32,
    duration_seconds: f32,
}
impl Default for PersistentImpulse {
    fn default() -> Self {
        Self {
            force: glam::Vec3::ZERO,
            falloff: |f| f,
            progress: 0f32,
            duration_seconds: 0f32,
        }
    }
}
impl PersistentImpulse {
    /// Persistent impulse with linear falloff.
    pub const fn new(force: glam::Vec3, duration_seconds: f32) -> Self {
        Self {
            force,
            duration_seconds,
            falloff: |t| 1.0 - t, // linear falloff
            progress: 0f32,
        }
    }

    /// Persistent impulse with custom falloff.
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
pub struct ImpulseTarget {
    pub impulse: IndirectIndex,
    pub entity: IndirectIndex,
}

#[derive(Debug, Default)]
pub struct ImpulseWorld {
    impulses: ParallelIndexArrayColumn<PersistentImpulse>,
    targets_particle: Vec<ImpulseTarget>,
    targets_body: Vec<ImpulseTarget>,
    targets_lattice: Vec<ImpulseTarget>,
    clear_buf: Vec<IndirectIndex>,
}
impl ImpulseWorld {
    pub fn new() -> Self {
        Self {
            impulses: ParallelIndexArrayColumn::new(),
            targets_particle: Vec::new(),
            targets_body: Vec::new(),
            targets_lattice: Vec::new(),
            clear_buf: Vec::new(),
        }
    }

    pub fn record_impulse(&mut self, pi: PersistentImpulse) -> IndirectIndex {
        self.impulses.insert(pi)
    }

    pub fn add_target_particle(&mut self, iid: IndirectIndex, eid: IndirectIndex) {
        self.targets_particle.push(ImpulseTarget {
            impulse: iid,
            entity: eid,
        });
    }

    pub fn add_target_body(&mut self, iid: IndirectIndex, eid: IndirectIndex) {
        self.targets_body.push(ImpulseTarget {
            impulse: iid,
            entity: eid,
        });
    }

    pub fn add_target_lattice(&mut self, iid: IndirectIndex, eid: IndirectIndex) {
        self.targets_lattice.push(ImpulseTarget {
            impulse: iid,
            entity: eid,
        });
    }

    pub fn tick(&mut self, delta: DeltaTime) {
        self.impulses.iter_mut().for_each(|e| e.tick(delta));
    }

    fn clear_targets(
        impulses: &ParallelIndexArrayColumn<PersistentImpulse>,
        targets: &mut Vec<ImpulseTarget>,
    ) {
        targets.retain(|t| {
            let iid = t.impulse;
            if let Some(idid) = impulses.solve_indirect(iid) {
                let impulse = impulses.contiguous()[idid.as_index()];
                impulse.progress_inbound()
            } else {
                false
            }
        });
    }

    pub fn clear(&mut self) {
        for (impulse, &id) in self
            .impulses
            .iter()
            .zip(self.impulses.handles().iter().skip(1))
        {
            if !impulse.progress_inbound() {
                self.clear_buf.push(id);
            }
        }

        Self::clear_targets(&self.impulses, &mut self.targets_particle);
        Self::clear_targets(&self.impulses, &mut self.targets_body);
        Self::clear_targets(&self.impulses, &mut self.targets_lattice);

        self.impulses.free_many(&self.clear_buf);
        self.clear_buf.clear();
    }

    pub const fn impulses_column(&self) -> &ParallelIndexArrayColumn<PersistentImpulse> {
        &self.impulses
    }

    pub const fn impulses_column_mut(
        &mut self,
    ) -> &mut ParallelIndexArrayColumn<PersistentImpulse> {
        &mut self.impulses
    }

    pub fn impulses(&self) -> &[PersistentImpulse] {
        self.impulses.contiguous()
    }

    pub fn impulses_mut(&mut self) -> &mut [PersistentImpulse] {
        self.impulses.contiguous_mut()
    }

    pub fn targets_particle(&self) -> &[ImpulseTarget] {
        &self.targets_particle
    }

    pub fn targets_body(&self) -> &[ImpulseTarget] {
        &self.targets_body
    }

    pub fn targets_lattice(&self) -> &[ImpulseTarget] {
        &self.targets_lattice
    }
}
