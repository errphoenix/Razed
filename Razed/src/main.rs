use ethel::{
    StartupHandler,
    mesh::{MeshStaging, Metadata, Vertex},
    state::data::hash::{FxSpatialHash, SpatialResolution},
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

    let fragment_mesh_mapping = {
        let mut mesh_stage = ethel::mesh::MeshStaging::new();
        let _debug_cube = mesh_stage.stage(&MESH_UNIT_CUBE);

        let group = generate_fragment_meshes(glam::Vec3::splat(3.0), mesh_stage);
        let mesh_stage = group.voronoi.stager;

        println!("{:?}", mesh_stage.metadata());

        start_handler.with_mesh_data(mesh_stage);
        start_handler.with_mesh_layout(data::LayoutMeshStorage::create());

        group.mapping
    };

    start_handler.with_gl_state(|| unsafe {
        janus::gl::ClipControl(janus::gl::LOWER_LEFT, janus::gl::ZERO_TO_ONE);
        janus::gl::DepthFunc(janus::gl::GREATER);
        janus::gl::ClearDepth(0.0);
        janus::gl::Enable(janus::gl::DEPTH_TEST);
    });

    let ctx = janus::context::Context::new(
        |state: &mut State, renderer: &mut Renderer| {
            state.handler_init_callback(|handle| {
                handle.fragment_mesh_mapping = fragment_mesh_mapping;
            });
            start_handler.init(state, renderer)
        },
        input_dispatch,
        DISPLAY_PARAMS,
    );

    janus::run(ctx);
}

#[derive(Debug)]
struct FragmentGroup {
    pub cubic_area: glam::Vec3,
    pub voronoi: CubeVoronoi,
    pub mapping: FxSpatialHash<ethel::mesh::Id>,
}

fn generate_fragment_meshes(cubic_area: glam::Vec3, mesh_stage: MeshStaging) -> FragmentGroup {
    const FRAG_UNIT: f32 = 1.0;
    const MAX_SHIFT: f32 = 0.5;
    const SEEK_RANGE: f32 = 3.0;

    let mut grid = voxel_grid(cubic_area.x, cubic_area.y, cubic_area.z, FRAG_UNIT);

    grid.repopulate_defaults();
    let count = grid.count();
    println!("seeds: {count}");

    // CubicVoronoiGenerator is guaranteed to process the seeds and meshes by
    // the same order they are in the given seeds collection.
    // we can use this to map the meshes by the generator to a local cell
    // coordinate.
    let mut seeds = Vec::with_capacity(count);
    let mut cells = Vec::with_capacity(count);
    for &voxel in grid.voxels().elements() {
        seeds.push(voxel);
        cells.push(grid.quantize_point(voxel));
    }

    let prev_head = mesh_stage.metadata().len();
    let voronoi = procedural::cubic_voronoi(&seeds, cubic_area, MAX_SHIFT, SEEK_RANGE, mesh_stage);

    let mut mapping = FxSpatialHash::with_capacity(SpatialResolution::new(FRAG_UNIT), cells.len());
    cells.drain(..).enumerate().for_each(|(i, cell)| {
        let mesh_id = unsafe { ethel::mesh::Id::from_value((i + prev_head) as u32) };
        println!("{cell:?}");
        mapping.put(cell, mesh_id);
    });

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
