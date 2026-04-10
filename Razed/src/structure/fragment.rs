use ethel::state::data::{
    Column, DirectIndex, IndirectIndex,
    hash::{FxLsSpatialHash, FxSpatialHash, SpatialResolution},
    table::TableView,
};
use glam::Vec4Swizzles;
use janus::context::DeltaTime;
use physics::{rigid::RigidBodySolver, xpbd::NodesRowTableView};
use rustc_hash::FxHashSet;

use crate::{structure::deforms::DeformsRowTableView, voxel::VoxelGrid};

const MIN_CLUSTER_SIZE: u32 = 3;

#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FragmentState {
    /// The fragment is attached to the lattice structure.
    ///
    /// The behaviour of the fragment is entirely driven by the lattice nodes
    /// it is attached to.
    #[default]
    Attached = 1,

    /// The fragment is an independent physical body.
    ///
    /// It is not attached to any structure and it is most likely in movement
    /// heading towards the ground.
    Debris = 0,

    /// Static/inactive
    ///
    /// The then fragment, and now debris has been on the ground for a
    /// prolonged period of time.
    ///
    /// It is likely scheduled for removal.
    InactiveDebris = 2,
}

pub const PARENTS_COUNT: usize = 4;
pub const ANCHORS_COUNT: usize = 8;

ethel::table_spec! {
    struct Fragments {
        state: FragmentState;

        parents: [IndirectIndex; PARENTS_COUNT];
        parents_weights: [f32; PARENTS_COUNT];

        anchors: [IndirectIndex; ANCHORS_COUNT];
        anchors_weights: [f32; ANCHORS_COUNT];

        // bind position at fragment creation
        // vec4 due to SSBO alignment requirements
        bind_position: glam::Vec4;

        // lattice contribution coefficient
        health_coeff: f32;
        // debris rigid body mass coefficient
        mass_coeff: f32;
        // normalised integrity of fragment [0; 1]
        integrity: f32;

        world_position: glam::Vec3;

        //todo: mesh id, structure id?
    }
}

