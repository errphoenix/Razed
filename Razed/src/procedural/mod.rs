pub mod voronoi;
pub mod voxel;

use ethel::mesh::MeshStaging;
pub use voronoi::{CubeVoronoi, CubeVoronoiGenerator};
pub use voxel::{VoxelGrid, VoxelGridOptions};

pub fn cubic_voronoi_alloc(
    seeds: &[glam::Vec3],
    unit: glam::Vec3,
    max_shift: f32,
    seek_range: f32,
) -> CubeVoronoi {
    let mut generator = CubeVoronoiGenerator::new(rand::rng(), max_shift);
    generator.generate(seeds, unit, seek_range);
    generator.consolidate_alloc()
}

/// Minimal allocation alternative for [`cubic_voronoi_alloc`].
///
/// The given `out_stage_buffer` is owned by the generator during mesh
/// generation, and then returned back in [`CubeVoronoi`].
///
/// Note: while this does not allocate a new [`MeshStaging`] buffer, this
/// still allocated a new collection for `seeds`.
/// See [`CubeVoronoiGenerator::generate`].
pub fn cubic_voronoi(
    seeds: &[glam::Vec3],
    unit: glam::Vec3,
    max_shift: f32,
    seek_range: f32,
    out_stage_buffer: MeshStaging,
) -> CubeVoronoi {
    let mut generator = CubeVoronoiGenerator::new(rand::rng(), max_shift);
    generator.generate(seeds, unit, seek_range);
    generator.consolidate(out_stage_buffer)
}

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
    condition: voxel::VoxelGridFn,
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
