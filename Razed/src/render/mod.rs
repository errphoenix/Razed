pub mod graphics;
pub mod pass;
pub mod shader_commons;

use std::sync::atomic::Ordering;

use ethel::{
    assets::{Handle, RawTexture},
    render::{Resolution, buffer::StorageSection, command::DrawGroups},
    state::camera::ViewPoint,
};
use gui::{
    render::UiDrawPassCtx,
    text::{GlyphAtlasTexture, GlyphRaster},
};
use janus::{
    StringHash,
    context::DeltaTime,
    sync::TriCell,
    texture::{
        ImageFormat, ImageType, MipLevels, Tex, Texture, TextureFiltering, TextureKind, TextureView,
    },
};
use rendrs::{
    graphics::{
        PixelResolution,
        reflection_filtering::{BSplineDownscaleCtx, ComputeShaderBSplineDownscale},
    },
    pipeline::{
        ImageAccessKind, ImageObject, ImageObjectTarget, OutputObject, Pass, RenderPool,
        RenderTarget, RenderTargetDescriptor, RenderTargetId, SamplerObject,
    },
};

#[cfg(feature = "devmode")]
use crate::render::pass::debug_lines_draw::DebugLinesData;
use crate::{assets, data::FrameDataBuffers, render::graphics::Materials};

pub const DEFAULT_DEPTH_FUNC: u32 = janus::gl::GREATER;

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
    hdr_base: RenderTargetId,
    base_depth: RenderTargetId,
    ldr_mapped: RenderTargetId,
}
impl RenderTargetHandles {
    pub const fn hdr_base(&self) -> RenderTargetId {
        self.hdr_base
    }

    pub const fn base_depth(&self) -> RenderTargetId {
        self.base_depth
    }

    pub const fn ldr_mapped(&self) -> RenderTargetId {
        self.ldr_mapped
    }
}

#[derive(Debug)]
pub struct PersistentSamplers {
    /// Citrus Orchard HDR
    dev_envmap_cubemap: SamplerObject,
}
impl PersistentSamplers {
    pub const fn dev_envmap_cube(&self) -> SamplerObject {
        self.dev_envmap_cubemap
    }
}

#[derive(Debug)]
pub struct RenderPipeline {
    cage_deform_compute_pass: pass::CageDeformComputePass,
    fd_preprocess_pass: pass::FdPreprocessComputePass,
    fragments_draw_pass: pass::FragmentsDrawPass,
    debris_draw_pass: pass::DebrisDrawPass,
    skybox_draw_pass: pass::SkyboxDrawPass,
    debug_lattice_draw_pass: pass::DebugLatticeDrawPass,
    debug_cage_draw_pass: pass::DebugCageDrawPass,
    interface_draw_pass: gui::render::UiDrawPass,

    tonemap_vfx_pass: pass::TonemapVfxPass,
    clear_pass: rendrs::ClearPass<3>,
    blit_pass: rendrs::BlitPass,

    #[cfg(feature = "devmode")]
    debug_lines_draw_pass: pass::DebugLinesDrawPass,
}
impl RenderPipeline {
    fn revalidate(&mut self, render_pool: &RenderPool) {
        self.fragments_draw_pass.revalidate(render_pool);
        self.debris_draw_pass.revalidate(render_pool);
        self.skybox_draw_pass.revalidate(render_pool);
        self.debug_lattice_draw_pass.revalidate(render_pool);
        self.debug_cage_draw_pass.revalidate(render_pool);
        self.interface_draw_pass.revalidate(render_pool);
        self.tonemap_vfx_pass.revalidate(render_pool);
        self.blit_pass.revalidate(render_pool);
        self.clear_pass.revalidate(render_pool);

        #[cfg(feature = "devmode")]
        self.debug_lines_draw_pass.revalidate(render_pool);
    }
}

#[derive(Debug, Default)]
pub struct RenderShaders {
    fragments: pass::ShaderFragment,
    debris: pass::ShaderDebris,

