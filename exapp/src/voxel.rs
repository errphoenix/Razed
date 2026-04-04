use ethel::state::data::hash::{Cell, FxSpatialHash, SpatialResolution};

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct VoxelGridOptions {
    pub width: f32,
    pub height: f32,
    pub depth: f32,
    pub cell_size: f32,
}

impl Default for VoxelGridOptions {
    fn default() -> Self {
        Self::new(1f32, 1f32, 1f32, 1f32)
    }
}

impl VoxelGridOptions {
    pub fn new(width: f32, height: f32, depth: f32, cell_size: f32) -> Self {
        Self {
            width,
            height,
            depth,
            cell_size,
        }
    }

    pub fn with_width(self, width: f32) -> Self {
        Self {
            width,
            height: self.height,
            depth: self.depth,
            cell_size: self.cell_size,
        }
    }

    pub fn with_height(self, height: f32) -> Self {
        Self {
            height,
            width: self.width,
            depth: self.depth,
            cell_size: self.cell_size,
        }
    }

    pub fn with_depth(self, depth: f32) -> Self {
        Self {
            depth,
            width: self.width,
            height: self.height,
            cell_size: self.cell_size,
        }
    }

    pub fn with_cell_size(self, cell_size: f32) -> Self {
        Self {
            cell_size,
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
    pub generator: VoxelGridFn,
    options: VoxelGridOptions,

    voxels: FxSpatialHash<VoxelIndex>,
}

impl Default for VoxelGrid {
    fn default() -> Self {
        let options = VoxelGridOptions::default();
        let voxels = FxSpatialHash::new(SpatialResolution::new(options.cell_size));

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
            voxels: FxSpatialHash::new(SpatialResolution::new(options.cell_size)),
        }
    }

    pub fn voxel_index(&self, cell: Cell) -> VoxelIndex {
        let (dim_x, dim_y, dim_z) = self.dimensions();

        #[cfg(debug_assertions)]
        {
            let x_bounds = dim_x / 2;
            let y_bounds = dim_y / 2;
            let z_bounds = dim_z / 2;

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
            x: cell.x + dim_x / 2,
            y: cell.y + dim_y / 2,
            z: cell.z + dim_z / 2,
        };

        VoxelIndex(cell.x * dim_y * dim_z + cell.y * dim_z + cell.z)
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
            (cell.x as f32 + 0.5) / self.options.cell_size as f32,
            (cell.y as f32 + 0.5) / self.options.cell_size as f32,
            (cell.z as f32 + 0.5) / self.options.cell_size as f32,
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
        let (dim_x, dim_y, dim_z) = self.dimensions();

        let yz = dim_y * dim_z;

        let cx = index / yz;
        let rem = index % yz;
        let cy = rem / dim_z;
        let cz = rem % dim_z;

        Cell {
            x: cx - dim_x / 2,
            y: cy - dim_y / 2,
            z: cz - dim_z / 2,
        }
    }

    pub fn repopulate(&mut self) {
        self.voxels.clear();

        let (vw, vh, vd) = self.dimensions();

        let hvw = vw / 2;
        let hvh = vh / 2;
        let hvd = vd / 2;

        for x in -hvw..=hvw {
            for y in -hvh..=hvh {
                for z in -hvd..=hvd {
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
        self.voxels.get(cell).copied()
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
    pub fn dimensions(&self) -> (i32, i32, i32) {
        let w = self.options.width;
        let h = self.options.height;
        let d = self.options.depth;
        let i = self.options.cell_size;
        (
            (w * i as f32).round() as i32,
            (h * i as f32).round() as i32,
            (d * i as f32).round() as i32,
        )
    }
}
