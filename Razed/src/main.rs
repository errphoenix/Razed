use ethel::{
    StartupHandler,
    assets::Handle,
    mesh::MeshStaging,
    state::data::hash::{FxSpatialHash, SpatialResolution},
};
use gui::text::{GlyphAtlas, GlyphAtlasTexture};
use janus::{context::Setup, window::DisplayParameters};

use crate::{
    data::FrameDataBuffers,
    procedural::{CubeVoronoi, voxel_grid},
    render::RenderGroup,
};

mod assets;
mod data;
mod procedural;
mod render;
mod state;
mod structure;
mod ui;

const DISPLAY_PARAMS: DisplayParameters = DisplayParameters::fullscreen("Razed");

type State = ethel::state::State<FrameDataBuffers, state::State, RenderGroup>;
type Renderer = ethel::render::Renderer<FrameDataBuffers, render::Renderer>;

fn main() {
    tracing_subscriber::FmtSubscriber::builder().init();

    let (input_system, input_dispatch) = janus::input::stream();
    input_system.surface_options().update(|mut flags| {
        flags.set_window_vsync(true);
        flags.set_dirty(true)
    });

    let mut start_handler = StartupHandler::new(input_system, || FrameDataBuffers::new());

    let mut textures_master_registry = assets::TextureRegistryBuilder::build();
    let textures_metadata_registry = textures_master_registry.create_metadata_registry();
    let texture_pipe = textures_master_registry.command_pipe();

    let fragment_mesh_mapping = {
        let mesh_stage = ethel::mesh::MeshStaging::new();
        let group = generate_fragment_meshes(glam::Vec3::splat(3.0), mesh_stage);
        let mesh_stage = group.voronoi.stager;

        start_handler.with_mesh_data(mesh_stage);
        start_handler.with_mesh_layout(data::LayoutMeshStorage::create());

        group.mapping
    };

    start_handler.with_gl_state(|| unsafe {
        janus::gl::PixelStorei(janus::gl::UNPACK_ALIGNMENT, 1);

        janus::gl::ClipControl(janus::gl::LOWER_LEFT, janus::gl::ZERO_TO_ONE);
        janus::gl::DepthFunc(render::DEFAULT_DEPTH_FUNC);
        janus::gl::ClearDepth(0.0);
        janus::gl::Enable(janus::gl::DEPTH_TEST);

        janus::gl::BlendFunc(janus::gl::SRC_ALPHA, janus::gl::ONE_MINUS_SRC_ALPHA);
        janus::gl::Enable(janus::gl::BLEND);
    });

    let ctx = janus::context::Context::new(
        |state: &mut State, renderer: &mut Renderer| {
            const GLYPH_ATLAS_SIZE: u32 = 2048;

            let (raster_tx, raster_rx) = crossbeam::channel::unbounded();

            renderer.handler_init_callback(|handle| {
                handle.textures_master_registry = textures_master_registry;

                let texture = handle
                    .glyph_atlas_texture
                    .create_atlas_texture(GLYPH_ATLAS_SIZE as i32);
                // special gpu-only asset which must never be touched
                let glyph_asset_handle = Handle::from_gpu_resource(
                    GlyphAtlasTexture::resource_id(),
                    texture,
                    &handle.textures_master_registry,
                );
                handle
                    .textures_master_registry
                    .add_handle(glyph_asset_handle);

                handle.glyph_pipe = Some(raster_rx);
            });
            state.handler_init_callback(|handle| {
                handle.frag_meshmap = fragment_mesh_mapping;

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

#[derive(Debug)]
struct FragmentGroup {
    #[allow(unused)]
    pub cubic_area: glam::Vec3,
    pub voronoi: CubeVoronoi,
    pub mapping: FxSpatialHash<ethel::mesh::Id>,
}

fn generate_fragment_meshes(cubic_area: glam::Vec3, mesh_stage: MeshStaging) -> FragmentGroup {
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
    let voronoi = procedural::cubic_voronoi(
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
        mapping.put(cell, mesh_id);
    });

    FragmentGroup {
        cubic_area,
        voronoi,
        mapping,
    }
}