    skybox: pass::ShaderSkybox,

    lattice: pass::ShaderDebugLattice,
    cage_visual: pass::debug_cage_draw::ShaderDebugCage,

    interface: gui::render::ShaderUiBasic,

    cage_deform: pass::ComputeShaderCageDeform,
    fd_preprocess: pass::ComputeShaderProcessCommand,

    util_equirect_decode: pass::ComputeShaderEquirectDecode,

    vfx_tonemap: pass::ComputeShaderTonemap,

    #[cfg(feature = "devmode")]
    lines: pass::ShaderDebugLines,
}

#[derive(Debug, Default)]
pub struct Renderer {
    last_frame_render: DeltaTime,

    materials: Materials,

    // safe to unwrap during rendering after resource initialization
    pipeline: Option<RenderPipeline>,
    target_handles: Option<RenderTargetHandles>,
    persistent_samplers: Option<PersistentSamplers>,
    resolution: PixelResolution,

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
            let last_resolution = self.resolution;
            screen.sync().unwrap();

            let resolution = screen.resolution();
            self.resolution =
                PixelResolution::new(resolution.width as u32, resolution.height as u32);

            if resolution.is_changed()
                && last_resolution.width() != resolution.width as u32
                && last_resolution.height() != resolution.height as u32
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
            skybox,
            cage_visual,
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

        cage_visual.bind();
        cage_visual.uniform_projection_mat4v([*proj]);
        cage_visual.uniform_view_mat4v([view_mat]);

        #[cfg(feature = "devmode")]
        {
            lines.bind();
            lines.uniform_projection_mat4v([*proj]);
            lines.uniform_view_mat4v([view_mat]);
        }

        skybox.bind();
        skybox.uniform_projection_mat4v([*proj]);
        skybox.uniform_view_mat4v([view_mat]);

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

        self.sync_cage_changes(frame_data, section);

        // cage deformation (derive cov. + svd) compute pass
        {
            use pass::cage_deform_compute::CageDeformComputeCtx;

            let ctx = CageDeformComputeCtx {
                total_cage_count: cage_count,
                cage_data: &frame_data.cages,
                lattice_data: &frame_data.lattice_debug,
                cage_feedback: &frame_data.cage_feedback,
            };

            self.shaders.cage_deform.bind();
            self.shaders
                .cage_deform
                .uniform_total_cage_count_uintv([cage_count]);

            self.pipeline()
                .cage_deform_compute_pass
                .execute(section, render_pool, &ctx);
        }
        // there is no barrier here: an ssbo barrier is set after
        // fd_preprocess, which does not depend on this pass

        // unsafe {
        //     janus::gl::Clear(janus::gl::COLOR_BUFFER_BIT | janus::gl::DEPTH_BUFFER_BIT);
        // }

