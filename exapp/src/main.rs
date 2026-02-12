use ethel::{StartupHandler, mesh::Vertex};
use janus::{context::Setup, window::DisplayParameters};

use crate::data::FrameDataBuffers;

mod data;
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
        let _triangle_id = mesh_stage.stage(&MESH_UNIT_CUBE);

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

const MESH_UNIT_CUBE: [Vertex; 36] = [
    // Z+
    Vertex {
        position: [0.5, -0.5, 0.5, 1.0],
        normal: [0.33, -0.33, 0.33, 1.0],
    },
    Vertex {
        position: [0.5, 0.5, 0.5, 1.0],
        normal: [0.33, 0.33, 0.33, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5, 0.5, 1.0],
        normal: [-0.33, 0.33, 0.33, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5, 0.5, 1.0],
        normal: [-0.33, 0.33, 0.33, 1.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.5, 1.0],
        normal: [-0.33, -0.33, 0.33, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.5, 1.0],
        normal: [0.33, -0.33, 0.33, 1.0],
    },
    // Z-
    Vertex {
        position: [0.5, -0.5, -0.5, 1.0],
        normal: [0.33, -0.33, -0.33, 1.0],
    },
    Vertex {
        position: [0.5, 0.5, -0.5, 1.0],
        normal: [0.33, 0.33, -0.33, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5, -0.5, 1.0],
        normal: [-0.33, 0.33, -0.33, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5, -0.5, 1.0],
        normal: [-0.33, 0.33, -0.33, 1.0],
    },
    Vertex {
        position: [-0.5, -0.5, -0.5, 1.0],
        normal: [-0.33, -0.33, -0.33, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, -0.5, 1.0],
        normal: [0.33, -0.33, -0.33, 1.0],
    },
    // Y+
    Vertex {
        position: [-0.5, 0.5, 0.5, 1.0],
        normal: [-0.33, 0.33, 0.33, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5, -0.5, 1.0],
        normal: [-0.33, 0.33, -0.33, 1.0],
    },
    Vertex {
        position: [0.5, 0.5, -0.5, 1.0],
        normal: [0.33, 0.33, -0.33, 1.0],
    },
    Vertex {
        position: [0.5, 0.5, -0.5, 1.0],
        normal: [0.33, 0.33, -0.33, 1.0],
    },
    Vertex {
        position: [0.5, 0.5, 0.5, 1.0],
        normal: [0.33, 0.33, 0.33, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5, 0.5, 1.0],
        normal: [-0.33, 0.33, 0.33, 1.0],
    },
    // Y-
    Vertex {
        position: [-0.5, -0.5, 0.5, 1.0],
        normal: [-0.33, -0.33, 0.33, 1.0],
    },
    Vertex {
        position: [-0.5, -0.5, -0.5, 1.0],
        normal: [-0.33, -0.33, -0.33, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, -0.5, 1.0],
        normal: [0.33, -0.33, -0.33, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, -0.5, 1.0],
        normal: [0.33, -0.33, -0.33, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.5, 1.0],
        normal: [0.33, -0.33, 0.33, 1.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.5, 1.0],
        normal: [-0.33, -0.33, 0.33, 1.0],
    },
    // X+
    Vertex {
        position: [0.5, 0.5, -0.5, 1.0],
        normal: [0.33, 0.33, -0.33, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, -0.5, 1.0],
        normal: [0.33, -0.33, -0.33, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.5, 1.0],
        normal: [0.33, -0.33, 0.33, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.5, 1.0],
        normal: [0.33, -0.33, 0.33, 1.0],
    },
    Vertex {
        position: [0.5, 0.5, 0.5, 1.0],
        normal: [0.33, 0.33, 0.33, 1.0],
    },
    Vertex {
        position: [0.5, 0.5, -0.5, 1.0],
        normal: [0.33, 0.33, -0.33, 1.0],
    },
    // X-
    Vertex {
        position: [-0.5, 0.5, -0.5, 1.0],
        normal: [-0.33, 0.33, -0.33, 1.0],
    },
    Vertex {
        position: [-0.5, -0.5, -0.5, 1.0],
        normal: [-0.33, -0.33, -0.33, 1.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.5, 1.0],
        normal: [-0.33, -0.33, 0.33, 1.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.5, 1.0],
        normal: [-0.33, -0.33, 0.33, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5, 0.5, 1.0],
        normal: [-0.33, 0.33, 0.33, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5, -0.5, 1.0],
        normal: [-0.33, 0.33, -0.33, 1.0],
    },
];
