use crate::data::FRAGMENTS_PARENTS_COUNT;
use ethel::state::data::{
    Column,
    hash::{Cell, FxSpatialHash, SpatialResolution},
};
use physics::xpbd::{LinkNodes, LinksRowTable};
use rustc_hash::FxHashSet;

const MIN_CLUSTER_SIZE: u32 = 6;

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

ethel::table_spec! {
    struct Fragments {
        parents: [u32; FRAGMENTS_PARENTS_COUNT];
        influence: [f32; FRAGMENTS_PARENTS_COUNT];
        // bind pose world position
        bind_world: glam::Vec4;

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

    /// sparse map of node ID to sequence of fragment IDs
    node_map: Vec<Vec<u32>>,

    /// alltime accumulated set of disabled node IDs; avoids dedup op
    disabled_nodes: FxHashSet<u32>,

    /// alltime accumulated set of disable fragment IDs; avoids dedup op
    /// these are the fragments' indirect indices (stable)
    disabled_frags_alltime: FxHashSet<u32>,

    /// per-frame list of disabled fragment IDs and an indirect node
    /// these are the fragments' direct indices (unstable)
    disabled_frags_frame: Vec<(u32, u32)>,
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

    /// Synchronise (stable II) `broken_ids` of constraints and
    /// [`degenerate_nodes`] with fragments state.
    pub fn handle_constraint_break(
        &mut self,
        broken_ids: &[u32],
        degenerate_nodes: &[u32],
        constraints: &LinksRowTable,
    ) {
        self.disabled_frags_frame.clear();
        for &degen in degenerate_nodes {
            if self.disabled_nodes.insert(degen) {
                for &frag_id in &self.node_map[degen as usize] {
                    if frag_id == 0 {
                        continue;
                    }

                    if self.disabled_frags_alltime.insert(frag_id) {
                        let idx = unsafe { self.fragments.get_indirect_unchecked(frag_id) };
                        self.disabled_frags_frame.push((idx, degen));
                    }
                }
            }
        }

        {
            let relations = constraints.relation_slice();
            for broken in broken_ids {
                let index = unsafe { constraints.get_indirect_unchecked(*broken) };
                let LinkNodes(a, b) = *unsafe { relations.get_unchecked(index as usize) };

                if self.disabled_nodes.insert(a) {
                    for frag_id in &self.node_map[a as usize] {
                        if *frag_id == 0 {
                            continue;
                        }
                        if self.disabled_frags_alltime.insert(*frag_id) {
                            let index = unsafe { self.fragments.get_indirect_unchecked(*frag_id) };
                            self.disabled_frags_frame.push((index, a));
                        }
                    }
                }
                if self.disabled_nodes.insert(b) {
                    for &frag_id in &self.node_map[b as usize] {
                        if frag_id == 0 {
                            continue;
                        }
                        if self.disabled_frags_alltime.insert(frag_id) {
                            let index = unsafe { self.fragments.get_indirect_unchecked(frag_id) };
                            self.disabled_frags_frame.push((index, b));
                        }
                    }
                }
            }
        }

        // validate disabled fragments and invalidate relations
        let (parents, weights, _, states, _, _, _, _) = self.fragments.split_mut();
        self.disabled_frags_frame.retain(|&(frag_idx, node_id)| {
            let parents = unsafe { parents.alpha.get_unchecked_mut(frag_idx as usize) };
            let weights = unsafe { weights.alpha.get_unchecked_mut(frag_idx as usize) };

            parents
                .iter_mut()
                .zip(weights.iter_mut())
                .for_each(|(id, weight)| {
                    if *id == node_id {
                        *id = 0;
                        *weight = 0.0;
                    }
                });

            // recalibrate weights
            let w_t = weights.iter().fold(0f32, |v0, vi| v0 + *vi);
            for w in weights {
                *w /= w_t;
            }

            let active_count = parents.iter().filter(|id| **id != 0).count();
            if active_count < MIN_CLUSTER_SIZE as usize {
                let state = unsafe { states.alpha.get_unchecked_mut(frag_idx as usize) };
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
    pub fn frame_disabled_frags_direct(&self) -> &[(u32, u32)] {
        &self.disabled_frags_frame
    }

    const LATTICE_SPATIAL_RESOLUTION: u32 = 1;
    const VOXEL_NEIGHBOR_QUERY_RADIUS: u32 = 4;

    /// Generate new fragments from a [`VoxelGrid`] and raw lattice data.
    ///
    /// The `voxels` [`VoxelGrid`] is expected to have been built previously
    /// with [`VoxelGrid::build`].
    ///
    /// The `(owners, handles, positions)` tuple parameter refer, in order, to
    /// the following Node table data of any [`NodesRowTable`]:
    /// * `owners` as in the collection of sparse slot indices as returned from
    ///   [`ethel::state::data::SparseSlot::slots_map`].
    /// * `handles` as in the data parallel collection of inverse owner indices
    ///   for each table element, may be returned by a method of the same naem.
    /// * `positions` the data slice of positions for each node parallel to
    ///   `handles`.
    pub fn generate_fragments(
        &mut self,
        origin: glam::Vec3,
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

        let mut near_buf = Vec::with_capacity(FRAGMENTS_PARENTS_COUNT);
        let mut world_points = vec![glam::Vec3::ZERO; grid.count()];

        grid.to_world(origin, &mut world_points);
        let mut i = 0;
        for voxel in world_points {
            let cell = node_hash.cell_at(voxel);

            #[cfg(not(debug_assertions))]
            let _ = node_hash.nearest_cells(
                cell,
                FRAGMENTS_PARENTS_COUNT as u32,
                Self::VOXEL_NEIGHBOR_QUERY_RADIUS,
                &mut near_buf,
                false,
            );

            #[cfg(debug_assertions)]
            {
                if let Err(rem) = node_hash.nearest_cells(
                    cell,
                    FRAGMENTS_PARENTS_COUNT as u32,
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

                    let ii0 = owners[*id0 as usize];
                    let ii1 = owners[*id1 as usize];
                    let p0 = positions[ii0 as usize];
                    let p1 = positions[ii1 as usize];

                    let l0 = p0 - cell_world;
                    let l1 = p1 - cell_world;

                    l0.length_squared()
                        .partial_cmp(&l1.length_squared())
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }

            let n_count = near_buf.len().min(FRAGMENTS_PARENTS_COUNT);

            // if n_count < MIN_CLUSTER_SIZE as usize {
            //     tracing::event!(
            //         name: "structure.fragment.build.query.skip_voxel",
            //         tracing::Level::WARN,
            //         "Skipping voxel {cell:?}: not enough {n_count} nearby nodes found."
            //     );
            //     continue;
            // }

            let (parents, weights) = {
                let (mut parents, mut weights) = (
                    [0u32; FRAGMENTS_PARENTS_COUNT],
                    [0f32; FRAGMENTS_PARENTS_COUNT],
                );

                near_buf
                    .drain(..)
                    .take(n_count)
                    .zip(&mut parents.iter_mut().zip(&mut weights))
                    .for_each(|(cell, (id, weight))| {
                        *id = node_hash.get(&cell).copied().unwrap_or_default();
                        let point = positions[owners[*id as usize] as usize];
                        let ds = voxel.distance_squared(point);
                        *weight = 1.0 / (ds + f32::EPSILON);
                    });

                let w_t = weights.iter().fold(0f32, |t, &v| t + v);
                weights.iter_mut().for_each(|v| *v /= w_t);

                (parents, weights)
            };

            let handle = self.fragments.put((
                parents,
                weights,
                glam::vec4(voxel.x, voxel.y, voxel.z, 1f32),
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

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct VoxelGridOptions {
    pub width: f32,
    pub height: f32,
    pub depth: f32,
    pub density: i32,
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

pub type VoxelGridFn = fn(Cell) -> bool;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VoxelIndex(i32);

impl VoxelIndex {
    pub fn as_i32(&self) -> i32 {
        self.0
    }
}

impl From<VoxelIndex> for i32 {
    fn from(value: VoxelIndex) -> Self {
        value.0
    }
}

#[derive(Clone, Debug)]
pub struct VoxelGrid {
    generator: VoxelGridFn,
    options: VoxelGridOptions,

    voxels: FxSpatialHash<VoxelIndex>,
}

impl Default for VoxelGrid {
    fn default() -> Self {
        let options = VoxelGridOptions::default();
        let voxels = FxSpatialHash::new(SpatialResolution::new(options.density as u32));

        Self {
            generator: |_| true,
            options,
            voxels,
        }
    }
}

impl VoxelGrid {
    pub fn new(generator: VoxelGridFn, options: VoxelGridOptions) -> Self {
        Self {
            generator,
            options,
            voxels: FxSpatialHash::new(SpatialResolution::new(options.density as u32)),
        }
    }

    pub fn voxel_index(&self, cell: Cell) -> VoxelIndex {
        let x_offset = (self.options.width * self.options.density as f32).round() as i32;
        let y_offset = (self.options.height * self.options.density as f32).round() as i32;
        let z_offset = (self.options.depth * self.options.density as f32).round() as i32;

        #[cfg(debug_assertions)]
        {
            let x_bounds = x_offset / 2;
            let y_bounds = y_offset / 2;
            let z_bounds = z_offset / 2;

            debug_assert!(
                cell.x >= -x_bounds && cell.x <= x_bounds,
                "Cell is out of bounds on X axis for bounds [{}; {}]: got {}",
                -x_bounds,
                x_bounds,
                cell.x
            );
            debug_assert!(
                cell.y >= -y_bounds && cell.y <= y_bounds,
                "Cell is out of bounds on Y axis for bounds [{}; {}]: got {}",
                -y_bounds,
                y_bounds,
                cell.y
            );
            debug_assert!(
                cell.z >= -z_bounds && cell.z <= z_bounds,
                "Cell is out of bounds on Z axis for bounds [{}; {}]: got {}",
                -z_bounds,
                z_bounds,
                cell.z
            );
        }

        let cell = Cell {
            x: cell.x + x_offset / 2,
            y: cell.y + y_offset / 2,
            z: cell.z + z_offset / 2,
        };

        VoxelIndex(cell.x * y_offset * z_offset + cell.y * z_offset + cell.z)
    }

    /// Transform an [`index`] to a point in space.
    ///
    /// The returned point corresponds to the center of the
    /// [`Voxel/Cell`](Cell) represented by `index`.
    ///
    /// The returned point is in the [`VoxelGrid's](VoxelGrid) local space,
    /// with Vec3(0,0,0) located at its centre.
    pub fn point_from_id(&self, index: VoxelIndex) -> glam::Vec3 {
        let cell = self.cell_from_id(index);
        glam::vec3(
            (cell.x as f32 + 0.5) / self.options.density as f32,
            (cell.y as f32 + 0.5) / self.options.density as f32,
            (cell.z as f32 + 0.5) / self.options.density as f32,
        )
    }

    /// Decode a [`Cell`] within an [`index`].
    ///
    /// This is in the [`VoxelGrid's`](VoxelGrid) local space and units, with
    /// Cell(0,0,0) located at its centre.
    ///
    /// This is not to be used in combination with other [`VoxelGrid`]s or
    /// world-space operations, unless you can guarantee:
    /// * They are in the same space with the same origin
    /// * If it is another [`VoxelGrid`], they must use the same spatial
    ///   resolution.
    ///
    /// Also see [`VoxelGrid::point_from_id`].
    pub fn cell_from_id(&self, index: VoxelIndex) -> Cell {
        let index = index.as_i32();
        let x_offset = (self.options.width * self.options.density as f32).round() as i32;
        let y_offset = (self.options.height * self.options.density as f32).round() as i32;
        let z_offset = (self.options.depth * self.options.density as f32).round() as i32;

        let yz = y_offset * z_offset;
        let rem = index % yz;

        let cx = index / yz;
        let cy = rem / z_offset;
        let cz = rem % z_offset;

        Cell {
            x: cx - x_offset / 2,
            y: cy - y_offset / 2,
            z: cz - z_offset / 2,
        }
    }

    pub fn repopulate(&mut self) {
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
                    let cell = Cell { x, y, z };
                    if (self.generator)(cell) {
                        self.voxels.put(cell, self.voxel_index(cell));
                    }
                }
            }
        }
    }

    pub fn to_world(&self, origin: glam::Vec3, world: &mut [glam::Vec3]) {
        self.voxels
            .elements()
            .zip(world)
            .for_each(|(&id, world)| *world = self.point_from_id(id) + origin);
    }

    pub fn get(&self, cell: Cell) -> Option<VoxelIndex> {
        self.voxels.get(&cell).copied()
    }

    pub fn options(&self) -> &VoxelGridOptions {
        &self.options
    }

    pub fn voxels(&self) -> &FxSpatialHash<VoxelIndex> {
        &self.voxels
    }

    pub fn count(&self) -> usize {
        self.voxels.len()
    }

    /// Returns the maximum amount of cells along each X, Y, Z plane.
    ///
    /// This value depends on the width, height, depth, and density options
    /// specified in [`VoxelGridOptions`].
    pub fn cell_bounds(&self) -> (i32, i32, i32) {
        let w = self.options.width;
        let h = self.options.height;
        let d = self.options.depth;
        let i = self.options.density;
        (
            (w * i as f32).round() as i32,
            (h * i as f32).round() as i32,
            (d * i as f32).round() as i32,
        )
    }
}