        // clear all render-targets once
        self.pipeline()
            .clear_pass
            .execute(section, render_pool, &());

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
                    cages_map: &frame_data.cage_map,
                    fragments_data: frags_buf,
                    fragments_commands: frags_cmd,
                    material_registry: self.materials.locations(),
                };
                self.pipeline()
                    .fragments_draw_pass
                    .execute(section, render_pool, &ctx);

                // skybox draw pass
                {
                    self.pipeline()
                        .skybox_draw_pass
                        .execute(section, render_pool, &());
                }

                // --------------------------------------------------
                // everything else currently draws on the default
                // framebuffer, so perform tonemapping (& blit) now
                // --------------------------------------------------

                // tonemapping + gamma correction vfx pass
                {
                    let ctx = pass::TonemapVfxCtx {
                        shader: &self.shaders.vfx_tonemap,
                        resolution: self.resolution,
                        gamma_override: None,
                    };
                    self.pipeline()
                        .tonemap_vfx_pass
                        .execute(section, render_pool, &ctx);
                }

                // blit specialized pass
                self.pipeline().blit_pass.execute(section, render_pool, &());
            }
            rendrs::framebuffer::bind_default();
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
                cage_data: &frame_data.cages,
                point_size: 0.65,
                cage_total_count: cage_count,
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

        // init persistent samplers
        {
            const DEV_ENV_NAME: &str = super::assets::ENVMAP_EQUIRECT_ENV_NAME_CITRUS_ORCHARD;
            const DEV_ENV_ID: StringHash = janus::hash_string(DEV_ENV_NAME);

            let equirect_tex = {
                let raw = {
                    let handle = texture_assets.get_mut(DEV_ENV_ID).unwrap();
                    handle.load_to_memory(&()).unwrap();
                    let raw = handle.take_from_memory().unwrap();
                    let rgba = image::DynamicImage::ImageRgba32F(raw.0.into_rgba32f());
                    RawTexture::new(rgba)
                };
                Texture::from_2d_image(raw.image(), MipLevels::default())
            }
            .unwrap();

            const CUBEMAP_RESOLUTION: i32 = 1024;
            let cubemap_tex = Texture::new_cubemap(
                CUBEMAP_RESOLUTION,
                CUBEMAP_RESOLUTION,
                MipLevels::default(),
                ImageType::Float16,
                ImageFormat::Rgba,
            );

            let shader = &self.shaders.util_equirect_decode;
            let decode_pass = pass::equirect_decode_compute::pass(shader);
            let decode_ctx = pass::EquirectDecodeCtx {
                shader,
                src_equirect: ImageObjectTarget::new(
                    ImageObject::from_direct_texture(equirect_tex.view()),
                    ImageAccessKind::ReadOnly,
                    pass::equirect_decode_compute::IMAGE_BINDING_SRC_EQUIRECT,
                    None,
                ),
                dst_cubemap: ImageObjectTarget::new(
                    ImageObject::from_direct_texture(cubemap_tex.view()),
                    ImageAccessKind::WriteOnly,
                    pass::equirect_decode_compute::IMAGE_BINDING_DST_CUBEMAP,
                    None,
                ),
            };
            decode_pass.execute(StorageSection::Spare, &self.render_pool, &decode_ctx);
            janus::gl::barrier_shader_image();

            texture_assets.add_handle(Handle::from_gpu_resource(
                DEV_ENV_ID,
                cubemap_tex,
                texture_assets,
            ));
            let dev_env_cubemap = texture_assets.get_gpu_view(DEV_ENV_ID).unwrap();

            self.persistent_samplers = Some(PersistentSamplers {
                dev_envmap_cubemap: SamplerObject::new(dev_env_cubemap),
            });
        }

        // init pipeline
        {
            let dev_materials = &self.materials.groups().dev;
            let skybox_sampler = self.persistent_samplers().dev_envmap_cube();

            let (base_hdr, base_depth, mapped_ldr) = {
                let id_hdr = self.render_target_handles().hdr_base;
                let id_depth = self.render_target_handles().base_depth;
                let id_ldr = self.render_target_handles().ldr_mapped;
                (
                    self.render_pool.accessor(id_hdr).unwrap(),
                    self.render_pool.accessor(id_depth).unwrap(),
                    self.render_pool.accessor(id_ldr).unwrap(),
                )
            };

            self.pipeline = Some(RenderPipeline {
                fd_preprocess_pass: pass::fd_preprocess::pass(&self.shaders.fd_preprocess),
                fragments_draw_pass: pass::fragments_draw::pass(
                    &self.shaders.fragments,
                    dev_materials,
                    base_hdr,
                    base_depth,
                ),
                debris_draw_pass: pass::debris_draw::pass(&self.shaders.debris),
                skybox_draw_pass: pass::skybox_draw::pass(
                    &self.shaders.skybox,
                    skybox_sampler,
                    base_hdr,
                    base_depth,
                ),
                debug_lattice_draw_pass: pass::debug_lattice_draw::pass(&self.shaders.lattice),
                cage_deform_compute_pass: pass::cage_deform_compute::pass(
                    &self.shaders.cage_deform,
                ),
                debug_cage_draw_pass: pass::debug_cage_draw::pass(&self.shaders.cage_visual),
                interface_draw_pass: gui::render::pass(&self.shaders.interface),
                tonemap_vfx_pass: pass::tonemap_compute::pass(
                    &self.shaders.vfx_tonemap,
                    base_hdr,
                    mapped_ldr,
                ),

                #[cfg(feature = "devmode")]
                debug_lines_draw_pass: pass::debug_lines_draw::pass(&self.shaders.lines),

                blit_pass: rendrs::BlitPass::new(mapped_ldr),
                clear_pass: rendrs::ClearPass::new([
                    OutputObject::Color(base_hdr),
                    OutputObject::Color(mapped_ldr),
                    OutputObject::Depth(base_depth),
                ]),
            });
        }
    }
}
impl Renderer {
    fn persistent_samplers(&self) -> &PersistentSamplers {
        self.persistent_samplers
            .as_ref()
            .expect("persistent samplers must be present after resource initialization")
    }

