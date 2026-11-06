use std::{
    sync::LazyLock,
    time::{Duration, Instant},
};

use ethel::{
    Column, DirectIndex, FxLsSpatialHash, IndirectIndex, SpatialHash,
    state::data::hash::ParIterSpatialHash,
};
use janus::{
    DeltaTime,
    jobs::buffered::{BufferedRoutine, WorkBuffers},
};
use physics::{
    collision::LightCollision,
    force::{ForceTarget, ForcesRegistry},
    rigid::{RbVelocity, RigidBodySolver},
};
use rayon::iter::ParallelIterator;

use crate::simulation::DebrisSpatialHash;

const MOTION_ACCUM_BUCKET_SIZE: Duration = Duration::from_millis(300);
const MOTION_ACCUM_BUCKET_COUNT: usize = 6;

#[derive(Clone, Copy, Debug)]
pub struct MotionAccumulator {
    pub window: ethel::state::time::AccumulationWindow<{ MOTION_ACCUM_BUCKET_COUNT }, f32>,
    pub last_position: glam::Vec3,
}

impl Default for MotionAccumulator {
    fn default() -> Self {
        Self {
            window: ethel::state::time::AccumulationWindow::new(MOTION_ACCUM_BUCKET_SIZE),
            last_position: glam::Vec3::default(),
        }
    }
}

ethel::table_spec! {
    struct Debris {
        // seconds since spawned
        age: f32;

        position: glam::Vec3;
        rotation: glam::Quat;

        velocity: RbVelocity;

        forces: glam::Vec3;
        torques: glam::Vec3;

        mass: f32;
        inv_inertia_loc: glam::Mat3;
        inv_inertia_abs: glam::Mat3;

        volume: physics::Sphere;

        motion: MotionAccumulator;

        mesh_id: ethel::mesh::Id;
    }
}

