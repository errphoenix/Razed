pub mod mesh;

use ethel::state::data::{
    Column, DirectIndex, IndirectIndex,
    hash::{Cell, FxSpatialHash},
    table::TableView,
};
use rustc_hash::FxHashSet;

use crate::{
    procedural::VoxelGrid,
    structure::{
        CageSystem,
        lattice::{DamagedNode, NodesRowTableView},
    },
};

const MIN_CLUSTER_SIZE: u32 = 3;

pub const PARENTS_COUNT: usize = 4;
pub const ANCHORS_COUNT: usize = 8;

ethel::table_spec! {
    struct Fragments {
        parents: [IndirectIndex; PARENTS_COUNT];
        parents_weights: [f32; PARENTS_COUNT];

        deformation_cage: IndirectIndex;

        // bind position at fragment creation
        // vec4 due to SSBO alignment requirements
        bind_position: glam::Vec4;

        // lattice contribution coefficient
        health_coeff: f32;
        // debris rigid body mass coefficient
        mass_coeff: f32;
        // normalised integrity of fragment [0; 1]
        integrity: f32;

        mesh_id: ethel::mesh::Id;
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
    pub mesh_id: ethel::mesh::Id,
    //todo:  structure id?
}

impl UninitFragment {
    fn new(position: glam::Vec3, mesh_id: ethel::mesh::Id) -> Self {
        Self {
            position,
            mesh_id,
            ..Default::default()
        }
    }
}

#[derive(Debug)]
pub struct FragmentSystem {
    fragments: FragmentsRowTable,
    uninitialised: Vec<UninitFragment>,

    /// sparse map of node ID to sequence of fragment IDs
    node_map: Vec<Vec<IndirectIndex>>,

    /// per-frame list of damaged fragments from nodes
    /// these are the fragments' direct indices (unstable)
    fragment_damage_frame: Vec<(DirectIndex, DamagedNode)>,

    disabled_frags_frame: FxHashSet<DirectIndex>,
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
            fragment_damage_frame: Vec::new(),
            disabled_frags_frame: FxHashSet::default(),
            // account for degenerate
            node_map: vec![Vec::new()],
        }
    }

    #[allow(unused)]
    pub fn with_capacity(capacity: usize) -> Self {
        // account for degenerate
        let mut node_map = Vec::with_capacity(capacity + 1);
        node_map.push(Vec::new());

        Self {
            fragments: FragmentsRowTable::with_capacity(capacity),
            uninitialised: Vec::with_capacity(capacity),
            fragment_damage_frame: Vec::new(),
            disabled_frags_frame: FxHashSet::default(),
            node_map,
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
    #[allow(unused)]
    pub fn fragments_of_node(&self, node: IndirectIndex) -> &[IndirectIndex] {
        &self.node_map[node.as_index()]
    }

    /// Get a mutable slice to the fragments IDs associated to `node` ID.
    ///
    /// See [`FragmentSystem::fragments_of_node`] for details on panics.
    #[allow(unused)]
    pub fn fragments_of_node_mut(&mut self, node: IndirectIndex) -> &mut [IndirectIndex] {
        &mut self.node_map[node.as_index()]
    }

    pub fn data(&self) -> &FragmentsRowTable {
        &self.fragments
    }

    pub fn data_mut(&mut self) -> &mut FragmentsRowTable {
        &mut self.fragments
    }

    #[allow(unused)]
    pub fn reset(&mut self) {
        self.node_map.clear();
    }

    pub fn clear_damage_buffer(&mut self) {
        self.disabled_frags_frame.clear();
        self.fragment_damage_frame.clear();
    }

    /// Synchronise stable indirect indices `broken_ids` of constraints and
    /// [`degenerate_nodes`] with fragments state.
    pub fn sync_lattice_damage(&mut self, broken_nodes: &[DamagedNode]) {
        for &node in broken_nodes {
            for &frag_id in &self.node_map[node.id.as_index()] {
                if frag_id.as_int() == 0 {
                    continue;
                }

                if let Some(direct) = self.fragments.solve_indirect(frag_id) {
                    self.fragment_damage_frame.push((direct, node));
                }
            }
        }

        // validate disabled fragments and invalidate relations
        let parents = &mut self.fragments.parents;
        let weights = &mut self.fragments.parents_weights;

        self.fragment_damage_frame.drain(..).for_each(
            |(
                frag_idx,
                DamagedNode {
                    id: node_id,
                    constraints_left,
                },
            )| {
                if constraints_left == 0 {
                    self.disabled_frags_frame.insert(frag_idx);
                    return;
                }

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
                    self.disabled_frags_frame.insert(frag_idx);
                }
            },
        );
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
    #[allow(unused)]
    pub fn frame_damaged_fragments(&self) -> &[(DirectIndex, DamagedNode)] {
        &self.fragment_damage_frame
    }

    pub fn frame_disabled_frags_count(&self) -> usize {
        self.disabled_frags_frame.len()
    }

    pub fn frame_disabled_frags(&self) -> impl Iterator<Item = &DirectIndex> {
        self.disabled_frags_frame.iter()
    }

    const VOXEL_NEIGHBOR_QUERY_RADIUS: u32 = 12;

    pub fn create_deformation_cages(
        &mut self,
        lattice_hash: &FxSpatialHash<IndirectIndex>,
        lattice: &NodesRowTableView,
        cage: &mut CageSystem,
    ) {
        self.uninitialised.retain(|frag| {
            if let UninitFragmentStage::Unfinished { indirect } = frag.stage {
                let fragment_world = frag.position;
                let cage_id = cage.generate_cage(fragment_world, lattice_hash, lattice);
                let direct = self.fragments.solve_indirect(indirect).unwrap();
                self.fragments.deformation_cage[direct.as_index()] = cage_id;
                false
            } else {
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
        self.node_map.resize_with(lattice.size(), || Vec::new());
        let mut near_buf = Vec::with_capacity(PARENTS_COUNT);

        self.uninitialised.iter_mut().for_each(|frag| {
            let fragment_world = frag.position;
            let fragment_mesh = frag.mesh_id;
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
                parents,
                weights,
                IndirectIndex::default(),
                position,
                50.0, // todo: health contribution
                1.0,  // todo: debris rigid body
                1.0,  // todo: damage and integrity
                fragment_mesh,
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
    /// The given voxel `grid` should be in
    /// [`absolute-space`](VoxelGrid::to_abs_space).
    ///
    /// The `voxels` [`VoxelGrid`] is expected to have been built previously
    /// with [`VoxelGrid::build`].
    pub fn generate(
        &mut self,
        origin: glam::Vec3,
        grid: &VoxelGrid,
        mesh_mapping: &FxSpatialHash<ethel::mesh::Id>,
    ) {
        let offset = {
            let opt = grid.options();
            let size = glam::vec3(opt.width, opt.height, opt.depth);
            (opt.cell_size - size) * 0.5
        };
        for &cell in grid.voxels().cells() {
            let point = grid.point_at_or_approx(cell) + origin + offset;

            // temporary: always force the only existing 3x3x3 mesh group
            let cell = Cell {
                x: cell.x % 3,
                y: cell.y % 3,
                z: cell.z % 3,
            };

            if let Some(&mesh_id) = mesh_mapping.get(cell) {
                self.uninitialised.push(UninitFragment::new(point, mesh_id));
            }
        }
    }
}
