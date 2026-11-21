use ethel::{AssetHandle, StartupHandler};
use gui::text::{GlyphAtlas, GlyphAtlasTexture};
use janus::{DisplayParameters, Setup};
use rendrs::GeometryBank;

use razed::{StartupMeshData, data::FrameDataBuffers, render::RenderGroup};
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};

const DISPLAY_PARAMS: DisplayParameters = DisplayParameters::fullscreen("Razed");
const LOG_FILENAME: &str = "../tracing.log";

type State = ethel::state::State<FrameDataBuffers, razed::simulation::State, RenderGroup>;
type Renderer = ethel::render::Renderer<FrameDataBuffers, razed::render::Renderer>;

fn main() {
    {
        let l_file = tracing_subscriber::fmt::layer()
            .with_ansi(true)
            .with_writer(
                std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .open(LOG_FILENAME)
                    .unwrap(),
            );
        let l_stdout = tracing_subscriber::fmt::layer().with_ansi(true);

        let filter = tracing_subscriber::filter::filter_fn(|#[allow(unused)] meta| {
            #[cfg(not(feature = "log-debug"))]
            {
                *meta.level() != tracing::Level::DEBUG && *meta.level() != tracing::Level::TRACE
            }
            #[cfg(feature = "log-debug")]
            {
                *meta.level() != tracing::Level::TRACE
            }
        });

        tracing_subscriber::registry()
            .with(l_file.with_filter(filter.clone()))
            .with(l_stdout.with_filter(filter))
            .init();
    }

    let (input_system, input_dispatch) = janus::input::stream();
    input_system.surface_options().update(|mut flags| {
        flags.set_window_vsync(true);
        flags.set_dirty(true)
    });

    let mut start_handler = StartupHandler::new(input_system, || FrameDataBuffers::new());

    let mut textures_master_registry = razed::assets::TextureRegistryBuilder::build();
    let textures_metadata_registry = textures_master_registry.create_metadata_registry();
    let texture_pipe = textures_master_registry.command_pipe();

    let StartupMeshData {
        stager: mesh_stager,
        fragment_mesh_volumes,
        fragment_mesh_mapping,
        common_meta: mesh_common_meta,
    } = razed::setup_mesh_data();

    start_handler.with_mesh_data(mesh_stager);
    start_handler.with_mesh_layouts(
        razed::data::LayoutMeshStorageStatic::create(),
        razed::data::LayoutMeshStorageTris::create(),
    );

    start_handler.with_gl_state(|| unsafe {
        janus::gl::PixelStorei(janus::gl::UNPACK_ALIGNMENT, 1);

        janus::gl::ClipControl(janus::gl::LOWER_LEFT, janus::gl::ZERO_TO_ONE);
        janus::gl::DepthFunc(razed::render::DEFAULT_DEPTH_FUNC);
        janus::gl::ClearDepth(0.0);
        janus::gl::Enable(janus::gl::DEPTH_TEST);

        janus::gl::Enable(janus::gl::CULL_FACE);
        janus::gl::BlendFunc(janus::gl::SRC_ALPHA, janus::gl::ONE_MINUS_SRC_ALPHA);
        janus::gl::Enable(janus::gl::BLEND);

        janus::gl::Enable(janus::gl::TEXTURE_CUBE_MAP_SEAMLESS);
    });

    let ctx = janus::context::Context::new(
        |state: &mut State, renderer: &mut Renderer| {
            const GLYPH_ATLAS_SIZE: u32 = 2048;

            let (raster_tx, raster_rx) = crossbeam::channel::unbounded();

            renderer.handler_init_callback(|handle| {
                handle.common_mesh_data = mesh_common_meta;
                handle.geometry_bank = GeometryBank::new();

                handle.textures_master_registry = textures_master_registry;
                handle.glyph_pipe = Some(raster_rx);

                // register glyph atlas texture as gpu-only asset entry
                {
                    let texture = handle
                        .glyph_atlas_texture
                        .create_atlas_texture(GLYPH_ATLAS_SIZE as i32);
                    let glyph_asset_handle = AssetHandle::from_gpu_resource(
                        GlyphAtlasTexture::resource_id(),
                        texture,
                        &handle.textures_master_registry,
                    );
                    handle
                        .textures_master_registry
                        .add_handle(glyph_asset_handle);
                }
            });
            state.handler_init_callback(|handle| {
                handle.frag_meshmap = fragment_mesh_mapping;
                handle.frag_volumes = fragment_mesh_volumes;

                handle.textures_metadata_registry = textures_metadata_registry;
                handle.texture_registry_pipe.set_pipe(texture_pipe);

                handle.glyph_atlas = GlyphAtlas::new(GLYPH_ATLAS_SIZE);
                handle.ui_system_mut().bind_system_fonts();

                handle.glyph_pipe = Some(raster_tx);
            });
            start_handler.init(state, renderer)
        },
        input_dispatch,
        DISPLAY_PARAMS,
    );

    janus::run(ctx);
}
