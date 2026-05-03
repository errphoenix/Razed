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

#[derive(Clone, Debug)]
pub struct VoxelGrid {
    pub generator: VoxelGridFn,
    options: VoxelGridOptions,

    voxels: FxSpatialHash<glam::Vec3>,
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

    pub fn quantize_point(&self, point: glam::Vec3) -> Cell {
        self.voxels.cell_at(point)
    }

    /// Get the point mapped to `cell`, if present.
    pub fn point_at(&self, cell: Cell) -> Option<glam::Vec3> {
        self.voxels.get(cell).copied()
    }

    /// Get the point mapped to `cell` or an approximation if not present.
    pub fn point_at_or_approx(&self, cell: Cell) -> glam::Vec3 {
        self.point_at(cell)
            .unwrap_or_else(|| self.voxels.approx_point_at(cell))
    }

    pub fn repopulate_defaults(&mut self) {
        let resolution = self.voxels.resolution;
        self.repopulate_with(|cell| resolution.approx_point(cell));
    }

    pub fn repopulate_with<F: Fn(Cell) -> glam::Vec3>(&mut self, point_from_cell: F) {
        self.voxels.clear();

        let (vw, vh, vd) = self.dimensions();

        let hvw = (vw - (vw % 2)) / 2;
        let hvh = (vh - (vh % 2)) / 2;
        let hvd = (vd - (vd % 2)) / 2;

        for x in -hvw..=hvw {
            for y in -hvh..=hvh {
                for z in -hvd..=hvd {
                    let cell = Cell { x, y, z };
                    if (self.generator)(cell) {
                        let point = point_from_cell(cell);
                        self.voxels.put(cell, point);
                    }
                }
            }
        }
    }

    pub fn options(&self) -> &VoxelGridOptions {
        &self.options
    }

    pub fn voxels(&self) -> &FxSpatialHash<glam::Vec3> {
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
            (w / i as f32).round() as i32,
            (h / i as f32).round() as i32,
            (d / i as f32).round() as i32,
        )
    }
}
