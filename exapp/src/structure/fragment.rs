use ethel::state::data::{
    Column, DirectIndex, IndirectIndex,
    hash::{FxSpatialHash, SpatialResolution},
    table::TableView,
};
use physics::xpbd::NodesRowTableView;
use rustc_hash::FxHashSet;

use crate::{structure::deforms::DeformsRowTableView, voxel::VoxelGrid};

const MIN_CLUSTER_SIZE: u32 = 7;

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

        // rigid body position, unused during attached state
        // vec3 to reduce memory footprint in physics solver
        position: glam::Vec3;
        velocity: glam::Vec3;
        forces: glam::Vec3;

        //todo: mesh id, structure id?
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

#[derive(Debug)]
pub struct FragmentSystem {
    fragments: FragmentsRowTable,
    uninitialised: Vec<UninitFragment>,

    /// sparse map of deform ID to sequence of fragment IDs
    deform_map: Vec<Vec<IndirectIndex>>,
    /// sparse map of node ID to sequence of fragment IDs
    node_map: Vec<Vec<IndirectIndex>>,

    /// alltime accumulated set of disabled fragment IDs; avoids dedup op
    /// these are the fragments' indirect indices (stable)
    disabled_frags_alltime: FxHashSet<IndirectIndex>,

    /// per-frame list of disabled fragment IDs and an indirect node
    /// these are the fragments' direct indices (unstable)
    disabled_frags_frame: Vec<(DirectIndex, IndirectIndex)>,
}

impl Default for FragmentSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl FragmentSystem {
    pub fn new() -> Self {
        Self {
            fragments: FragmentsRowTable::new(),
            uninitialised: Vec::new(),

            // account for degenerate
            deform_map: vec![Vec::new()],
            // account for degenerate
            node_map: vec![Vec::new()],

            disabled_frags_alltime: FxHashSet::default(),
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

            deform_map,
            node_map,

            disabled_frags_alltime: FxHashSet::default(),
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

    pub fn table(&self) -> &FragmentsRowTable {
        &self.fragments
    }

    pub fn table_mut(&mut self) -> &mut FragmentsRowTable {
        &mut self.fragments
    }

    pub fn reset(&mut self) {
        self.deform_map.clear();
        self.node_map.clear();
        self.disabled_frags_alltime.clear();
    }

    pub fn clear_damage_buffer(&mut self) {
        self.disabled_frags_frame.clear();
    }

    pub fn sync_deform_damage(&mut self, dead_points: &[IndirectIndex]) {
        for deform in dead_points {
            for &frag_id in &self.deform_map[deform.as_index()] {
                if frag_id.as_int() == 0 {
                    continue;
                }

                if let Some(direct) = self.fragments.solve_indirect(frag_id) {
                    let anchors = &mut self.fragments.anchors[direct.as_index()];
                    let weights = &mut self.fragments.anchors_weights[direct.as_index()];
                    let mut empty_weight = 0.0;

                    anchors
                        .iter_mut()
                        .zip(weights.iter_mut())
                        .for_each(|(anchor, weight)| {
                            if *anchor == *deform {
                                *anchor = IndirectIndex::default();
                                empty_weight = *weight;
                                *weight = 0f32;
                            }
                        });

                    // redistribute lost weight
                    weights.iter_mut().for_each(|w| *w += empty_weight * *w);
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

                if self.disabled_frags_alltime.insert(frag_id) {
                    let idx = unsafe { self.fragments.solve_indirect_unchecked(frag_id) };
                    self.disabled_frags_frame.push((idx, node));
                }
            }
        }

        // validate disabled fragments and invalidate relations
        let (states, parents, weights, _, _, _, _, _, _, _, _, _) = self.fragments.split_mut();
        self.disabled_frags_frame.retain(|&(frag_idx, node_id)| {
            let parents = unsafe { parents.alpha.get_unchecked_mut(frag_idx.as_index()) };
            let weights = unsafe { weights.alpha.get_unchecked_mut(frag_idx.as_index()) };
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
                let state = unsafe { states.alpha.get_unchecked_mut(frag_idx.as_index()) };
                *state = FragmentState::Debris;
                false
            } else {
                true
            }
        });
    }

    /// Return a slice containing the *direct indices* of all fragments
    /// disabled in the last frame.
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
    pub fn frame_disabled_frags_direct(&self) -> &[(DirectIndex, IndirectIndex)] {
        &self.disabled_frags_frame
    }

    const LATTICE_SPATIAL_RESOLUTION: u32 = 2;
    const VOXEL_NEIGHBOR_QUERY_RADIUS: u32 = 10;

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
                        let deform = deforms_hash.get(&cell).copied().expect("deforms hash neighbors are populated");
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
                            .get(&cell)
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

            let handle = self.fragments.insert((
                FragmentState::Attached,
                parents,
                weights,
                [IndirectIndex::default(); ANCHORS_COUNT],
                [0f32; ANCHORS_COUNT],
                glam::vec4(fragment_world.x, fragment_world.y, fragment_world.z, 1.0),
                1.0, // todo: health contribution
                1.0, // todo: debris rigid body
                1.0, // todo: damage and integrity
                fragment_world,
                glam::Vec3::ZERO,
                glam::Vec3::ZERO,
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