    fn pipeline(&self) -> &RenderPipeline {
        self.pipeline
            .as_ref()
            .expect("render pipeline must be present after resource initialization")
    }

    fn render_target_handles(&self) -> &RenderTargetHandles {
        self.target_handles
            .as_ref()
            .expect("render targethandles must be present after resource initialization")
    }

    fn initialize_shaders(&mut self) {
        self.shaders.lattice = pass::ShaderDebugLattice::new_compiled();
        self.shaders.fragments = pass::ShaderFragment::new_compiled();
        self.shaders.debris = pass::ShaderDebris::new_compiled();
        self.shaders.cage_deform = pass::ComputeShaderCageDeform::new_compiled();
        self.shaders.cage_visual = pass::ShaderDebugCage::new_compiled();
        self.shaders.interface = gui::render::ShaderUiBasic::new_compiled();
        self.shaders.fd_preprocess = pass::ComputeShaderProcessCommand::new_compiled();
        self.shaders.skybox = pass::ShaderSkybox::new_compiled();
        self.shaders.util_equirect_decode = pass::ComputeShaderEquirectDecode::new_compiled();
        self.shaders.vfx_tonemap = pass::ComputeShaderTonemap::new_compiled();

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
        let hdr_base = self.render_pool.add(RenderTarget::new(
            "base-drawbuffer-HDR",
            RenderTargetDescriptor::new(
                ImageFormat::Rgba,
                ImageType::Float16,
                TextureFiltering::Nearest,
                MipLevels::default(),
                1.0,
            ),
            resolution,
        ));
        let base_depth = self.render_pool.add(RenderTarget::new(
            "base-drawbuffer-depth24",
            RenderTargetDescriptor::new(
                ImageFormat::Depth,
                ImageType::Bits24,
                TextureFiltering::Nearest,
                MipLevels::default(),
                1.0,
            ),
            resolution,
        ));
        let ldr_mapped = self.render_pool.add(RenderTarget::new(
            "mapped-encoded-LDR",
            RenderTargetDescriptor::new(
                ImageFormat::Rgba,
                ImageType::Bits8,
                TextureFiltering::Nearest,
                MipLevels::default(),
                1.0,
            ),
            resolution,
        ));

        self.target_handles = Some(RenderTargetHandles {
            hdr_base,
            base_depth,
            ldr_mapped,
        });
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

    fn sync_cage_changes(&self, frame_data: &FrameDataBuffers, section: StorageSection) {
        let sync = frame_data.cage_sync_frame.gpu();

        let cage_buf = &frame_data.cages;
        let cage_map = &frame_data.cage_map;
        let mut imap = unsafe { cage_map.view_section_mut(section.as_index()) };

        sync.upload(section, cage_buf);
        sync.delete(section, cage_buf, imap.as_mut_slice());
    }
}
