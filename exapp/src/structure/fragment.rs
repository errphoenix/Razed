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
    }
}

#[derive(Debug)]
pub struct FragmentSystem {
    fragments: FragmentsRowTable,

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
            // account for degenerate
            node_map: vec![Vec::new()],

            disabled_frags_alltime: FxHashSet::default(),
            disabled_frags_frame: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        // account for degenerate
        let mut node_map = Vec::with_capacity(capacity + 1);
        node_map.push(Vec::new());

        Self {
            fragments: FragmentsRowTable::with_capacity(capacity),
            node_map,

            disabled_frags_alltime: FxHashSet::default(),
            disabled_frags_frame: Vec::new(),
        }
    }

    /// Get a slice to the fragments IDs associated to `node` ID.
    ///
    /// # Panics
    /// Will panic if `node` is out-of-bounds; i.e. the node has not been
    /// registered with [`FragmentSystem::generate_fragments`].
    ///
    /// This will not panic if the `node` has no associated fragments: an empty
    /// slice will be returned instead.
    pub fn fragments_of(&self, node: IndirectIndex) -> &[IndirectIndex] {
        &self.node_map[node.as_index()]
    }

    /// Get a mutable slice to the fragments IDs associated to `node` ID.
    ///
    /// See [`FragmentSystem::fragments_of`] for details on panics.
    pub fn fragments_of_mut(&mut self, node: IndirectIndex) -> &mut [IndirectIndex] {
        &mut self.node_map[node.as_index()]
    }

    pub fn table(&self) -> &FragmentsRowTable {
        &self.fragments
    }

    pub fn table_mut(&mut self) -> &mut FragmentsRowTable {
        &mut self.fragments
    }

    pub fn reset(&mut self) {
        self.node_map.clear();
    }

    /// Synchronise (stable II) `broken_ids` of constraints and
    /// [`degenerate_nodes`] with fragments state.
    pub fn sync_lattice_damage(&mut self, broken_nodes: &[IndirectIndex]) {
        self.disabled_frags_frame.clear();
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

            parents
                .iter_mut()
                .zip(weights.iter_mut())
                .for_each(|(id, weight)| {
                    if *id == node_id {
                        *id = IndirectIndex::default();
                        *weight = 0.0;
                    }
                });

            // recalibrate weights
            let w_t = weights.iter().fold(0f32, |v0, vi| v0 + *vi);
            for w in weights {
                *w /= w_t;
            }

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

    /// Generate new fragments from a [`VoxelGrid`] and raw lattice data.
    ///
    /// The `voxels` [`VoxelGrid`] is expected to have been built previously
    /// with [`VoxelGrid::build`].
    pub fn generate_fragments(
        &mut self,
        origin: glam::Vec3,
        grid: &VoxelGrid,
        lattice: &NodesRowTableView,
    ) {
        let node_hash = {
            let mut node_hash = FxSpatialHash::with_capacity(
                SpatialResolution::new(Self::LATTICE_SPATIAL_RESOLUTION),
                lattice.len(),
            );
            node_hash.dump_soa(lattice.current_pos, lattice.handles);
            node_hash
        };

        for _ in 0..lattice.len() {
            self.node_map.push(Vec::<IndirectIndex>::new());
        }

        let mut near_buf = Vec::with_capacity(PARENTS_COUNT);
        let mut world_points = vec![glam::Vec3::ZERO; grid.count()];

        grid.to_world(origin, &mut world_points);
        let mut i = 0;
        for voxel in world_points {
            let cell = node_hash.cell_at(voxel);

            #[cfg(not(debug_assertions))]
            let _ = node_hash.nearest_cells(
                cell,
                PARENTS_COUNT as u32,
                Self::VOXEL_NEIGHBOR_QUERY_RADIUS,
                &mut near_buf,
                false,
            );

            #[cfg(debug_assertions)]
            {
                if let Err(rem) = node_hash.nearest_cells(
                    cell,
                    PARENTS_COUNT as u32,
                    Self::VOXEL_NEIGHBOR_QUERY_RADIUS,
                    &mut near_buf,
                    false,
                ) {
                    tracing::event!(
                        name: "structure.fragment.build.query.err_maybe_miss",
                        tracing::Level::ERROR,
                        "Query for nearby nodes to {cell:?} could not produce {rem} amount of nodes within range: maybe a miss? or lattice is malformed.",
                    )
                }
            }

            {
                let cell_world = node_hash.approx_point_at(cell);

                near_buf.sort_by(|c0, c1| {
                    let id0 = node_hash.get(c0).expect("query populated neighbour cell");
                    let id1 = node_hash.get(c1).expect("query populated neighbour cell");

                    let p0 = lattice.current_pos(*id0);
                    let p1 = lattice.current_pos(*id1);

                    let l0 = p0 - cell_world;
                    let l1 = p1 - cell_world;

                    l0.length_squared()
                        .partial_cmp(&l1.length_squared())
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }

            let n_count = near_buf.len().min(PARENTS_COUNT);

            if n_count < MIN_CLUSTER_SIZE as usize {
                tracing::event!(
                    name: "structure.fragment.build.query.skip_voxel",
                    tracing::Level::WARN,
                    "Skipping voxel {cell:?}: not enough {n_count} nearby nodes found."
                );
                continue;
            }

            let (parents, weights) = {
                let (mut parents, mut weights) =
                    ([Default::default(); PARENTS_COUNT], [0f32; PARENTS_COUNT]);

                near_buf
                    .drain(..)
                    .take(n_count)
                    .zip(&mut parents.iter_mut().zip(&mut weights))
                    .for_each(|(cell, (id, weight))| {
                        *id = node_hash.get(&cell).copied().unwrap_or_default();
                        let point = lattice.current_pos(*id);
                        let ds = voxel.distance_squared(*point);
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
                glam::vec4(voxel.x, voxel.y, voxel.z, 1f32),
                1.0, // todo: health contribution
                1.0, // todo: debris rigid body
                1.0, // todo: damage and integrity
                voxel,
                glam::Vec3::ZERO,
                glam::Vec3::ZERO,
            ));
            i += 1;

            for node in parents {
                self.node_map[node.as_index()].push(handle);
            }
        }

        println!("done; {i} fragments");
    }
}
