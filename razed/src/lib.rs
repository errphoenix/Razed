use std::num::NonZeroU32;

use ethel::{FxSpatialHash, MeshId, MeshStaging, SpatialHash, SpatialResolution, mesh::Vertex};
use physics::Sphere;
use polys::{TriFace, convex::Convex};

use crate::{
    procedural::{CubeVoronoi, cubic_voronoi, voxel_grid},
    render::CommonMeshMetadata,
};

pub mod assets;
pub mod data;
pub mod input;
pub mod procedural;
pub mod render;
pub mod simulation;
pub mod structure;
pub mod ui;

pub const PROFILER_COMMAND_PAUSE: u8 = 0b00000001;
pub const PROFILER_COMMAND_START: u8 = 0b00000010;
pub const PROFILER_COMMAND_DUMP: u8 = 0b00000100;

#[derive(Clone, Debug, Default)]
pub struct FragmentMeshVolumes(Vec<Sphere>);
impl std::ops::Deref for FragmentMeshVolumes {
    type Target = [Sphere];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for FragmentMeshVolumes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl FragmentMeshVolumes {
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    pub fn set(&mut self, mesh: MeshId, volume: Sphere) {
        if self.0.len() as u32 <= mesh.as_int() {
            self.0.resize(mesh.as_int() as usize + 1, Sphere::new(0.0));
        }
        self.0[mesh.as_int() as usize] = volume;
    }

    pub fn get(&self, mesh: MeshId) -> Option<Sphere> {
        self.0.get(mesh.as_int() as usize).copied()
    }
}

/// Assumes `mesh` is in local space ([`Convex::make_local`]).
pub fn approximate_volume_conservative(vertices: &[Vertex]) -> Sphere {
    let mut max = f32::MIN;
    let mut min = f32::MAX;

    for &v in vertices {
        let p = glam::vec3(v.pos_x, v.pos_y, v.pos_z);
        let l = p.length();
        max = max.max(l);
        min = min.min(l);
    }

    const MAX_BIAS: f32 = 0.2;
    let radius = min + (max - min) * MAX_BIAS;
    Sphere { radius }
}

#[derive(Debug, Default)]
pub struct StartupMeshData {
    pub stager: MeshStaging,
    pub fragment_mesh_mapping: FxSpatialHash<MeshId>,
    pub fragment_mesh_volumes: FragmentMeshVolumes,
    pub common_meta: CommonMeshMetadata,
}

pub fn setup_mesh_data() -> StartupMeshData {
    let mut fragment_mesh_volumes = FragmentMeshVolumes::new();
    let mut stager = ethel::mesh::MeshStaging::new();

    let fragment_mesh_mapping = {
        let group =
            generate_fragment_meshes(glam::Vec3::splat(3.0), stager, &mut fragment_mesh_volumes);
        stager = group.voronoi.stager;
        group.mapping
    };

    let common_meta = {
        let mut stage_and_get = |mesh: Convex<TriFace>| {
            let id = mesh.stage(&mut stager);
            let meta = stager.metadata().last().copied().unwrap();
            (id, meta)
        };

        let unit_sphere = polys::generate_sphere(1.0, NonZeroU32::new(32).unwrap());
        let (id_sphere, meta_sphere) = stage_and_get(unit_sphere);

        // select last fragment mesh as boulder
        let id_boulder = fragment_mesh_mapping.elements().last().copied().unwrap();
        let meta_boulder = *stager.metadata().get(id_boulder);

        CommonMeshMetadata {
            meta_sphere,
            meta_boulder,
            id_sphere,
            id_boulder,
        }
    };

    StartupMeshData {
        stager,
        fragment_mesh_mapping,
        fragment_mesh_volumes,
        common_meta,
    }
}

#[derive(Debug)]
struct FragmentGroup {
    #[allow(unused)]
    pub cubic_area: glam::Vec3,
    pub voronoi: CubeVoronoi,
    pub mapping: FxSpatialHash<ethel::mesh::Id>,
}

fn generate_fragment_meshes(
    cubic_area: glam::Vec3,
    mesh_stage: MeshStaging,
    volumes: &mut FragmentMeshVolumes,
) -> FragmentGroup {
    const FRAG_UNIT: f32 = 1.0;
    const MAX_SHIFT: f32 = 0.8;
    const SEEK_RANGE: f32 = 2.0;

    let mut grid = voxel_grid(cubic_area.x, cubic_area.y, cubic_area.z, FRAG_UNIT);

    grid.repopulate_defaults();
    let count = grid.count();

    // CubicVoronoiGenerator is guaranteed to process the seeds and meshes by
    // the same order they are in the given seeds collection.
    // we can use this to map the meshes by the generator to a local cell
    // coordinate.
    let mut seeds = Vec::with_capacity(count);
    let mut cells = Vec::with_capacity(count);
    for &voxel in grid.voxels().elements() {
        seeds.push(voxel + FRAG_UNIT * 0.5);
        cells.push(grid.quantize_point(voxel));
    }

    let prev_head = mesh_stage.metadata().len();
    let voronoi = cubic_voronoi(
        &seeds,
        cubic_area,
        glam::Vec3::splat(FRAG_UNIT),
        MAX_SHIFT,
        SEEK_RANGE,
        mesh_stage,
    );

    let mut mapping = FxSpatialHash::with_capacity(SpatialResolution::new(FRAG_UNIT), cells.len());
    cells.drain(..).enumerate().for_each(|(i, cell)| {
        let mesh_id = unsafe { ethel::mesh::Id::from_value((i + prev_head) as u32) };

        let meta = voronoi.stager.metadata()[mesh_id.as_int() as usize];
        let vert_offset = meta.vert_offset as usize;
        let vert_end = vert_offset + meta.vert_length as usize;
        let vertices = &voronoi.stager.vertex_storage()[vert_offset..vert_end];
        let volume = approximate_volume_conservative(vertices);
        volumes.set(mesh_id, volume);

        mapping.put(cell, mesh_id);
    });

    FragmentGroup {
        cubic_area,
        voronoi,
        mapping,
    }
}
