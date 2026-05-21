use std::time::{Duration, Instant};

use ethel::state::data::{
    Column, DirectIndex, IndirectIndex,
    hash::{FxLsSpatialHash, SpatialResolution},
};
use janus::context::DeltaTime;
use physics::rigid::{RbVelocity, RigidBodySolver};

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

const DEBRIS_FREEZE_TIME_THRESHOLD: f32 = 8.0;
const DEBRIS_FREEZE_MOVE_THRESHOLD: f32 = 0.35;
const HASH_RESOLUTION: SpatialResolution = SpatialResolution::new(5.0);

#[derive(Debug)]
pub struct DebrisSystem {
    debris: DebrisRowTable,
    debris_phys: RigidBodySolver,
    debris_hash: FxLsSpatialHash<DirectIndex>,

    debris_volume_buffer: DebrisVolumeBuffer,
    debris_trash_buffer: Vec<IndirectIndex>,

    rubber: RubberRowTable,
}

impl Default for DebrisSystem {
    fn default() -> Self {
        Self {
            debris: DebrisRowTable::default(),
            debris_phys: RigidBodySolver::default(),
            debris_hash: FxLsSpatialHash::new(HASH_RESOLUTION),
            debris_volume_buffer: DebrisVolumeBuffer::default(),
            debris_trash_buffer: Vec::new(),
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
            debris_phys: RigidBodySolver::default(),
            debris_hash: FxLsSpatialHash::with_capacity(HASH_RESOLUTION, capacity),
            debris_volume_buffer: DebrisVolumeBuffer::new(),
            debris_trash_buffer: Vec::with_capacity(capacity / 2),
            rubber: RubberRowTable::with_capacity(capacity / 2),
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

    pub fn rubber_mut(&mut self) -> &mut RubberRowTable {
        &mut self.rubber
    }

    pub fn hash_debris(&mut self) {
        self.debris_hash.clear();

        let debris_pos = self.debris.position_view().join(self.debris.handles_view());
        debris_pos
            .into_iter()
            .enumerate()
            .for_each(|(i, (&pos, &handle))| {
                let cell = self.debris_hash.cell_at(pos);
                let direct_id = DirectIndex::from_index(i, handle.generation());
                self.debris_hash.put(cell, direct_id);
            });
    }

    pub fn simulate_bodies(&mut self, delta: DeltaTime) {
        let positions = &mut self.debris.position;
        let rotations = &mut self.debris.rotation;
        let velocities = &mut self.debris.velocity;
        let forces = &mut self.debris.forces;
        let torques = &mut self.debris.torques;
        let masses = &self.debris.mass;
        let inv_inertia_loc = &self.debris.inv_inertia_loc;
        let inv_inertia_abs = &mut self.debris.inv_inertia_abs;
        let volumes = &self.debris.volume;

        self.debris_hash
            .elements()
            .filter(|vec| !vec.is_empty())
            .for_each(|debris| {
                self.debris_volume_buffer.clear();

                debris.iter().for_each(|index| {
                    let position = positions[index.as_index()];
                    let volume = volumes[index.as_index()];
                    self.debris_volume_buffer.push(position, volume, *index);
                });

                let DebrisVolumeBuffer {
                    positions,
                    volumes,
                    direct_indices,
                } = &self.debris_volume_buffer;

                self.debris_phys
                    .detect_collisions(positions, volumes, direct_indices);
            });

        {
            self.debris_phys.clear_static_volumes();
            let static_volumes = {
                let (_, pos, _, volumes, _) = self.rubber.split();
                pos.join(volumes)
            };
            for (&p, &v) in static_volumes {
                self.debris_phys.add_static_volume(p, v);
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

            self.rubber
                .insert((age, position, rotation, volume, mesh_id));
            self.debris.free(debris_id);
        });
    }
}
