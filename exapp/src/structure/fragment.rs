use ethel::state::data::{
    Column,
    hash::{Cell, FxSpatialHash, SpatialResolution},
};
use physics::xpbd::{LinkNodes, LinksRowTable};
use rustc_hash::FxHashSet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FragmentState {
    /// The fragment is attached to the lattice structure.
    ///
    /// The behaviour of the fragment is entirely driven by the lattice nodes
    /// it is attached to.
    #[default]
    Attached,

    /// The fragment is an independent physical body.
    ///
    /// It is not attached to any structure and it is most likely in movement
    /// heading towards the ground.
    Debris,

    /// Static/inactive
    ///
    /// The then fragment, and now debris has been on the ground for a
    /// prolonged period of time.
    ///
    /// It is likely scheduled for removal.
    InactiveDebris,
}

ethel::table_spec! {
    struct Fragments {
        parents: [u32; 4];
        influence: [f32; 4];
        rest_offset: glam::Vec3;

        state: FragmentState;
        health: f32; // also acts as mass in Debris state

        position: glam::Vec3;
        velocity: glam::Vec3;
        forces: glam::Vec3;
    }
}

#[derive(Debug)]
pub struct FragmentSystem {
    fragments: FragmentsRowTable,

    // sparse map of node ID to sequence of fragment IDs
    node_map: Vec<Vec<u32>>,

    // alltime accumulated set of disabled node IDs; avoids dedup op
    disabled_nodes: FxHashSet<u32>,

    // alltime accumulated set of disable fragment IDs; avoids dedup op
    // these are the fragments' indirect indices (stable)
    disabled_frags_alltime: FxHashSet<u32>,
    // per-frame list of disabled fragment IDs
    // these are the fragments' direct indices (unstable)
    disabled_frags_frame: Vec<u32>,
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

            disabled_nodes: FxHashSet::default(),
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