ethel::table_spec! {
    struct Rubber {
        // seconds since spawned (nanosecond detail)
        // includes its original age as non-rubber debris
        age: f32;

        position: glam::Vec3;
        rotation: glam::Quat;

        volume: physics::Sphere;

        mesh_id: ethel::mesh::Id;

        volume_id: IndirectIndex;
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CollisionBuffers {
    pub debris: DebrisBuffer,
    pub rubber: Vec<RubberStaticData>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DebrisBuffer {
    pub positions: Vec<glam::Vec3>,
    pub volumes: Vec<::physics::Sphere>,
    pub direct_indices: Vec<DirectIndex>,
}
#[allow(unused)]
impl DebrisBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            positions: Vec::with_capacity(capacity),
            volumes: Vec::with_capacity(capacity),
            direct_indices: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, position: glam::Vec3, volume: ::physics::Sphere, index: DirectIndex) {
        self.positions.push(position);
        self.volumes.push(volume);
        self.direct_indices.push(index);
    }

    pub fn clear(&mut self) {
        self.positions.clear();
        self.volumes.clear();
        self.direct_indices.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.direct_indices.is_empty()
    }

    pub fn len(&self) -> usize {
        self.direct_indices.len()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RubberStaticData {
    pub position: glam::Vec3,
    pub volume: ::physics::Sphere,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct RubberStaticEntity {
    pub data: RubberStaticData,
    pub handle: IndirectIndex,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct RubberVolumeStage {
    entities: Vec<RubberStaticEntity>,
}
impl RubberVolumeStage {
    #[allow(unused)]
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(unused)]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entities: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, position: glam::Vec3, volume: ::physics::Sphere, handle: IndirectIndex) {
        self.entities.push(RubberStaticEntity {
            data: RubberStaticData { position, volume },
            handle,
        });
    }

    #[allow(unused)]
    pub fn clear(&mut self) {
        self.entities.clear();
    }

    pub fn drain(&mut self) -> std::vec::Drain<'_, RubberStaticEntity> {
        self.entities.drain(..)
    }
}

const DEBRIS_FREEZE_TIME_THRESHOLD: f32 = 8.0;
const DEBRIS_FREEZE_MOVE_THRESHOLD: f32 = 0.05;

#[derive(Debug)]
pub struct DebrisSystem {
    pub(crate) data: DebrisRowTable,
    pub(crate) physics: RigidBodySolver,
    pub(crate) hashed: FxLsSpatialHash<IndirectIndex>,

    collision_job: LazyLock<BufferedRoutine<CollisionBuffers, LightCollision>>,

    trash_buffer: Vec<IndirectIndex>,

    rubber_volume_stage: RubberVolumeStage,
    rubber: RubberRowTable,
}

impl Default for DebrisSystem {
    fn default() -> Self {
        Self {
            data: DebrisRowTable::default(),
            physics: RigidBodySolver::default(),
            hashed: FxLsSpatialHash::new(super::HASH_RESOLUTION),

            trash_buffer: Vec::new(),
            rubber_volume_stage: RubberVolumeStage::default(),
            rubber: RubberRowTable::default(),

            collision_job: LazyLock::new(|| BufferedRoutine::new(rayon::current_num_threads())),
        }
    }
}
impl DebrisSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: DebrisRowTable::with_capacity(capacity),
            physics: RigidBodySolver::default(),
            hashed: FxLsSpatialHash::with_capacity(super::HASH_RESOLUTION, capacity),

            trash_buffer: Vec::with_capacity(capacity / 2),
            rubber_volume_stage: RubberVolumeStage::new(),
            rubber: RubberRowTable::with_capacity(capacity / 2),

            collision_job: LazyLock::new(|| BufferedRoutine::new(rayon::current_num_threads())),
        }
    }

    /// Get the total amount of debris across dynamic debris and static
    /// debris (rubber).
    ///
    /// This subtracts `2` from the sum of the lengths of the `debris`
    /// and `rubber` to exclude degenerate elements.
    pub fn total_debris_count(&self) -> usize {
        self.data.len() + self.rubber.len() - 2
    }

    pub fn data(&self) -> &DebrisRowTable {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut DebrisRowTable {
        &mut self.data
    }

    pub fn rubber(&self) -> &RubberRowTable {
        &self.rubber
    }

    pub const fn solver(&self) -> &RigidBodySolver {
        &self.physics
    }

    pub fn integrate_global_forces(&mut self, forces: &ForcesRegistry) {
        const TORQUE_COEFF: f32 = 0.05;
        forces
            .targets_body()
            .iter()
            .for_each(|&ForceTarget { force, entity }| {
                if let Some(fdid) = forces.forces_column().solve_indirect(force)
                    && let Some(edid) = self.data.solve_indirect(entity)
                {
                    let force = forces.forces()[fdid.as_index()].get();
                    let out_forces = &mut self.data.forces[edid.as_index()];
                    let out_torque = &mut self.data.torques[edid.as_index()];
                    *out_forces += force;
                    *out_torque += force.any_orthogonal_vector() * TORQUE_COEFF;
                }
            });
    }

    pub fn clear_rubber(&mut self) {
        for i in (1..self.rubber.len()).rev() {
            let handle = self.rubber.handles[i];
            let volume_id = self.rubber.volume_id[i];
            self.rubber.free(handle);
            self.physics.remove_static_volume(volume_id);
        }
    }

    pub const fn hashed(&self) -> &DebrisSpatialHash {
        &self.hashed
    }

    pub const fn hashed_mut(&mut self) -> &DebrisSpatialHash {
        &mut self.hashed
    }

    pub fn delete_rubber(&mut self, handle: IndirectIndex) {
        let direct = self.rubber.solve_indirect(handle);

        #[cfg(any(feature = "devmode", debug_assertions))]
        if direct.is_none() {
            tracing::event!(
                tracing::Level::ERROR,
                "Tried to delete invalid rubber entity {handle:?}."
            );
        }

        let direct = direct.unwrap();
        let volume_id = self.rubber.volume_id[direct.as_index()];

        #[cfg(any(feature = "devmode", debug_assertions))]
        if volume_id.as_int() == 0 {
            tracing::event!(
                tracing::Level::ERROR,
                "Volume ID for rubber entity {handle:?} is, unexpectedbly, unitialized."
            );
            return;
        }

        self.rubber.free(handle);
        self.physics.remove_static_volume(volume_id);
    }

    pub fn hash_debris(&mut self) {
        self.hashed.clear();

        let pos = self.data.position_view();
        let bounds = self.data.volume_view();
        let handles = self.data.handles_view();
        let debris = pos.join(bounds).join(handles);

        for (&pos, &_bounds, &handle) in debris.into_iter() {
            let cell = self.hashed.cell_at(pos);
            self.hashed.put_element(cell, handle);
        }
    }

    pub fn simulate_bodies(&mut self, delta: DeltaTime) {
        // register pending static entites
        for RubberStaticEntity { data, handle } in self.rubber_volume_stage.drain() {
            let volume_id = self.physics.add_static_volume(data.position, data.volume);

            let direct = self.rubber.solve_indirect(handle).unwrap();
            self.rubber.volume_id[direct.as_index()] = volume_id;
        }

        {
            let positions = &self.data.position;
            let volumes = &self.data.volume;

            let rubber_par_iter = self.physics.static_volumes_hash().par_iter();
            let par_iter = self
                .hashed
                .par_iter()
                .chain(rubber_par_iter)
                .filter_map(|(&cell, bucket)| (!bucket.is_empty()).then(|| cell));

            let static_hash = self.physics.static_volumes_hash();
            let static_data = self.physics.static_volumes_data();

            self.collision_job
                .dispatch_jobs(par_iter, |WorkBuffers { buffer, result }, cell| {
                    let debris = &mut buffer.debris;
                    let rubber = &mut buffer.rubber;

                    // dynamic - dynamic
                    {
                        // reconstruct debris buffer
                        debris.clear();
                        if let Some(bucket) = self.hashed.get(cell) {
                            bucket.iter().for_each(|i| {
                                let direct = unsafe { self.data.solve_indirect_unchecked(*i) };
                                let position = positions[direct.as_index()];
                                let volume = volumes[direct.as_index()];
                                debris.push(position, volume, direct);
                            });
                        }

                        let DebrisBuffer {
                            positions,
                            volumes,
                            direct_indices,
                        } = &debris;

                        ::physics::collision::detect_n2(positions, volumes, direct_indices, result);
                    }

                    // static - dynamic
                    {
                        if !debris.is_empty() {
                            // reconstruct rubber buffer
                            rubber.clear();
                            if let Some(bucket) = static_hash.get(cell) {
                                bucket.iter().for_each(|i| {
                                    if let Some(direct) = self.rubber.solve_indirect(*i) {
                                        let position = static_data.position[direct.as_index()];
                                        let volume = static_data.volume[direct.as_index()];
                                        rubber.push(RubberStaticData { position, volume });
                                    }
                                });
                            }

                            let rubber = &buffer.rubber;

                            for i in 0..debris.len() {
                                let p0 = debris.positions[i];
                                let v0 = debris.volumes[i];
                                let id0 = debris.direct_indices[i];

                                // static body needs no id
                                let id1 = DirectIndex::default();

                                for &RubberStaticData {
                                    position: p1,
                                    volume: v1,
                                } in rubber
                                {
                                    result.extend(::physics::collision::detect_sphere(
                                        p0, p1, v0, v1, id0, id1,
                                    ));
                                }
                            }
                        }
                    }
                });

            let contacts = self.physics.collision_buffer_mut();

            self.collision_job
                .thread_buffers()
                .buffers_mut()
                .for_each(|buffer| {
                    let thread_results = &mut buffer.result;
                    thread_results.drain(..).for_each(|contact| {
                        contacts.push(contact);
                    });
                });
        }

        let positions = &mut self.data.position;
        let rotations = &mut self.data.rotation;
        let velocities = &mut self.data.velocity;
        let forces = &mut self.data.forces;
        let torques = &mut self.data.torques;
        let masses = &self.data.mass;
        let inv_inertia_loc = &self.data.inv_inertia_loc;
        let inv_inertia_abs = &mut self.data.inv_inertia_abs;

        self.physics.solve_collisions(positions, velocities, masses);

        self.physics.apply_gravity(forces, velocities);
        self.physics
            .sync_inertia(rotations, inv_inertia_loc, inv_inertia_abs);

        self.physics
            .integrate(velocities, forces, torques, masses, inv_inertia_abs, delta);
        self.physics
            .update_bodies(positions, rotations, velocities, delta);

        self.physics.damp_velocity(velocities, delta);
        self.physics.constrain_ground(positions, velocities);
    }

    pub fn accumulate_motion(&mut self) {
        let time = Instant::now();

        let positions = &mut self.data.position;
        let accums = &mut self.data.motion;

        for (
            live_position,
            MotionAccumulator {
                window,
                last_position,
            },
        ) in positions.iter_mut().zip(accums)
        {
            let d = *live_position - *last_position;
            let f = d.length_squared();
            window.register(f, time);
            *last_position = *live_position;
        }
    }

    pub fn freeze_old_debris(&mut self, delta: DeltaTime) {
        let age = &mut self.data.age;
        let handles = &self.data.handles;
        let motion = &self.data.motion;

        for ((age, motion), &id) in age.iter_mut().zip(motion).zip(handles).skip(1) {
            if motion.window.accumulated() < DEBRIS_FREEZE_MOVE_THRESHOLD
                && *age > DEBRIS_FREEZE_TIME_THRESHOLD
            {
                self.trash_buffer.push(id);
            }

            *age += delta.as_f32();
        }

        self.trash_buffer.drain(..).for_each(|debris_id| {
            if debris_id.as_int() == 0 {
                return;
            }
            if let Some(direct) = self.data.solve_indirect(debris_id) {
                let position = self.data.position[direct.as_index()];
                let rotation = self.data.rotation[direct.as_index()];
                let volume = self.data.volume[direct.as_index()];
                let age = self.data.age[direct.as_index()];
                let mesh_id = self.data.mesh_id[direct.as_index()];

                let rubber_data = (
                    age,
                    position,
                    rotation,
                    volume,
                    mesh_id,
                    IndirectIndex::default(),
                );
                let handle = self.rubber.insert(rubber_data);
                self.rubber_volume_stage.push(position, volume, handle);
                self.data.free(debris_id);
            }
        });
    }

    pub fn apply_impulse(
        &mut self,
        index: IndirectIndex,
        linear: Option<glam::Vec3>,
        angular: Option<glam::Vec3>,
    ) {
        if linear.is_none() && angular.is_none() {
            return;
        }

        if let Some(node) = self.data.solve_indirect(index) {
            let mass = self.data.mass[node.as_index()];
            let inertia = self.data.inv_inertia_abs[node.as_index()];
            let out_vel = &mut self.data.velocity[node.as_index()];
            if let Some(linear) = linear {
                out_vel.linear += linear * mass;
            }
            if let Some(angular) = angular {
                out_vel.angular += inertia * angular;
            }
        }
    }

    pub fn apply_impulse_multi(
        &mut self,
        indices: &[IndirectIndex],
        linear: Option<glam::Vec3>,
        angular: Option<glam::Vec3>,
    ) {
        if linear.is_none() && angular.is_none() {
            return;
        }
        for &index in indices {
            self.apply_impulse(index, linear, angular);
        }
    }

    pub fn apply_impulse_batched(
        &mut self,
        linear: Option<glam::Vec3>,
        angular: Option<glam::Vec3>,
    ) {
        if linear.is_none() && angular.is_none() {
            return;
        }
        let linear = linear.unwrap_or_default();
        let angular = angular.unwrap_or_default();
        let mass = &self.data.mass;
        let inertia = &self.data.inv_inertia_abs;
        let velocity = &mut self.data.velocity;
        for (vel, (&w, &i)) in velocity.iter_mut().zip(mass.iter().zip(inertia)).skip(1) {
            vel.linear += linear * w;
            vel.angular += i * angular;
        }
    }
}
