use ethel::{StartupHandler, mesh::Vertex};
use janus::{context::Setup, window::DisplayParameters};

use crate::data::FrameDataBuffers;

mod data;
mod render;
mod state;

const DISPLAY_PARAMS: DisplayParameters = DisplayParameters::fullscreen("Razed");

type State = ethel::state::State<FrameDataBuffers, state::State>;
type Renderer = ethel::render::Renderer<FrameDataBuffers, render::Renderer>;

fn main() {
    tracing_subscriber::FmtSubscriber::builder().init();

    let (input_system, input_dispatch) = janus::input::stream();
    let mut start_handler = StartupHandler::new(input_system, || FrameDataBuffers::new());
    {
        let mut mesh_stage = ethel::mesh::MeshStaging::new();
        let triangle = [
            Vertex {
                position: [1.0, 0.0, 0.0, 1.0],
                normal: [0.33, -0.33, 0.33, 1.0],
            },
            Vertex {
                position: [0.0, 1.0, 0.0, 1.0],
                normal: [0.0, 0.5, 0.5, 1.0],
            },
            Vertex {
                position: [-1.0, 0.0, 0.0, 1.0],
                normal: [-0.33, -0.33, 0.33, 1.0],
            },
        ];
        let _triangle_id = mesh_stage.stage(&triangle);

        start_handler.with_mesh_data(mesh_stage);
        start_handler.with_mesh_layout(data::LayoutMeshStorage::create());
    }

    start_handler.with_gl_state(|| {});

    let ctx = janus::context::Context::new(
        |state: &mut State, renderer: &mut Renderer| start_handler.init(state, renderer),
        input_dispatch,
        DISPLAY_PARAMS,
    );

    janus::run(ctx);
}
