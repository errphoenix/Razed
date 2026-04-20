use ethel::{
    StartupHandler,
    mesh::{MeshStaging, Metadata, Vertex},
    state::data::hash::Cell,
};
use janus::{context::Setup, window::DisplayParameters};

use crate::{
    data::FrameDataBuffers,
    procedural::{CubeVoronoi, voxel_grid},
};

mod data;
mod procedural;
mod render;
mod state;
mod structure;

const DISPLAY_PARAMS: DisplayParameters = DisplayParameters::fullscreen("Razed");

type State = ethel::state::State<FrameDataBuffers, state::State>;
type Renderer = ethel::render::Renderer<FrameDataBuffers, render::Renderer>;

fn main() {
    tracing_subscriber::FmtSubscriber::builder().init();

    let (input_system, input_dispatch) = janus::input::stream();
    let mut start_handler = StartupHandler::new(input_system, || FrameDataBuffers::new());

    {
        let mut mesh_stage = ethel::mesh::MeshStaging::new();
        let _debug_cube = mesh_stage.stage(&MESH_UNIT_CUBE);

        start_handler.with_mesh_data(mesh_stage);
        start_handler.with_mesh_layout(data::LayoutMeshStorage::create());
    }

    start_handler.with_gl_state(|| unsafe {
        janus::gl::ClipControl(janus::gl::LOWER_LEFT, janus::gl::ZERO_TO_ONE);
        janus::gl::DepthFunc(janus::gl::GREATER);
        janus::gl::ClearDepth(0.0);
        janus::gl::Enable(janus::gl::DEPTH_TEST);
    });

    let ctx = janus::context::Context::new(
        |state: &mut State, renderer: &mut Renderer| start_handler.init(state, renderer),
        input_dispatch,
        DISPLAY_PARAMS,
    );

    janus::run(ctx);
}

#[derive(Debug)]
struct FragmentGroup {
    pub cubic_area: glam::Vec3,
    pub voronoi: CubeVoronoi,
    pub mapping: Vec<(Cell, Metadata)>,
}

fn generate_fragment_meshes(cubic_area: glam::Vec3, mesh_stage: MeshStaging) -> FragmentGroup {
    const FRAG_UNIT: f32 = 1.0;
    const MAX_SHIFT: f32 = 0.325;
    const SEEK_RANGE: f32 = 1.25;

    let mut grid = voxel_grid(cubic_area.x, cubic_area.y, cubic_area.z, FRAG_UNIT);

    grid.repopulate();
    let count = grid.count();

    // CubicVoronoiGenerator is guaranteed to process the seeds and meshes by
    // the same order they are in the given seeds collection.
    // we can use this to map the meshes by the generator to a local cell
    // coordinate.
    let mut seeds = Vec::with_capacity(count);
    let mut cells = Vec::with_capacity(count);
    for &voxel in grid.voxels().elements() {
        seeds.push(grid.point_from_id(voxel));
        cells.push(grid.cell_from_id(voxel));
    }

    let prev_head = mesh_stage.metadata().head() as usize;
    let voronoi = procedural::cubic_voronoi(&seeds, cubic_area, MAX_SHIFT, SEEK_RANGE, mesh_stage);
    let metadata = voronoi.stager.metadata();

    let mapping = cells
        .drain(..)
        .enumerate()
        .map(|(i, cell)| {
            let j = i + prev_head;
            let meta = metadata[j];
            (cell, meta)
        })
        .collect::<Vec<_>>();

    FragmentGroup {
        cubic_area,
        voronoi,
        mapping,
    }
}

const MESH_UNIT_CUBE: [Vertex; 36] = [
    // Z+
    Vertex {
        position: [0.5, -0.5, 0.5, 1.0],
        normal: [0.0, 0.0, 1.0, 1.0],
    },
    Vertex {
        position: [0.5, 0.5, 0.5, 1.0],
        normal: [0.0, 0.0, 1.0, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5, 0.5, 1.0],
        normal: [0.0, 0.0, 1.0, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5, 0.5, 1.0],
        normal: [0.0, 0.0, 1.0, 1.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.5, 1.0],
        normal: [0.0, 0.0, 1.0, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.5, 1.0],
        normal: [0.0, 0.0, 1.0, 1.0],
    },
    // Z-
    Vertex {
        position: [0.5, -0.5, -0.5, 1.0],
        normal: [0.0, 0.0, -1.0, 1.0],
    },
    Vertex {
        position: [0.5, 0.5, -0.5, 1.0],
        normal: [0.0, 0.0, -1.0, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5, -0.5, 1.0],
        normal: [0.0, 0.0, -1.0, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5, -0.5, 1.0],
        normal: [0.0, 0.0, -1.0, 1.0],
    },
    Vertex {
        position: [-0.5, -0.5, -0.5, 1.0],
        normal: [0.0, 0.0, -1.0, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, -0.5, 1.0],
        normal: [0.0, 0.0, -1.0, 1.0],
    },
    // Y+
    Vertex {
        position: [-0.5, 0.5, 0.5, 1.0],
        normal: [0.0, 1.0, 0.0, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5, -0.5, 1.0],
        normal: [0.0, 1.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.5, 0.5, -0.5, 1.0],
        normal: [0.0, 1.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.5, 0.5, -0.5, 1.0],
        normal: [0.0, 1.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.5, 0.5, 0.5, 1.0],
        normal: [0.0, 1.0, 0.0, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5, 0.5, 1.0],
        normal: [0.0, 1.0, 0.0, 1.0],
    },
    // Y-
    Vertex {
        position: [-0.5, -0.5, 0.5, 1.0],
        normal: [0.0, -1.0, 0.0, 1.0],
    },
    Vertex {
        position: [-0.5, -0.5, -0.5, 1.0],
        normal: [0.0, -1.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, -0.5, 1.0],
        normal: [0.0, -1.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, -0.5, 1.0],
        normal: [0.0, -1.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.5, 1.0],
        normal: [0.0, -1.0, 0.0, 1.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.5, 1.0],
        normal: [0.0, -1.0, 0.0, 1.0],
    },
    // X+
    Vertex {
        position: [0.5, 0.5, -0.5, 1.0],
        normal: [1.0, 0.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, -0.5, 1.0],
        normal: [1.0, 0.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.5, 1.0],
        normal: [1.0, 0.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.5, 1.0],
        normal: [1.0, 0.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.5, 0.5, 0.5, 1.0],
        normal: [1.0, 0.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.5, 0.5, -0.5, 1.0],
        normal: [1.0, 0.0, 0.0, 1.0],
    },
    // X-
    Vertex {
        position: [-0.5, 0.5, -0.5, 1.0],
        normal: [-1.0, 0.0, 0.0, 1.0],
    },
    Vertex {
        position: [-0.5, -0.5, -0.5, 1.0],
        normal: [-1.0, 0.0, 0.0, 1.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.5, 1.0],
        normal: [-1.0, 0.0, 0.0, 1.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.5, 1.0],
        normal: [-1.0, 0.0, 0.0, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5, 0.5, 1.0],
        normal: [-1.0, 0.0, 0.0, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5, -0.5, 1.0],
        normal: [-1.0, 0.0, 0.0, 1.0],
    },
];
