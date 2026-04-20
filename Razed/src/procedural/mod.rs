pub mod voronoi;
pub mod voxel;

pub use voxel::{VoxelGrid, VoxelGridOptions};

use crate::procedural::voxel::VoxelGridFn;

pub fn voxel_grid(width: f32, height: f32, depth: f32, cell_size: f32) -> VoxelGrid {
    VoxelGrid::new(
        |_| true,
        VoxelGridOptions {
            width,
            height,
            depth,
            cell_size,
        },
    )
}

pub fn voxel_grid_cond(
    width: f32,
    height: f32,
    depth: f32,
    cell_size: f32,
    condition: VoxelGridFn,
) -> VoxelGrid {
    VoxelGrid::new(
        condition,
        VoxelGridOptions {
            width,
            height,
            depth,
            cell_size,
        },
    )
}