ethel::table_spec! {
    struct Debris {
        state: FragmentState;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum UninitFragmentStage {
    #[default]
    Unregistered,
    Unfinished {
        indirect: IndirectIndex,
    },
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UninitFragment {
    pub stage: UninitFragmentStage,
    pub position: glam::Vec3,
    //todo: mesh id, structure id?
}

impl UninitFragment {
    fn new(position: glam::Vec3) -> Self {
        Self {
            position,
            ..Default::default()
        }
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

#[derive(Debug)]
pub struct FragmentSystem {
    fragments: FragmentsRowTable,
    uninitialised: Vec<UninitFragment>,

    debris: DebrisRowTable,
    debris_phys: RigidBodySolver,
    debris_hash: FxLsSpatialHash<DirectIndex>,
    debris_volume_buffer: DebrisVolumeBuffer,

    /// sparse map of deform ID to sequence of fragment IDs
    deform_map: Vec<Vec<IndirectIndex>>,
    /// sparse map of node ID to sequence of fragment IDs
    node_map: Vec<Vec<IndirectIndex>>,

    /// accumulated hash set of disabled fragment IDs; avoids dedup op
    /// these are the fragments' indirect indices (stable)
    disabled_frags_hash: FxHashSet<IndirectIndex>,
    /// per-frame list of damaged fragments from nodes
    /// these are the fragments' direct indices (unstable)
    fragment_damage_frame: Vec<(DirectIndex, IndirectIndex)>,

    disabled_frags_frame: Vec<DirectIndex>,
}

impl Default for FragmentSystem {
    fn default() -> Self {
        Self::new()
    }
}

const DEBRIS_HASH_RESOLUTION: SpatialResolution = SpatialResolution::new(2.0);

impl FragmentSystem {
    pub fn new() -> Self {
        Self {
            fragments: FragmentsRowTable::new(),
            uninitialised: Vec::new(),

            debris: DebrisRowTable::new(),
            debris_phys: RigidBodySolver::default(),
            debris_hash: FxLsSpatialHash::new(DEBRIS_HASH_RESOLUTION),
            debris_volume_buffer: DebrisVolumeBuffer::new(),

            // account for degenerate
            deform_map: vec![Vec::new()],
            // account for degenerate
            node_map: vec![Vec::new()],

            disabled_frags_hash: FxHashSet::default(),
            fragment_damage_frame: Vec::new(),
            disabled_frags_frame: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        // account for degenerate
        let mut deform_map = Vec::with_capacity(capacity + 1);
        deform_map.push(Vec::new());

        // account for degenerate
        let mut node_map = Vec::with_capacity(capacity + 1);
        node_map.push(Vec::new());

        Self {
            fragments: FragmentsRowTable::with_capacity(capacity),
            uninitialised: Vec::with_capacity(capacity),

            debris: DebrisRowTable::with_capacity(capacity),
            debris_phys: RigidBodySolver::default(),
            debris_hash: FxLsSpatialHash::with_capacity(DEBRIS_HASH_RESOLUTION, capacity),
            debris_volume_buffer: DebrisVolumeBuffer::new(),

            deform_map,
            node_map,

            disabled_frags_hash: FxHashSet::default(),
            fragment_damage_frame: Vec::new(),
            disabled_frags_frame: Vec::new(),
        }
    }

    /// Get a slice to the fragments IDs associated to `node` ID.
    ///
    /// # Panics
    /// Will panic if `node` is out-of-bounds; i.e. the node has not been
    /// registered with [`FragmentSystem::bind_lattice`].
    ///
    /// This will not panic if the `node` has no associated fragments: an empty
    /// slice will be returned instead.
    pub fn fragments_of_node(&self, node: IndirectIndex) -> &[IndirectIndex] {
        &self.node_map[node.as_index()]
    }

    /// Get a mutable slice to the fragments IDs associated to `node` ID.
    ///
    /// See [`FragmentSystem::fragments_of_node`] for details on panics.
    pub fn fragments_of_node_mut(&mut self, node: IndirectIndex) -> &mut [IndirectIndex] {
        &mut self.node_map[node.as_index()]
    }

    /// Get a slice to the fragments IDs associated to `deform` ID.
    ///
    /// # Panics
    /// Will panic if `deform` is out-of-bounds; i.e. the deform has not been
    /// registered with [`FragmentSystem::bind_deforms`].
    ///
    /// This will not panic if the `deform` has no associated fragments: an
    /// empty slice will be returned instead.
    pub fn fragments_of_deform(&self, deform: IndirectIndex) -> &[IndirectIndex] {
        &self.deform_map[deform.as_index()]
    }

    /// Get a mutable slice to the fragments IDs associated to `deform` ID.
    ///
    /// See [`FragmentSystem::fragments_of_deform`] for details on panics.
    pub fn fragments_of_deform_mut(&mut self, deform: IndirectIndex) -> &mut [IndirectIndex] {
        &mut self.deform_map[deform.as_index()]
    }

    pub fn debris(&self) -> &DebrisRowTable {
        &self.debris
    }

    pub fn debris_mut(&mut self) -> &mut DebrisRowTable {
        &mut self.debris
    }

    pub fn hash_debris(&mut self) {
        self.debris_hash.clear();

        let positions = &self.debris.position;
        for i in 1..positions.len() {
            let pos = positions[i];
            let cell = self.debris_hash.cell_at(pos);
            self.debris_hash.put(cell, DirectIndex::from_index(i));
        }
    }

    pub fn simulate_debris(&mut self, delta: DeltaTime) {
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

        self.debris_hash
            .elements()
            .filter(|vec| !vec.is_empty())
            .for_each(|debris| {
                self.debris_volume_buffer.clear();

                debris.iter().for_each(|index| {
                    let position = positions[index.as_index()];
                    let volume = volumes[index.as_index()];
                    let handle = handles[index.as_index()];
                    self.debris_volume_buffer.push(position, volume, handle);
                });

                let DebrisVolumeBuffer {
                    positions,
                    volumes,
                    handles,
                } = &self.debris_volume_buffer;

                self.debris_phys
                    .detect_collisions(positions, volumes, handles);
            });
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

    pub fn fragments(&self) -> &FragmentsRowTable {
        &self.fragments
    }

    pub fn fragments_mut(&mut self) -> &mut FragmentsRowTable {
        &mut self.fragments
    }

    pub fn reset(&mut self) {
        self.deform_map.clear();
        self.node_map.clear();
    }

    pub fn clear_damage_buffer(&mut self) {
        self.disabled_frags_hash.clear();
        self.disabled_frags_frame.clear();
        self.fragment_damage_frame.clear();
    }

    pub fn compute_world_positions(&mut self, deforms: &DeformsRowTableView) {
        let length = self.fragments.len();
        let state = &self.fragments.state;
        let anchors = &self.fragments.anchors;
        let anchor_weights = &self.fragments.anchors_weights;
        let bind = &self.fragments.bind_position;
        let world_pos = &mut self.fragments.world_position;

        let deform_pose = &deforms.pose;
        let deform_now = &deforms.deformed;

        for i in 1..length {
            let state = state[i];
            if !matches!(state, FragmentState::Attached) {
                continue;
            }

            let mut pos = bind[i].xyz();
            let anchors = anchors[i];
            let weights = anchor_weights[i];

            anchors.iter().zip(weights).for_each(|(i, w)| {
                let direct = deforms.solve(*i);
                let d_pose = deform_pose[direct.as_index()];
                let d_now = deform_now[direct.as_index()];
                let v = d_now - d_pose;
                pos += v * w;
            });

            world_pos[i] = pos;
        }
    }

    pub fn sync_deform_damage(
        &mut self,
        dead_points: &[IndirectIndex],
        deforms: &DeformsRowTableView,
    ) {
        for deform in dead_points {
            for &frag_id in &self.deform_map[deform.as_index()] {
                if frag_id.as_int() == 0 {
                    continue;
                }

                if let Some(direct) = self.fragments.solve_indirect(frag_id) {
                    let state = self.fragments.state[direct.as_index()];
                    if !matches!(state, FragmentState::Attached) {
                        continue;
                    }

                    let fragment_world = self.fragments.world_position[direct.as_index()];
                    let anchors = &mut self.fragments.anchors[direct.as_index()];
                    let weights = &mut self.fragments.anchors_weights[direct.as_index()];

                    anchors
                        .iter_mut()
                        .zip(weights.iter_mut())
                        .for_each(|(anchor, weight)| {
                            if *anchor == *deform {
                                *anchor = IndirectIndex::default();
                                *weight = 0f32;
                            } else {
                                let direct = deforms.solve(*anchor);
                                let deform = deforms.deformed[direct.as_index()];
                                let ds = fragment_world.distance_squared(deform);
                                *weight = 1.0 / (ds + f32::EPSILON);
                            }
                        });

                    let w_t = weights.iter().sum::<f32>();
                    weights.iter_mut().for_each(|w| *w /= w_t);

                    if self.disabled_frags_hash.insert(frag_id) {
                        self.disabled_frags_frame.push(direct);
                    }
                }
            }
        }
    }

    /// Synchronise stable indirect indices `broken_ids` of constraints and
    /// [`degenerate_nodes`] with fragments state.
    pub fn sync_lattice_damage(&mut self, broken_nodes: &[IndirectIndex]) {
        for &node in broken_nodes {
            for &frag_id in &self.node_map[node.as_index()] {
                if frag_id.as_int() == 0 {
                    continue;
                }

                let direct = unsafe { self.fragments.solve_indirect_unchecked(frag_id) };
                let state = self.fragments.state[direct.as_index()];
                if !matches!(state, FragmentState::Attached) {
                    continue;
                }

                self.fragment_damage_frame.push((direct, node));
            }
        }

        // validate disabled fragments and invalidate relations
        let parents = &mut self.fragments.parents;
        let weights = &mut self.fragments.parents_weights;
        let states = &mut self.fragments.state;

        self.fragment_damage_frame
            .drain(..)
            .for_each(|(frag_idx, node_id)| {
                let parents = unsafe { parents.get_unchecked_mut(frag_idx.as_index()) };
                let weights = unsafe { weights.get_unchecked_mut(frag_idx.as_index()) };
                let mut empty_weight = 0.0;

                parents
                    .iter_mut()
                    .zip(weights.iter_mut())
                    .for_each(|(id, weight)| {
                        if *id == node_id {
                            *id = IndirectIndex::default();
                            empty_weight = *weight;
                            *weight = 0.0;
                        }
                    });

                // redistribute lost weight
                weights.iter_mut().for_each(|w| *w += empty_weight * *w);

                let active_count = parents.iter().filter(|id| id.as_int() != 0).count();
                if active_count < MIN_CLUSTER_SIZE as usize {
                    let state = unsafe { states.get_unchecked_mut(frag_idx.as_index()) };
                    *state = FragmentState::Debris;

                    let indirect = self.fragments.handles[frag_idx.as_index()];
                    if self.disabled_frags_hash.insert(indirect) {
                        self.disabled_frags_frame.push(frag_idx);
                    }
                }
            });
    }

    /// Return a slice containing the *direct indices* of all fragments
    /// damaged in the last frame.
    ///
    /// Each entry is a tuple that contains the fragment index first, then the
    /// node ID it was broken off of.
    ///
    /// Note: this returns **direct indices**; these are the direct element
    /// indices inside of the fragments table. These are not stable handles.
    ///
    /// These are unstable and may be invalidated on the next frame; they are
    /// intended for use only during the same frame this was populated in and
    /// before any operation that might add/remove elements to the table.
    pub fn frame_damaged_fragments(&self) -> &[(DirectIndex, IndirectIndex)] {
        &self.fragment_damage_frame
    }

    pub fn frame_disabled_frags(&self) -> &[DirectIndex] {
        &self.disabled_frags_frame
    }

    pub fn frame_disabled_frags_hash(&self) -> &FxHashSet<IndirectIndex> {
        &self.disabled_frags_hash
    }

    const VOXEL_NEIGHBOR_QUERY_RADIUS: u32 = 16;

    pub fn bind_deforms(
        &mut self,
        deforms_hash: &FxSpatialHash<IndirectIndex>,
        deforms: &DeformsRowTableView,
    ) {
        {
            let deforms_len = deforms.view_offset() + deforms.len();
            self.deform_map.resize_with(deforms_len, || Vec::new());
        }

        let mut near_buf = Vec::with_capacity(ANCHORS_COUNT);

        self.uninitialised.retain(|frag| {
            if let UninitFragmentStage::Unfinished { indirect } = frag.stage {
                let fragment_world = frag.position;
                let fragment_cell = deforms_hash.cell_at(fragment_world);

                if let Err(rem) = deforms_hash.nearest_cells(
                    fragment_cell,
                    ANCHORS_COUNT as u32,
                    Self::VOXEL_NEIGHBOR_QUERY_RADIUS,
                    &mut near_buf,
                    false,
                ) {
                    tracing::event!(
                        name: "fragment.bind_deforms.near_query.miss",
                        tracing::Level::ERROR,
                        "Query for nearby deforms to {fragment_cell:?}: miss {rem} deforms within range.",
                    )
                }

                let near_count = near_buf.len().min(ANCHORS_COUNT);
                let fragment_direct = self.fragments.solve_indirect(indirect).expect("fragment indirect always valid");
                let fragment_anchors = &mut self.fragments.anchors[fragment_direct.as_index()];
                let fragment_weights = &mut self.fragments.anchors_weights[fragment_direct.as_index()];

                near_buf.drain(..)
                    .take(near_count)
                    .zip(fragment_anchors.iter_mut())
                    .zip(fragment_weights.iter_mut())
                    .for_each(|((cell, anchor_id), anchor_weight)| {
                        let deform = deforms_hash.get(cell).copied().expect("deforms hash neighbors are populated");
                        let point = deforms.pose(deform);
                        let ds = fragment_world.distance_squared(*point);

                        *anchor_id = deform;
                        *anchor_weight = 1.0 / (ds + f32::EPSILON);
                    });

                let w_t = fragment_weights.iter().sum::<f32>();
                fragment_weights.iter_mut().for_each(|w| *w /= w_t);

                for anchor in fragment_anchors {
                    self.deform_map[anchor.as_index()].push(indirect);
                }

                // remove fragment from intermediate buffer
                false
            } else {
                // keep other unfinished fragments
                true
            }
        });
    }

    /// Binds all currently uninitialised fragments to `lattice` through
    /// `lattice_hash` spatial info.
    ///
    /// The given `lattice` and `lattice_hash` must be in world-space.
    pub fn bind_lattice(
        &mut self,
        lattice_hash: &FxSpatialHash<IndirectIndex>,
        lattice: &NodesRowTableView,
    ) {
        {
            let lattice_len = lattice.view_offset() + lattice.len();
            self.node_map.resize_with(lattice_len, || Vec::new());
        }

        let mut near_buf = Vec::with_capacity(PARENTS_COUNT);

        self.uninitialised.iter_mut().for_each(|frag| {
            let fragment_world = frag.position;
            let fragment_cell = lattice_hash.cell_at(fragment_world);

            if let Err(rem) = lattice_hash.nearest_cells(
                fragment_cell,
                PARENTS_COUNT as u32,
                Self::VOXEL_NEIGHBOR_QUERY_RADIUS,
                &mut near_buf,
                false,
            ) {
                tracing::event!(
                    name: "fragment.bind_lattice.near_query.miss",
                    tracing::Level::ERROR,
                    "Query for nearby nodes to {fragment_cell:?}: miss {rem} nodes within range.",
                )
            }

            let near_count = near_buf.len().min(PARENTS_COUNT);
            // if n_count < MIN_CLUSTER_SIZE as usize {
            //     tracing::event!(
            //         name: "structure.fragment.build.query.skip_voxel",
            //         tracing::Level::WARN,
            //         "Skipping voxel {cell:?}: not enough {n_count} nearby nodes found."
            //     );
            //     continue;
            // }

            let (parents, weights) = {
                let mut parents = [IndirectIndex::default(); PARENTS_COUNT];
                let mut weights = [0f32; PARENTS_COUNT];

                near_buf
                    .drain(..)
                    .take(near_count)
                    .zip(&mut parents.iter_mut().zip(&mut weights))
                    .for_each(|(cell, (id, weight))| {
                        *id = lattice_hash
                            .get(cell)
                            .copied()
                            .expect("lattice hash neighbors are populated");
                        let point = lattice.current_pos(*id);
                        let ds = fragment_world.distance_squared(*point);
                        *weight = 1.0 / (ds + f32::EPSILON);
                    });

                let w_t = weights.iter().fold(0f32, |t, &v| t + v);
                weights.iter_mut().for_each(|v| *v /= w_t);

                (parents, weights)
            };

            let position = glam::vec4(fragment_world.x, fragment_world.y, fragment_world.z, 1.0);
            let handle = self.fragments.insert((
                FragmentState::Attached,
                parents,
                weights,
                [IndirectIndex::default(); ANCHORS_COUNT],
                [0f32; ANCHORS_COUNT],
                position,
                1.0, // todo: health contribution
                0.5, // todo: debris rigid body
                1.0, // todo: damage and integrity
                fragment_world,
            ));
            frag.stage = UninitFragmentStage::Unfinished { indirect: handle };

            for node in parents {
                self.node_map[node.as_index()].push(handle);
            }
        });
    }

    /// Create new uninitialised fragments from a `voxels` describing
    /// their positions in space.
    ///
    /// The `voxels` [`VoxelGrid`] is expected to have been built previously
    /// with [`VoxelGrid::build`].
    pub fn generate_fragments(&mut self, origin: glam::Vec3, grid: &VoxelGrid) {
        let mut world_points = vec![glam::Vec3::ZERO; grid.count()];

        grid.to_world(origin, &mut world_points);
        for voxel in world_points {
            self.uninitialised.push(UninitFragment::new(voxel));
        }
    }
}
