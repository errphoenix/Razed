use ethel::state::data::{
    Column, DirectIndex, IndirectIndex,
    hash::{FxLsSpatialHash, SpatialResolution},
};
use janus::context::DeltaTime;
use physics::rigid::RigidBodySolver;

ethel::table_spec! {
    struct Debris {
        // milliseconds since spawned
        age: u32;

        position: glam::Vec3;
        rotation: glam::Quat;

        velocity: glam::Vec3;
        angular_velocity: glam::Vec3;

        forces: glam::Vec3;
        torques: glam::Vec3;

        mass: f32;
        inv_inertia_loc: glam::Mat3;
        inv_inertia_abs: glam::Mat3;

        volume: physics::Sphere;
    }
}

ethel::table_spec! {
    struct Rubber {
        age: u32;

        position: glam::Vec3;
        rotation: glam::Quat;

        volume: physics::Sphere;
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DebrisVolumeBuffer {
    pub positions: Vec<glam::Vec3>,
    pub volumes: Vec<::physics::Sphere>,
    pub handles: Vec<IndirectIndex>,
}

impl DebrisVolumeBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            positions: Vec::with_capacity(capacity),
            volumes: Vec::with_capacity(capacity),
            handles: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, position: glam::Vec3, volume: ::physics::Sphere, handle: IndirectIndex) {
        self.positions.push(position);
        self.volumes.push(volume);
        self.handles.push(handle);
    }

    pub fn clear(&mut self) {
        self.positions.clear();
        self.volumes.clear();
        self.handles.clear();
    }
}

const HASH_RESOLUTION: SpatialResolution = SpatialResolution::new(2.0);

#[derive(Debug)]
pub struct DebrisSystem {
    debris: DebrisRowTable,
    debris_phys: RigidBodySolver,
    debris_hash: FxLsSpatialHash<DirectIndex>,
    debris_volume_buffer: DebrisVolumeBuffer,

    rubber: RubberRowTable,
}

impl Default for DebrisSystem {
    fn default() -> Self {
        Self {
            debris: DebrisRowTable::default(),
            debris_phys: RigidBodySolver::default(),
            debris_hash: FxLsSpatialHash::new(HASH_RESOLUTION),
            debris_volume_buffer: DebrisVolumeBuffer::default(),
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
            rubber: RubberRowTable::with_capacity(capacity / 2),
        }
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

    /// this offset is used to determine whether the indices used in
    /// `debris_hash` are debris or rubber data.
    fn internal_debris_rubber_offset(&self) -> usize {
        self.debris.len().max(self.rubber.len())
    }

    pub fn hash_all(&mut self) {
        self.debris_hash.clear();

        let positions = &self.debris.position;
        for i in 1..positions.len() {
            let pos = positions[i];
            let cell = self.debris_hash.cell_at(pos);
            self.debris_hash.put(cell, DirectIndex::from_index(i));
        }

        let positions = &self.rubber.position;
        let offset = self.internal_debris_rubber_offset();

        for i in 1..positions.len() {
            let pos = positions[i];
            let i = i + offset;
            let cell = self.debris_hash.cell_at(pos);
            self.debris_hash.put(cell, DirectIndex::from_index(i));
        }
    }

    pub fn simulate_bodies(&mut self, delta: DeltaTime) {
        let slice_offset = self.internal_debris_rubber_offset();
        let positions = &mut self.debris.position;
        let rotations = &mut self.debris.rotation;
        let velocities = &mut self.debris.velocity;
        let ang_velocities = &mut self.debris.angular_velocity;
        let forces = &mut self.debris.forces;
        let torques = &mut self.debris.torques;
        let masses = &self.debris.mass;
        let inv_inertia_loc = &self.debris.inv_inertia_loc;
        let inv_inertia_abs = &mut self.debris.inv_inertia_abs;
        let volumes = &self.debris.volume;
        let handles = &self.debris.handles;

        {
            let r_positions = &self.rubber.position;
            let r_volumes = &self.rubber.volume;

            self.debris_hash
                .elements()
                .filter(|vec| !vec.is_empty())
                .for_each(|debris| {
                    self.debris_volume_buffer.clear();

                    debris.iter().for_each(|index| {
                        let index = index.as_index();

                        if index > slice_offset {
                            let index = index - slice_offset;
                            let position = r_positions[index];
                            let volume = r_volumes[index];
                            let handle = IndirectIndex::default();
                            self.debris_volume_buffer.push(position, volume, handle);
                        } else {
                            let position = positions[index];
                            let volume = volumes[index];
                            let handle = handles[index];
                            self.debris_volume_buffer.push(position, volume, handle);
                        }
                    });

                    let DebrisVolumeBuffer {
                        positions,
                        volumes,
                        handles,
                    } = &self.debris_volume_buffer;

                    self.debris_phys
                        .detect_collisions(positions, volumes, handles);
                });
        }
        self.debris_phys
            .solve_collisions(positions, velocities, ang_velocities, handles);

        self.debris_phys.apply_gravity(forces);
        self.debris_phys
            .sync_inertia(rotations, inv_inertia_loc, inv_inertia_abs);

        self.debris_phys.integrate(
            velocities,
            ang_velocities,
            forces,
            torques,
            masses,
            inv_inertia_abs,
            delta,
        );
        self.debris_phys
            .update_bodies(positions, rotations, velocities, ang_velocities, delta);

        self.debris_phys
            .damp_velocity(velocities, ang_velocities, delta);
        self.debris_phys
            .constrain_ground(positions, velocities, ang_velocities);
    }
}
