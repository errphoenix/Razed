use std::{
    sync::LazyLock,
    time::{Duration, Instant},
};

use ethel::state::data::{
    Column, DirectIndex, IndirectIndex,
    hash::{FxLsSpatialHash, SpatialResolution},
};
use janus::{
    context::DeltaTime,
    jobs::buffered::{BufferedRoutine, WorkBuffers},
};
use physics::{
    collision::LightCollision,
    rigid::{RbVelocity, RigidBodySolver},
};
use rayon::iter::ParallelIterator;

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
pub struct DebrisVolumeBuffer {
    pub positions: Vec<glam::Vec3>,
    pub volumes: Vec<::physics::Sphere>,
    pub direct_indices: Vec<DirectIndex>,
}

impl DebrisVolumeBuffer {
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
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct RubberStaticEntity {
    pub position: glam::Vec3,
    pub volume: ::physics::Sphere,
    pub handle: IndirectIndex,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct RubberVolumeStage {
    entities: Vec<RubberStaticEntity>,
}

impl RubberVolumeStage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entities: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, position: glam::Vec3, volume: ::physics::Sphere, handle: IndirectIndex) {
        self.entities.push(RubberStaticEntity {
            position,
            volume,
            handle,
        });
    }

    pub fn clear(&mut self) {
        self.entities.clear();
    }

    pub fn drain(&mut self) -> std::vec::Drain<'_, RubberStaticEntity> {
        self.entities.drain(..)
    }
}

const DEBRIS_FREEZE_TIME_THRESHOLD: f32 = 4.0;
const DEBRIS_FREEZE_MOVE_THRESHOLD: f32 = 0.75;
pub const HASH_RESOLUTION: SpatialResolution = SpatialResolution::new(4.0);

#[derive(Debug)]
pub struct DebrisSystem {
    debris: DebrisRowTable,
    debris_phys: RigidBodySolver,
    debris_hash: FxLsSpatialHash<DirectIndex>,

    collision_job: LazyLock<BufferedRoutine<DebrisVolumeBuffer, LightCollision>>,

    debris_trash_buffer: Vec<IndirectIndex>,

    rubber_volume_stage: RubberVolumeStage,
    rubber: RubberRowTable,
}

impl Default for DebrisSystem {
    fn default() -> Self {
        Self {
            debris: DebrisRowTable::default(),
            debris_phys: RigidBodySolver::default(),
            debris_hash: FxLsSpatialHash::new(HASH_RESOLUTION),
            collision_job: LazyLock::new(|| BufferedRoutine::new(rayon::current_num_threads())),
            debris_trash_buffer: Vec::new(),
            rubber_volume_stage: RubberVolumeStage::default(),
            rubber: RubberRowTable::default(),
        }
    }
}

