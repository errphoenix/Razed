pub mod graphics;
pub mod pass;
pub mod shader_commons;

use std::sync::atomic::Ordering;

use ethel::{
    render::{Resolution, command::DrawGroups},
    state::camera::ViewPoint,
};
use gui::{
    render::UiDrawPassCtx,
    text::{GlyphAtlasTexture, GlyphRaster},
};
use janus::{
    context::DeltaTime,
    sync::TriCell,
    texture::{ImageFormat, ImageType, MipLevels, TextureFiltering},
};
use rendrs::pipeline::{Pass, RenderPool, RenderTarget, RenderTargetDescriptor, RenderTargetId};

#[cfg(feature = "devmode")]
use crate::render::pass::debug_lines_draw::DebugLinesData;
use crate::{
    assets,
    data::{FrameDataBuffers, LayoutFragmentData},
    render::graphics::Materials,
};

#[allow(unused)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderGroup {
    Generic,
    Fragment,
    Debris,
    LatticeDebug,
}

impl std::fmt::Display for RenderGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl DrawGroups for RenderGroup {
    fn as_str(&self) -> &'static str {
        match self {
            RenderGroup::Generic => "generic",
            RenderGroup::Fragment => "fragments",
            RenderGroup::Debris => "debris_n_rubber",
            RenderGroup::LatticeDebug => "lattice_debug",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RenderTargetHandles {
    base: RenderTargetId,
}
impl RenderTargetHandles {
    pub fn base(&self) -> RenderTargetId {
        self.base
    }
}

#[derive(Debug)]
pub struct RenderPipeline {
    cage_rotation_compute_pass: pass::CageRotateComputePass,
    fd_preprocess_pass: pass::FdPreprocessComputePass,
    fragments_draw_pass: pass::FragmentsDrawPass,
    debris_draw_pass: pass::DebrisDrawPass,
    debug_lattice_draw_pass: pass::DebugLatticeDrawPass,
    debug_cage_draw_pass: pass::DebugCageDrawPass,
    interface_draw_pass: gui::render::UiDrawPass,

    #[cfg(feature = "devmode")]
    debug_lines_draw_pass: pass::DebugLinesDrawPass,
}
impl RenderPipeline {
    fn revalidate(&mut self, render_pool: &RenderPool) {
        self.fragments_draw_pass.revalidate(render_pool);
        self.debris_draw_pass.revalidate(render_pool);
        self.debug_lattice_draw_pass.revalidate(render_pool);
        self.debug_cage_draw_pass.revalidate(render_pool);
        self.interface_draw_pass.revalidate(render_pool);

        #[cfg(feature = "devmode")]
        self.debug_lines_draw_pass.revalidate(render_pool);
    }
}

#[derive(Debug, Default)]
pub struct RenderShaders {
    lattice: pass::ShaderDebugLattice,
    fragments: pass::ShaderFragment,
    debris: pass::ShaderDebris,
    cage: pass::debug_cage_draw::ShaderDebugCage,
    interface: gui::render::ShaderUiBasic,
    cage_rotate: pass::ComputeShaderCageRotate,
    fd_preprocess: pass::ComputeShaderProcessCommand,

    #[cfg(feature = "devmode")]
    lines: pass::ShaderDebugLines,
}

#[derive(Debug, Default)]
pub struct Renderer {
    last_frame_render: DeltaTime,

    materials: Materials,

    // safe to unwrap during rendering
    pipeline: Option<RenderPipeline>,

    // safe to unwrap during rendering
    target_handles: Option<RenderTargetHandles>,
    render_pool: RenderPool,
    shaders: RenderShaders,

    #[cfg(feature = "devmode")]
    lines_debug_buffer: DebugLinesData,

    pub glyph_atlas_texture: GlyphAtlasTexture,
    pub glyph_pipe: Option<crossbeam::channel::Receiver<GlyphRaster>>,

    pub textures_master_registry: assets::TextureRegistry,
}
impl ethel::RenderHandler<FrameDataBuffers> for Renderer {
    fn pre_frame(
        &mut self,
        screen: &mut janus::sync::Mirror<ethel::render::ScreenSpace>,
        view: &janus::sync::TriCell<ethel::state::camera::ViewPoint>,
        delta: janus::context::DeltaTime,
    ) {
        self.last_frame_render = delta;

        {
            let last_resolution = screen.resolution();
            screen.sync().unwrap();

            let resolution = screen.resolution();
            if resolution.is_changed()
                && last_resolution.width != resolution.width
                && last_resolution.height != resolution.height
            {
                let render_pool = &mut self.render_pool;
                render_pool.revalidate_targets(resolution);

                self.pipeline.as_mut().unwrap().revalidate(render_pool);
            }
        }

        let view_mat = view.into_mat4().inverse();
        let proj = screen.projection();
        let ortho_proj = screen.orto_projection();
        let cam_position = view.position;
        let cam_forward = view.forward();

        #[cfg(feature = "devmode")]
        self.setup_debug_lines(view);

        let RenderShaders {
            lattice,
            fragments: frags,
            debris,
            cage,
            interface,
            #[cfg(feature = "devmode")]
            lines,
            ..
        } = &self.shaders;

        interface.bind();
        interface.uniform_projection_mat4v([*ortho_proj]);

        lattice.bind();
        lattice.uniform_projection_mat4v([*proj]);
        lattice.uniform_view_mat4v([view_mat]);

        cage.bind();
        cage.uniform_projection_mat4v([*proj]);
        cage.uniform_view_mat4v([view_mat]);

        #[cfg(feature = "devmode")]
        {
            lines.bind();
            lines.uniform_projection_mat4v([*proj]);
            lines.uniform_view_mat4v([view_mat]);
        }

        debris.bind();
        debris.uniform_camera_forward_vec3v([cam_forward]);
        debris.uniform_projection_mat4v([*proj]);
        debris.uniform_view_mat4v([view_mat]);

        frags.bind();
        frags.uniform_camera_forward_vec3v([cam_forward]);
        frags.uniform_camera_position_vec3v([cam_position]);
        frags.uniform_projection_mat4v([*proj]);
        frags.uniform_view_mat4v([view_mat]);

        // copy requested glyphs to atlas
        {
            if let Some(pipe) = &self.glyph_pipe {
                while let Ok(raster) = pipe.try_recv() {
                    self.glyph_atlas_texture.copy_glyph(raster);
                }
            }
        }
    }

    fn render_frame(
        &self,
        frame_data: &FrameDataBuffers,
        section: ethel::render::buffer::StorageSection,
    ) {
        let _ = frame_data
            .render_frame_last_duration
            .set_and_advance(self.last_frame_render);

        let render_pool = &self.render_pool;
        let cage_count = frame_data.cage_points_count.load(Ordering::Acquire);

        // cage extract rotations compute pass
        {
            use pass::cage_rotate_compute::CageRotateComputeCtx;

            let ctx = CageRotateComputeCtx {
                total_cage_count: cage_count,
                cage_data: &frame_data.cages,
            };

            self.shaders.cage_rotate.bind();
            self.shaders
                .cage_rotate
                .uniform_total_cage_count_uintv([cage_count]);

            self.pipeline()
                .cage_rotation_compute_pass
                .execute(section, render_pool, &ctx);
        }
        // there is no barrier here: an ssbo barrier is set after
        // fd_preprocess, which does not depend on this pass

        unsafe {
            janus::gl::Clear(janus::gl::COLOR_BUFFER_BIT | janus::gl::DEPTH_BUFFER_BIT);
        }

        // fragments & debris (preprocess + draw)
        {
            let cages_buf = &frame_data.cages;
            let frags_buf = &frame_data.fragments;
            let frags_cmd = &frame_data.fragment_commands;

            let debris_buf = &frame_data.debris;
            let debris_cmd = &frame_data.debris_commands;

            // command precompute pass
            {
                use pass::fd_preprocess::{FdPreprocessCtx, FdPreprocessTarget};

                // fragments
                let mut ctx = FdPreprocessCtx {
                    target: FdPreprocessTarget::Fragments,
                    fragment_commands: frags_cmd,
                    fragment_data: frags_buf,
                    debris_commands: debris_cmd,
                    debris_data: debris_buf,
                };
                self.pipeline()
                    .fd_preprocess_pass
                    .execute(section, render_pool, &ctx);

                // debris
                ctx.target = FdPreprocessTarget::Debris;
                self.pipeline()
                    .fd_preprocess_pass
                    .execute(section, render_pool, &ctx);
            }

            janus::gl::barrier_shader_storage();
            janus::gl::barrier_commands();

            // fragments draw pass
            {
                let ctx = pass::FragmentsDrawCtx {
                    cages_data: cages_buf,
                    fragments_data: frags_buf,
                    fragments_commands: frags_cmd,
                    material_registry: self.materials.locations(),
                };
                self.pipeline()
                    .fragments_draw_pass
                    .execute(section, render_pool, &ctx);
            }
            // debris draw pass
            {
                let ctx = pass::DebrisDrawCtx {
                    debris_data: debris_buf,
                    debris_commands: debris_cmd,
                };
                self.pipeline()
                    .debris_draw_pass
                    .execute(section, render_pool, &ctx);
            }
        }
        // cage draw pass
        {
            let ctx = pass::DebugCageDrawCtx {
                fragments_data: &frame_data.fragments,
                cage_data: &frame_data.cages,
                point_size: 2.5,
                cage_total_count: cage_count * 8,
            };
            self.pipeline()
                .debug_cage_draw_pass
                .execute(section, render_pool, &ctx);
        }

        // interface draw pass
        {
            let ctx = UiDrawPassCtx {
                ui_shader: &self.shaders.interface,
                data: &frame_data.interface_storage,
                commands: &frame_data.interface_commands,
            };
            self.pipeline()
                .interface_draw_pass
                .execute(section, render_pool, &ctx);
        }

        // debug lattice draw pass
        {
            let count = frame_data.lattice_constraint_count.load(Ordering::Acquire);
            let ctx = pass::DebugLatticeDrawCtx {
                lattice_data: &frame_data.lattice_debug,
                constraints_count: count as i32,
            };
            self.pipeline()
                .debug_lattice_draw_pass
                .execute(section, render_pool, &ctx);
        }

        // debug lines draw pass
        #[cfg(feature = "devmode")]
        {
            use crate::render::pass::debug_lines_draw::DebugLinesDrawCtx;
            let ctx = DebugLinesDrawCtx {
                lines_data: &frame_data.lines_debug,
                lines_buffer: &self.lines_debug_buffer,
            };
            self.pipeline()
                .debug_lines_draw_pass
                .execute(section, render_pool, &ctx);
        }
    }

    fn init_resources(&mut self, resolution: Resolution) {
        self.initialize_shaders();
        self.initialize_render_targets(resolution);

        let texture_assets = &mut self.textures_master_registry;
        self.materials.initialize(texture_assets);

        let dev_materials = &self.materials.groups().dev;

        self.pipeline = Some(RenderPipeline {
            fd_preprocess_pass: pass::fd_preprocess::pass(&self.shaders.fd_preprocess),
            fragments_draw_pass: pass::fragments_draw::pass(&self.shaders.fragments, dev_materials),
            debris_draw_pass: pass::debris_draw::pass(&self.shaders.debris),
            debug_lattice_draw_pass: pass::debug_lattice_draw::pass(&self.shaders.lattice),
            cage_rotation_compute_pass: pass::cage_rotate_compute::pass(&self.shaders.cage_rotate),
            debug_cage_draw_pass: pass::debug_cage_draw::pass(&self.shaders.cage),
            interface_draw_pass: gui::render::pass(&self.shaders.interface),

            #[cfg(feature = "devmode")]
            debug_lines_draw_pass: pass::debug_lines_draw::pass(&self.shaders.lines),
        });
    }
}
impl Renderer {
    fn pipeline(&self) -> &RenderPipeline {
        self.pipeline
            .as_ref()
            .expect("render pipeline must be present after resource initialization")
    }

    fn initialize_shaders(&mut self) {
        self.shaders.lattice = pass::ShaderDebugLattice::new_compiled();
        self.shaders.fragments = pass::ShaderFragment::new_compiled_variant(
            pass::ShaderFragmentVariants::WindowedAttenuation,
        );
        self.shaders.debris = pass::ShaderDebris::new_compiled();
        self.shaders.cage_rotate = pass::ComputeShaderCageRotate::new_compiled();
        self.shaders.cage = pass::debug_cage_draw::ShaderDebugCage::new_compiled();
        self.shaders.interface = gui::render::ShaderUiBasic::new_compiled();
        self.shaders.fd_preprocess = pass::ComputeShaderProcessCommand::new_compiled();

        #[cfg(feature = "devmode")]
        {
            self.shaders.lines = pass::ShaderDebugLines::new_compiled();
        }

        let sampler_uniforms = std::array::from_fn(|i| i as i32);
        self.shaders.interface.bind();
        self.shaders
            .interface
            .uniform_texture_map_sampler2Dv(sampler_uniforms);
    }

    fn initialize_render_targets(&mut self, resolution: Resolution) {
        let base = self.render_pool.add(RenderTarget::new(
            "base",
            RenderTargetDescriptor::new(
                ImageFormat::Rgb,
                ImageType::Bits8,
                TextureFiltering::Nearest,
                MipLevels::default(),
                1.0,
            ),
            resolution,
        ));

        self.target_handles = Some(RenderTargetHandles { base });
    }

    fn setup_debug_lines(&mut self, view: &TriCell<ViewPoint>) {
        // world axis indicator
        #[cfg(feature = "devmode")]
        {
            self.lines_debug_buffer.clear();

            const OFFSET: f32 = 1.0;
            let o = view.position + view.forward() * OFFSET;

            const R: glam::Vec4 = glam::vec4(1.0, 0.0, 0.0, 1.0);
            const G: glam::Vec4 = glam::vec4(0.0, 1.0, 0.0, 1.0);
            const B: glam::Vec4 = glam::vec4(0.0, 0.0, 1.0, 1.0);

            const AXIS_SIZE: f32 = 0.075;
            let x = o + glam::Vec3::X * AXIS_SIZE;
            let y = o + glam::Vec3::Y * AXIS_SIZE;
            let z = o + glam::Vec3::Z * AXIS_SIZE;

            self.lines_debug_buffer.add(o, R);
            self.lines_debug_buffer.add(x, R);

            self.lines_debug_buffer.add(o, G);
            self.lines_debug_buffer.add(y, G);

            self.lines_debug_buffer.add(o, B);
            self.lines_debug_buffer.add(z, B);
        }

        // prepare debris spatial hash grid
        #[cfg(feature = "devmode")]
        {
            use ethel::state::data::hash::SpatialResolution;

            const COLOR: glam::Vec4 = glam::vec4(1.0, 0.2, 1.0, 0.1);
            const RANGE: f32 = 20.0;
            const RANGE_CELLS: i32 = (RANGE / RESOLUTION.get()) as i32;

            const RESOLUTION: SpatialResolution = crate::structure::debris::HASH_RESOLUTION;

            let camera = RESOLUTION.encode_point(view.position);
            let f_camera = RESOLUTION.approx_point(camera);

            for x in -RANGE_CELLS..RANGE_CELLS {
                for y in -RANGE_CELLS..RANGE_CELLS {
                    for z in -RANGE_CELLS..RANGE_CELLS {
                        let po = glam::vec3(x as f32, y as f32, z as f32) * RESOLUTION.get();
                        let px = glam::vec3((x + 1) as f32, y as f32, z as f32) * RESOLUTION.get();
                        let py = glam::vec3(x as f32, (y + 1) as f32, z as f32) * RESOLUTION.get();
                        let pz = glam::vec3(x as f32, y as f32, (z + 1) as f32) * RESOLUTION.get();

                        let po = po + f_camera;
                        let px = px + f_camera;
                        let py = py + f_camera;
                        let pz = pz + f_camera;

                        self.lines_debug_buffer.add_position(po);
                        self.lines_debug_buffer.add_position(px);

                        self.lines_debug_buffer.add_position(po);
                        self.lines_debug_buffer.add_position(py);

                        self.lines_debug_buffer.add_position(po);
                        self.lines_debug_buffer.add_position(pz);
                    }
                }
            }

            self.lines_debug_buffer.set_color_fallback(COLOR);
        }
    }
}
