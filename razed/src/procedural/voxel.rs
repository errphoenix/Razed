use ethel::{
    data::hash::FxLsSpatialHash,
    state::data::hash::{Cell, FxSpatialHash, SpatialResolution},
};
use polys::{TriFace, convex::Convex};
use std::fmt::Debug;

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

    #[allow(unused)]
    pub fn with_cell_size(self, cell_size: f32) -> Self {
        Self {
            cell_size,
            width: self.width,
            height: self.height,
            depth: self.depth,
        }
    }
}

pub trait VoxelFn {
    fn pass(&self, cell: Cell) -> bool;
}
impl<F> VoxelFn for F
where
    F: Fn(Cell) -> bool,
{
    fn pass(&self, cell: Cell) -> bool {
        self(cell)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NullVoxelFn;
impl VoxelFn for NullVoxelFn {
    fn pass(&self, _cell: Cell) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FullVoxelFn;
impl VoxelFn for FullVoxelFn {
    fn pass(&self, _cell: Cell) -> bool {
        true
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AlternatingVoxelFn;
impl VoxelFn for AlternatingVoxelFn {
    fn pass(&self, cell: Cell) -> bool {
        if cell.y % 2 == 0 {
            cell.x % 2 == 0 && cell.z % 2 == 0
        } else {
            cell.x % 2 == 1 && cell.z % 2 == 1
        }
    }
}

pub struct VoxelGrid<'func> {
    pub generator: Box<dyn VoxelFn + 'func>,
    options: VoxelGridOptions,

    voxels: FxSpatialHash<glam::Vec3>,
}
impl<'func> std::fmt::Debug for VoxelGrid<'func> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoxelGrid")
            .field("options", &self.options)
            .field("voxels", &self.voxels)
            .finish_non_exhaustive()
    }
}
impl<'func> Default for VoxelGrid<'func> {
    fn default() -> Self {
        let options = VoxelGridOptions::default();
        let voxels = FxSpatialHash::new(SpatialResolution::new(options.cell_size));

        Self {
            generator: Box::new(FullVoxelFn),
            options,
            voxels,
        }
    }
}
impl<'func> VoxelGrid<'func> {
    pub fn new<F: VoxelFn + 'func>(generator: F, options: VoxelGridOptions) -> Self {
        Self {
            generator: Box::new(generator),
            options,
            voxels: FxSpatialHash::new(SpatialResolution::new(options.cell_size)),
        }
    }

    /// Creates an absolute-space copy of this [`VoxelGrid`].
    ///
    /// With "absolute-space" is intended that all [`Cell`]s part of the grid
    /// have a positive coordinate. This is not a simple `.abs()` operation:
    /// all cells are offset in a way that maintains their spatial relations
    /// to each-other.
    ///
    /// Essentially, this operation moves the grid's "origin" (0,0,0) from the
    /// center to the lowest left-most point of the grid.
    ///
    /// # Panics
    /// If this [`VoxelGrid`] is empty, as the lowest left-most point cannot be
    /// determined.
    pub fn make_abs(&mut self) {
        let lowest_cell = {
            let mut min_cell = Cell::MAX;
            for &cell in self.voxels.cells() {
                min_cell = min_cell.min(cell);
            }
            min_cell
        };
        let lowest_cell_point = self.voxels.resolution.approx_point(lowest_cell);

        let mut voxels = FxSpatialHash::with_capacity(self.voxels.resolution, self.voxels.len());
        for (&cell, &point) in self.voxels.cells().zip(self.voxels.elements()) {
            let a_cell = cell - lowest_cell;
            let a_point = point - lowest_cell_point;
            voxels.put(a_cell, a_point);
        }

        self.voxels = voxels;
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

        let hvw = vw / 2;
        let hvh = vh / 2;
        let hvd = vd / 2;

        for x in -hvw..(hvw + (vw % 2)) {
            for y in -hvh..(hvh + (vh % 2)) {
                for z in -hvd..(hvd + (vd % 2)) {
                    let cell = Cell { x, y, z };
                    if self.generator.pass(cell) {
                        let point = point_from_cell(cell);
                        self.voxels.put(cell, point);
                    }
                }
            }
        }
    }

    pub const fn options(&self) -> &VoxelGridOptions {
        &self.options
    }

    pub const fn voxels(&self) -> &FxSpatialHash<glam::Vec3> {
        &self.voxels
    }

    pub fn to_voxels(self) -> FxSpatialHash<glam::Vec3> {
        self.voxels
    }

    pub fn count(&self) -> usize {
        self.voxels.len()
    }

    /// Returns the maximum amount of cells along each X, Y, Z plane.
    ///
    /// This value depends on the width, height, depth, and density options
    /// specified in [`VoxelGridOptions`].
    pub const fn dimensions(&self) -> (i32, i32, i32) {
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

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct MeshCell {
    pub vertex: u32,
    pub triangle: u32,
}

#[derive(Debug)]
pub struct MeshVolumeHash {
    mesh_hash: FxLsSpatialHash<MeshCell>,
    voxels: FxSpatialHash<glam::Vec3>,
    size: glam::Vec3,
}
impl MeshVolumeHash {
    const SHASH: SpatialResolution = SpatialResolution::new(1.0);

    pub fn new(mesh: &Convex<TriFace>) -> Self {
        let mut mesh_hash = FxLsSpatialHash::new(Self::SHASH);
        Self::hash_coarse(mesh, &mut mesh_hash);
        let size = {
            let min = mesh_hash.min();
            let max = mesh_hash.max();
            let dx = (max.x - min.x) as f32;
            let dy = (max.y - min.y) as f32;
            let dz = (max.z - min.z) as f32;
            glam::vec3(dx, dy, dz)
        };
        let voxels = {
            let options = VoxelGridOptions::new(size.x, size.y, size.z, 1.0);
            let mut vg = VoxelGrid::new(|cell| Self::voxel_generator(cell, &mesh_hash), options);
            vg.repopulate_defaults();
            vg.voxels
        };
        Self {
            mesh_hash,
            voxels,
            size,
        }
    }

    fn voxel_generator(cell: Cell, hash: &FxLsSpatialHash<MeshCell>) -> bool {
        hash.get(cell).is_some()
    }

    fn hash_coarse(mesh: &Convex<TriFace>, hash: &mut FxLsSpatialHash<MeshCell>) {
        for (i, tri) in mesh.faces().iter().enumerate() {
            let triangle = i as u32;
            for j in 0..3 {
                let vertex = tri[j];
                let pos = mesh.vertices()[vertex as usize];
                let cell = Self::SHASH.encode_point(pos);
                hash.put(cell, MeshCell { vertex, triangle });
            }
        }
    }

    pub fn get_extents_on_axis(&self, axis: glam::Vec3) -> glam::Vec3 {
        let unit = axis.normalize();
        let max_extent = self.size.max_element();
        let max_cells = max_extent as u32;
        let mut last = glam::Vec3::ZERO;
        for i in 0..max_cells {
            let lookup = Self::SHASH.encode_point(unit * i as f32);
            if let Some(&point) = self.voxels.get(lookup) {
                last = point;
            }
        }
        last
    }

    pub const fn coarse_extents(&self) -> glam::Vec3 {
        self.size
    }

    pub const fn voxels(&self) -> &FxSpatialHash<glam::Vec3> {
        &self.voxels
    }

    pub const fn voxels_mut(&mut self) -> &mut FxSpatialHash<glam::Vec3> {
        &mut self.voxels
    }

    pub const fn hash_mesh(&self) -> &FxLsSpatialHash<MeshCell> {
        &self.mesh_hash
    }

    pub const fn hash_mesh_mut(&mut self) -> &mut FxLsSpatialHash<MeshCell> {
        &mut self.mesh_hash
    }
}