impl DebrisSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            debris: DebrisRowTable::with_capacity(capacity),
            debris_hash: FxLsSpatialHash::with_capacity(HASH_RESOLUTION, capacity),
            collision_job: LazyLock::new(|| BufferedRoutine::new(rayon::current_num_threads())),
            debris_trash_buffer: Vec::with_capacity(capacity / 2),
            rubber_volume_stage: RubberVolumeStage::new(),
            rubber: RubberRowTable::with_capacity(capacity / 2),
            ..Default::default()
        }
    }

    /// Get the total amount of debris across dynamic debris and static
    /// debris (rubber).
    ///
    /// This subtracts `2` from the sum of the lengths of the `debris`
    /// and `rubber` to exclude degenerate elements.
    pub fn total_debris_count(&self) -> usize {
        self.debris.len() + self.rubber.len() - 2
    }

    pub fn data(&self) -> &DebrisRowTable {
        &self.debris
    }

    pub fn data_mut(&mut self) -> &mut DebrisRowTable {
        &mut self.debris
    }

    pub fn rubber(&self) -> &RubberRowTable {
        &self.rubber
    }

    pub fn clear_rubber(&mut self) {
        for i in (1..self.rubber.len()).rev() {
            let handle = self.rubber.handles[i];
            let volume_id = self.rubber.volume_id[i];
            self.rubber.free(handle);
            self.debris_phys.remove_static_volume(volume_id);
        }
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
        self.debris_phys.remove_static_volume(volume_id);
    }

    pub fn hash_debris(&mut self) {
        self.debris_hash.clear();

        let pos = self.debris.position_view();
        let bounds = self.debris.volume_view();
        let handles = self.debris.handles_view();
        let debris = pos.join(bounds).join(handles);

        for (i, (&pos, &_bounds, &handle)) in debris.into_iter().enumerate() {
            let cell = self.debris_hash.cell_at(pos);
            let direct_id = DirectIndex::from_index(i, handle.generation());
            self.debris_hash.put(cell, direct_id);

            // let cells = self.debris_hash.aligned_adjacent_cells(pos);

            // let direct_id = DirectIndex::from_index(i, handle.generation());
            // for cell in cells {
            //     let aabb = {
            //         let entry = self.spatial_bounds_cache.entry(cell);
            //         let res = self.debris_hash.resolution;
            //         entry.or_insert_with(|| Aabb::from_cell(cell, res))
            //     };

            //     if aabb.intersects_sphere(bounds, pos) {
            //         self.debris_hash.put(cell, direct_id);
            //     }
            // }
        }
    }

    pub fn simulate_bodies(
        &mut self,
        delta: DeltaTime,
        #[cfg(feature = "devmode")] profiler: &mut ethel::profile::Profiler,
    ) {
        let positions = &mut self.debris.position;
        let rotations = &mut self.debris.rotation;
        let velocities = &mut self.debris.velocity;
        let forces = &mut self.debris.forces;
        let torques = &mut self.debris.torques;
        let masses = &self.debris.mass;
        let inv_inertia_loc = &self.debris.inv_inertia_loc;
        let inv_inertia_abs = &mut self.debris.inv_inertia_abs;
        let volumes = &self.debris.volume;


        // single threaded
        {
            // self.debris_hash
            //     .elements()
            //     .filter(|vec| !vec.is_empty())
            //     .for_each(|debris| {
            //         self.debris_volume_buffer.clear();

            //         debris.iter().for_each(|index| {
            //             let position = positions[index.as_index()];
            //             let volume = volumes[index.as_index()];
            //             self.debris_volume_buffer.push(position, volume, *index);
            //         });

            //         let DebrisVolumeBuffer {
            //             positions,
            //             volumes,
            //             direct_indices,
            //         } = &self.debris_volume_buffer;

            //         self.debris_phys
            //             .detect_collisions(positions, volumes, direct_indices);
            //     });
        }

        {
            let par_iter = self
                .debris_hash
                .par_iter()
                .filter_map(|(_, bucket)| (!bucket.is_empty()).then(|| bucket));

            self.collision_job
                .dispatch_jobs(par_iter, |WorkBuffers { buffer, result }, debris| {
                    buffer.clear();

                    debris.iter().for_each(|index| {
                        let position = positions[index.as_index()];
                        let volume = volumes[index.as_index()];
                        buffer.push(position, volume, *index);
                    });

                    let DebrisVolumeBuffer {
                        positions,
                        volumes,
                        direct_indices,
                    } = &buffer;

                    ::physics::collision::detect_n2(positions, volumes, direct_indices, result);
                });
        }


        {
            for RubberStaticEntity {
                position,
                volume,
                handle,
            } in self.rubber_volume_stage.drain()
            {
                let volume_id = self.debris_phys.add_static_volume(position, volume);
                let direct = self.rubber.solve_indirect(handle).unwrap();
                self.rubber.volume_id[direct.as_index()] = volume_id;
            }

            self.debris_phys
                .detect_static_collisions(&self.debris_hash, positions, volumes);
        }

        self.debris_phys
            .solve_collisions(positions, velocities, masses);

        self.debris_phys.apply_gravity(forces);
        self.debris_phys
            .sync_inertia(rotations, inv_inertia_loc, inv_inertia_abs);

        self.debris_phys
            .integrate(velocities, forces, torques, masses, inv_inertia_abs, delta);
        self.debris_phys
            .update_bodies(positions, rotations, velocities, delta);

        self.debris_phys.damp_velocity(velocities, delta);
        self.debris_phys.constrain_ground(positions, velocities);
    }

    pub fn accumulate_motion(&mut self) {
        let time = Instant::now();

        let positions = &mut self.debris.position;
        let accums = &mut self.debris.motion;

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
        let age = &mut self.debris.age;
        let handles = &self.debris.handles;
        let motion = &self.debris.motion;

        for ((age, motion), &id) in age.iter_mut().zip(motion).zip(handles).skip(1) {
            if motion.window.accumulated() < DEBRIS_FREEZE_MOVE_THRESHOLD
                && *age > DEBRIS_FREEZE_TIME_THRESHOLD
            {
                self.debris_trash_buffer.push(id);
            }

            *age += delta.as_f32();
        }

        self.debris_trash_buffer.drain(..).for_each(|debris_id| {
            if debris_id.as_int() == 0 {
                return;
            }
            let direct = self.debris.solve_indirect(debris_id).unwrap();

            let position = self.debris.position[direct.as_index()];
            let rotation = self.debris.rotation[direct.as_index()];
            let volume = self.debris.volume[direct.as_index()];
            let age = self.debris.age[direct.as_index()];
            let mesh_id = self.debris.mesh_id[direct.as_index()];

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
            self.debris.free(debris_id);
        });
    }
}