            disabled_nodes: FxHashSet::default(),
            disabled_frags_alltime: FxHashSet::default(),
            disabled_frags_frame: Vec::new(),
        }
    }

    /// Get a slice to the fragments associated to `node`.
    ///
    /// # Panics
    /// Will panic if `node` is out-of-bounds; i.e. the node has not been
    /// registered with [`FragmentSystem::generate_fragments`].
    ///
    /// This will not panic if the `node` has no associated fragments: an empty
    /// slice will be returned instead.
    pub fn fragments_of(&self, node: u32) -> &[u32] {
        &self.node_map[node as usize]
    }

    /// Get a mutable slice to the fragments associated to `node`.
    ///
    /// See [`FragmentSystem::fragments_of`] for details on panics.
    pub fn fragments_of_mut(&mut self, node: u32) -> &mut [u32] {
        &mut self.node_map[node as usize]
    }

    pub fn table(&self) -> &FragmentsRowTable {
        &self.fragments
    }

    pub fn table_mut(&mut self) -> &mut FragmentsRowTable {
        &mut self.fragments
    }

    pub fn reset(&mut self) {
        self.disabled_nodes.clear();
        self.node_map.clear();
    }

    pub fn handle_constraint_break(&mut self, broken_ids: &[u32], constraints: &LinksRowTable) {
        self.disabled_frags_frame.clear();
        {
            let f_handles = self.fragments.handles();
            let relations = constraints.relation_slice();

            for broken in broken_ids {
                let index = unsafe { constraints.get_indirect_unchecked(*broken) };
                let LinkNodes(a, b) = *unsafe { relations.get_unchecked(index as usize) };

                if self.disabled_nodes.insert(a) {
                    for &frag_id in &self.node_map[a as usize] {
                        if frag_id == 0 {
                            continue;
                        }
                        if self.disabled_frags_alltime.insert(frag_id) {
                            let index = *unsafe { f_handles.get_unchecked(frag_id as usize) };
                            self.disabled_frags_frame.push(index);
                        }
                    }
                }
                if self.disabled_nodes.insert(b) {
                    for &frag_id in &self.node_map[b as usize] {
                        if frag_id == 0 {
                            continue;
                        }
                        if self.disabled_frags_alltime.insert(frag_id) {
                            let index = *unsafe { f_handles.get_unchecked(frag_id as usize) };
                            self.disabled_frags_frame.push(index);
                        }
                    }
                }
            }
        }

        let states = self.fragments.state_mut_slice();
        self.disabled_frags_frame.iter().for_each(|&frag_id| {
            *unsafe { states.get_unchecked_mut(frag_id as usize) } = FragmentState::Debris;
        });
    }

    /// Return a slice containing the *direct indices* of all fragments
    /// disabled in the last frame.
    ///
    /// Note: this returns **direct indices**; these are the direct element
    /// indices inside of the fragments table. These are not stable handles.
    ///
    /// These are unstable and may change from one frame to another; they are
    /// intended for use only during the same frame this was populated in and
    /// before any operation that might add/remove elements to the table.
    pub fn frame_disabled_frags_direct(&self) -> &[u32] {
        &self.disabled_frags_frame
    }

    const LATTICE_SPATIAL_RESOLUTION: u32 = 2;
    const QUERY_MAX_RANGE: u32 = 3 * Self::LATTICE_SPATIAL_RESOLUTION;

    /// Generate new fragments from a [`VoxelGrid`] and `lattice`.
    ///
    /// The `voxels` [`VoxelGrid`] is expected to have been built previously
    /// with [`VoxelGrid::build`].
    ///
    /// The `lattice` slices indicate the node IDs and node positions. These
    /// must be parallel to one another: each node ID must correspond to its
    /// node's position at the same index.
    pub fn generate_fragments(
        &mut self,
        grid: &VoxelGrid,
        (owners, handles, positions): (&[u32], &[u32], &[glam::Vec3]),
    ) {
        let node_hash = {
            let mut node_hash = FxSpatialHash::with_capacity(
                SpatialResolution::new(Self::LATTICE_SPATIAL_RESOLUTION),
                handles.len(),
            );
            node_hash.dump_soa(positions, handles);
            node_hash
        };

        let len = handles.len();
        for _ in 0..len {
            self.node_map.push(Vec::<u32>::new());
        }

        let mut near_buf = Vec::with_capacity(4);
        let voxels = grid.voxels().values();
        let mut i = 0;
        for &voxel in voxels {
            let cell = node_hash.cell_at(voxel);

            #[cfg(not(debug_assertions))]
            let _ = node_hash.nearest_cells(
                cell,
                4,
                Self::QUERY_MAX_RANGE,
                Self::QUERY_MAX_RANGE,
                Self::QUERY_MAX_RANGE,
                &mut near_buf,
            );

            #[cfg(debug_assertions)]
            {
                if let Err(rem) = node_hash.nearest_cells(
                    cell,
                    4,
                    Self::QUERY_MAX_RANGE,
                    Self::QUERY_MAX_RANGE,
                    Self::QUERY_MAX_RANGE,
                    &mut near_buf,
                ) {
                    tracing::event!(
                        name: "structure.fragment.build.query.err_maybe_miss",
                        tracing::Level::ERROR,
                        "Query for nearby nodes to {cell:?} could not produce {rem} amount of nodes within range {}: maybe a miss? or lattice is malformed.",
                        Self::QUERY_MAX_RANGE
                    )
                }
            }

            let n_count = near_buf.len().min(4);
            if n_count == 0 {
                tracing::event!(
                    name: "structure.fragment.build.query.skip_voxel",
                    tracing::Level::WARN,
                    "Skipping voxel {cell:?}: no nearby nodes in spatial hash."
                );
                continue;
            }

            let (parents, weights, mut rest_offset) = {
                let (mut parents, mut weights) = ([0u32; 4], [0f32; 4]);
                let mut rest_offset = glam::Vec3::ZERO;

                near_buf
                    .drain(..n_count)
                    .zip(&mut parents.iter_mut().zip(&mut weights))
                    .for_each(|(cell, (id, weight))| {
                        *id = node_hash.get(&cell).copied().unwrap_or_default();
                        let point = positions[owners[*id as usize] as usize];
                        *weight = voxel.distance_squared(point);
                    });

                let w_t = weights.iter().fold(0f32, |t, &v| t + v);
                weights.iter_mut().for_each(|v| *v /= w_t);

                parents
                    .iter()
                    .zip(&weights)
                    .take(n_count)
                    .for_each(|(&parent, &weight)| {
                        let point = positions[owners[parent as usize] as usize];
                        rest_offset += point * weight;
                    });

                (parents, weights, rest_offset)
            };
            rest_offset = voxel - rest_offset;

            let handle = self.fragments.put((
                parents,
                weights,
                rest_offset,
                FragmentState::Attached,
                100.0, //todo; health
                voxel,
                glam::Vec3::ZERO,
                glam::Vec3::ZERO,
            ));
            i += 1;

            for node in parents {
                self.node_map[node as usize].push(handle);
            }
        }

        println!("done; {i} fragments");
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VoxelCell {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl From<Cell> for VoxelCell {
    fn from(value: Cell) -> Self {
        VoxelCell {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}

impl Into<Cell> for VoxelCell {
    fn into(self) -> Cell {
        Cell {
            x: self.x,
            y: self.y,
            z: self.z,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct VoxelGridOptions {
    width: f32,
    height: f32,
    depth: f32,
    density: i32,
}

impl Default for VoxelGridOptions {
    fn default() -> Self {
        Self::new(1f32, 1f32, 1f32, 1)
    }
}

impl VoxelGridOptions {
    pub fn new(width: f32, height: f32, depth: f32, density: i32) -> Self {
        Self {
            width,
            height,
            depth,
            density,
        }
    }

    pub fn with_width(self, width: f32) -> Self {
        Self {
            width,
            height: self.height,
            depth: self.depth,
            density: self.density,
        }
    }

    pub fn with_height(self, height: f32) -> Self {
        Self {
            height,
            width: self.width,
            depth: self.depth,
            density: self.density,
        }
    }

    pub fn with_depth(self, depth: f32) -> Self {
        Self {
            depth,
            width: self.width,
            height: self.height,
            density: self.density,
        }
    }

    pub fn with_density(self, density: i32) -> Self {
        Self {
            density,
            width: self.width,
            height: self.height,
            depth: self.depth,
        }
    }
}

pub type VoxelGridFn = fn(VoxelCell) -> bool;
pub type VoxelOffsetFn = fn(VoxelCell) -> glam::Vec3;

#[derive(Clone, Debug)]
pub struct VoxelGrid {
    generator: VoxelGridFn,
    offset_fn: VoxelOffsetFn,
    options: VoxelGridOptions,

    voxels: std::collections::HashMap<VoxelCell, glam::Vec3>,
}

impl Default for VoxelGrid {
    fn default() -> Self {
        Self {
            generator: |_| true,
            offset_fn: |_| glam::Vec3::ZERO,
            options: VoxelGridOptions::default(),
            voxels: Default::default(),
        }
    }
}

impl VoxelGrid {
    pub fn new(generator: VoxelGridFn, options: VoxelGridOptions) -> Self {
        Self {
            generator,
            options,
            ..Default::default()
        }
    }

    pub fn with_offsets(
        generator: VoxelGridFn,
        options: VoxelGridOptions,
        offset_fn: VoxelOffsetFn,
    ) -> Self {
        Self {
            generator,
            options,
            offset_fn,
            ..Default::default()
        }
    }

    pub fn build(&mut self, center: glam::Vec3) {
        self.voxels.clear();

        let vw = (self.options.density as f32 * self.options.width) as i32;
        let vh = (self.options.density as f32 * self.options.height) as i32;
        let vd = (self.options.density as f32 * self.options.depth) as i32;

        let hvw = vw / 2;
        let hvh = vh / 2;
        let hvd = vd / 2;

        for x in -hvw..hvw {
            for y in -hvh..hvh {
                for z in -hvd..hvd {
                    let cell = VoxelCell { x, y, z };
                    if (self.generator)(cell) {
                        let position = glam::vec3(
                            (cell.x as f32 / vw as f32) * self.options.width,
                            (cell.y as f32 / vh as f32) * self.options.height,
                            (cell.z as f32 / vd as f32) * self.options.depth,
                        );
                        let offset = (self.offset_fn)(cell);
                        self.voxels.insert(cell, center + position + offset);
                    }
                }
            }
        }
    }

    pub fn get_voxel(&self, cell: VoxelCell) -> Option<glam::Vec3> {
        self.voxels.get(&cell).copied()
    }

    pub fn options(&self) -> &VoxelGridOptions {
        &self.options
    }

    pub fn options_mut(&mut self) -> &mut VoxelGridOptions {
        &mut self.options
    }

    pub fn voxels(&self) -> &std::collections::HashMap<VoxelCell, glam::Vec3> {
        &self.voxels
    }

    pub fn count(&self) -> usize {
        self.voxels.len()
    }
}
