use ethel::state::data::{
    Column,
    hash::{Cell, FxSpatialHash, SpatialResolution},
};

use crate::state::physics::XpbdSystem;

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

        state: FragmentState;
        health: f32; // also acts as mass in Debris state

        position: glam::Vec3;
        velocity: glam::Vec3;
        forces: glam::Vec3;
    }
}

#[derive(Debug, Default)]
pub struct FragmentSystem {
    fragments: FragmentsRowTable,

    // sparse map of node ID to sequence of fragment IDs
    node_map: Vec<Vec<u32>>,
}

impl FragmentSystem {
    const LATTICE_SPATIAL_RESOLUTION: u32 = 2;

    /// Initialise a new fragment complex from a [`VoxelGrid`] and `lattice`.
    ///
    /// The `voxels` [`VoxelGrid`] is expected to have been built previously
    /// with [`VoxelGrid::build`].
    pub fn new(voxels: VoxelGrid, lattice: XpbdSystem) -> Self {
        let mut node_hash = FxSpatialHash::with_capacity(
            SpatialResolution::new(Self::LATTICE_SPATIAL_RESOLUTION),
            lattice.nodes().len(),
        );

        {
            let positions = lattice.nodes().current_pos_slice();
            let handles = lattice.nodes().handles();
            node_hash.dump_soa(positions, handles);
        }

        let mut node_map = {
            let len = lattice.nodes().len();
            let mut node_map = Vec::with_capacity(len);
            for _ in 0..len {
                node_map.push(Vec::<u32>::new());
            }
            node_map
        };

        let mut fragments = FragmentsRowTable::with_capacity(voxels.count());

        const QUERY_MAX_RANGE: u32 = 16;

        let mut near_buf = Vec::with_capacity(4);
        let voxels = voxels.voxels().values();

        for &voxel in voxels {
            let cell = node_hash.cell_at(voxel);

            #[cfg(not(debug_assertions))]
            let _ = node_hash.nearest_cells(cell, 4, QUERY_MAX_RANGE, &mut near_buf);

            #[cfg(debug_assertions)]
            {
                if let Err(rem) = node_hash.nearest_cells(cell, 4, QUERY_MAX_RANGE, &mut near_buf) {
                    tracing::event!(
                        name: "structure.fragment.build.query.err_maybe_miss",
                        tracing::Level::DEBUG,
                        "Query for nearby nodes to {cell:?} could not produce {rem} amount of nodes within range {QUERY_MAX_RANGE}: maybe a miss? or lattice is malformed."
                    )
                }
            }

            let (parents, weights) = {
                let (mut parents, mut weights) = ([0u32; 4], [0f32; 4]);

                near_buf
                    .drain(..4)
                    .zip(&mut parents.iter_mut().zip(&mut weights))
                    .for_each(|(cell, (id, weight))| {
                        *id = node_hash.get(&cell).copied().unwrap_or_default();
                        let point = node_hash.approx_point_at(cell);
                        *weight = voxel.distance_squared(point);
                    });

                near_buf.clear();

                let w_t = weights.iter().fold(0f32, |t, &v| t + v);
                weights.iter_mut().for_each(|v| *v /= w_t);
                (parents, weights)
            };

            let handle = fragments.put((
                parents,
                weights,
                FragmentState::Attached,
                100.0, //todo; health
                voxel,
                glam::Vec3::ZERO,
                glam::Vec3::ZERO,
            ));

            for node in parents {
                node_map[node as usize].push(handle);
            }
        }

        Self {
            fragments,
            node_map,
        }
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

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct VoxelGridOptions {
    width: f32,
    height: f32,
    depth: f32,
    density: i32,
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
