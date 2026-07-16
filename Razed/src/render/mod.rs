pub mod shaders;

use std::sync::atomic::Ordering;

use ethel::render::command::{DrawGroups, GpuCommandDispatch};
use gui::{
    draw::Batch,
    text::{GlyphAtlasTexture, GlyphRaster},
};

#[cfg(feature = "devmode")]
use crate::render::shaders::lines::DebugLinesData;
use crate::{
    assets,
    data::{FrameDataBuffers, LayoutDebrisData, LayoutFragmentData},
};

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

#[derive(Debug, Default)]
pub struct Renderer {
    lattice_shader: shaders::debug::ShaderDebugLattice,
    frags_shader: shaders::ShaderFragment,
    debris_shader: shaders::ShaderDebris,
    lines_shader: shaders::ShaderDebugLines,
    cage_shader: shaders::debug::ShaderDebugCage,
    interface_shader: gui::shaders::ShaderUiBasic,

    command_process_compute: shaders::compute::ComputeShaderProcessCommand,

    #[cfg(feature = "devmode")]
    lines_debug_buffer: DebugLinesData,

    pub textures_master_registry: assets::TextureRegistry,

    pub glyph_atlas_texture: GlyphAtlasTexture,
    pub glyph_pipe: Option<crossbeam::channel::Receiver<GlyphRaster>>,
}
impl ethel::RenderHandler<FrameDataBuffers> for Renderer {
    fn pre_frame(
        &mut self,
        screen: &mut janus::sync::Mirror<ethel::render::ScreenSpace>,
        view: &janus::sync::TriCell<ethel::state::camera::ViewPoint>,
        _delta: janus::context::DeltaTime,
    ) {
        screen.sync().unwrap();

        let view_mat = view.into_mat4().inverse();
        let proj = screen.projection();
        let ortho_proj = screen.orto_projection();
        let cam_forward = view.forward();

        // world axis indicator
        #[cfg(feature = "devmode")]
        {
            self.lines_debug_buffer.clear();

            const OFFSET: f32 = 1.0;
            let o = view.position + cam_forward * OFFSET;

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

        self.interface_shader.bind();
        self.interface_shader
            .uniform_projection_mat4v([*ortho_proj]);

        self.lattice_shader.bind();
        self.lattice_shader.uniform_projection_mat4v([*proj]);
        self.lattice_shader.uniform_view_mat4v([view_mat]);

        self.cage_shader.bind();
        self.cage_shader.uniform_projection_mat4v([*proj]);
        self.cage_shader.uniform_view_mat4v([view_mat]);

        self.lines_shader.bind();
        self.lines_shader.uniform_projection_mat4v([*proj]);
        self.lines_shader.uniform_view_mat4v([view_mat]);

        self.debris_shader.bind();
        self.debris_shader
            .uniform_camera_forward_vec3v([cam_forward]);
        self.debris_shader.uniform_projection_mat4v([*proj]);
        self.debris_shader.uniform_view_mat4v([view_mat]);

        self.frags_shader.bind();
        self.frags_shader
            .uniform_camera_forward_vec3v([cam_forward]);
        self.frags_shader.uniform_projection_mat4v([*proj]);
        self.frags_shader.uniform_view_mat4v([view_mat]);

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
        unsafe {
            janus::gl::Clear(janus::gl::COLOR_BUFFER_BIT | janus::gl::DEPTH_BUFFER_BIT);
        }
        let buf_idx = section.as_index();

        // fragments & debris
        {
            let frags_buf = &frame_data.fragments;
            let frags_cmd = &frame_data.fragment_commands;
            let frags_cmd_view = frags_cmd.view_section(buf_idx);

            let debris_buf = &frame_data.debris;
            let debris_cmd = &frame_data.debris_commands;
            let debris_cmd_view = debris_cmd.view_section(buf_idx);

            const COMPUTE_WG_INVOCATIONS: u32 =
                shaders::compute::process_command::WORKGROUP_INVOCATIONS;

            // command preprocess (compute) - fragments, debris
            {
                self.command_process_compute.bind();
                let cmd_len = frags_cmd_view.length();
                let wg_d_count = cmd_len.div_ceil(COMPUTE_WG_INVOCATIONS);
                self.command_process_compute
                    .set_workgroups_size(wg_d_count, 1, 1);
                {
                    let i_mesh_id =
                        shaders::compute::process_command::SSBO_INDEX_FRAGMENTS_MESH_IDS;
                    frags_buf.bind_shader_storage_single(
                        buf_idx,
                        LayoutFragmentData::PodMeshId as usize,
                        Some(i_mesh_id),
                    );

                    let i_cmd_buf = shaders::compute::process_command::SSBO_INDEX_COMMAND_BUFFER;
                    frags_cmd.bind_shader_storage(buf_idx, i_cmd_buf as usize, 0);
                }
                self.command_process_compute.dispatch();
            }
            {
                self.command_process_compute.bind();
                let cmd_len = debris_cmd_view.length();
                let wg_d_count = cmd_len.div_ceil(COMPUTE_WG_INVOCATIONS);
                self.command_process_compute
                    .set_workgroups_size(wg_d_count, 1, 1);
                {
                    let i_mesh_id =
                        shaders::compute::process_command::SSBO_INDEX_FRAGMENTS_MESH_IDS;
                    debris_buf.bind_shader_storage_single(
                        buf_idx,
                        LayoutDebrisData::PodMeshId as usize,
                        Some(i_mesh_id),
                    );

                    let i_cmd_buf = shaders::compute::process_command::SSBO_INDEX_COMMAND_BUFFER;
                    debris_cmd.bind_shader_storage(buf_idx, i_cmd_buf as usize, 0);
                }
                self.command_process_compute.dispatch();
            }

            janus::gl::barrier_shader_storage();
            janus::gl::barrier_commands();

            // draw dispatch - fragments, debris (also debug cage)
            {
                frags_buf.bind_shader_storage(buf_idx);
                self.frags_shader.bind();
                GpuCommandDispatch::from_view(frags_cmd_view).dispatch();

                let debug_cage_size = frame_data.cage_points_count.load(Ordering::Acquire);
                self.cage_shader.bind();
                frags_buf.bind_shader_storage_single(
                    buf_idx,
                    LayoutFragmentData::PodDeformsPositions as usize,
                    Some(7),
                );
                unsafe {
                    janus::gl::PointSize(3.0);
                    janus::gl::DrawArrays(janus::gl::POINTS, 0, debug_cage_size as i32);
                }

                debris_buf.bind_shader_storage(buf_idx);
                self.debris_shader.bind();
                GpuCommandDispatch::from_view(debris_cmd_view).dispatch();
            }
        }

        unsafe {
            janus::gl::Disable(janus::gl::DEPTH_TEST);
        }

        // draw dispatch - interface
        {
            self.interface_shader.bind();

            const QUAD_SSBO_INDEX: u32 = gui::shaders::SSBO_INDEX_POD_ELEMENTS;

            let quads = &frame_data.interface_storage;
            let commands = &frame_data.interface_commands.view_section(buf_idx);

            quads.bind_shader_storage(buf_idx, QUAD_SSBO_INDEX as usize, 0);

            let mut texture_masks = [0u32; Batch::UNITS];

            for command in commands.iter() {
                if command.instance_count == 0 {
                    continue;
                }

                command.bind_texture_units();
                let offset = command.instance_offset;

                for i in 0..Batch::UNITS {
                    let unit = command.texture_units[i];
                    let has_texture = unit.is_some_and(|tex| tex.0 != 0);
                    texture_masks[i] = has_texture as u32;
                }

                self.interface_shader
                    .uniform_texture_masks_uintv(texture_masks);
                self.interface_shader
                    .uniform_instance_offset_uintv([offset]);

                let count = command.vertex_count;
                let instance_count = command.instance_count;
                unsafe {
                    janus::gl::DrawArraysInstanced(
                        janus::gl::TRIANGLE_STRIP,
                        0,
                        count as i32,
                        instance_count as i32,
                    );
                }
            }
        }

        // draw dispatch - lattice (debug)
        {
            self.lattice_shader.bind();
            let xpbd_dbg = &frame_data.lattice_debug;
            xpbd_dbg.bind_shader_storage(buf_idx);

            let xpbd_count = frame_data.lattice_constraint_count.load(Ordering::Acquire) as i32;

            unsafe {
                janus::gl::DrawArraysInstanced(janus::gl::LINES, 0, 2, xpbd_count);
            }
        }

        // draw dispatch - lines (debug)
        #[cfg(feature = "devmode")]
        {
            use crate::data::LayoutDebugLinesData;

            let lines_data = &frame_data.lines_debug;

            unsafe {
                lines_data.blit_part_padded(
                    buf_idx,
                    LayoutDebugLinesData::PodPoints as usize,
                    &self.lines_debug_buffer.positions,
                    0,
                    4,
                );

                lines_data.blit_part(
                    buf_idx,
                    LayoutDebugLinesData::PodColors as usize,
                    &self.lines_debug_buffer.colors,
                    0,
                );
            }

            self.lines_shader.bind();
            lines_data.bind_shader_storage(buf_idx);

            let count = self.lines_debug_buffer.len();

            unsafe {
                janus::gl::DrawArrays(janus::gl::LINES, 0, count as i32);
            }
        }

        unsafe {
            janus::gl::Enable(janus::gl::DEPTH_TEST);
        }
    }

    fn init_resources(&mut self, _resolution: ethel::render::Resolution) {
        self.lattice_shader = shaders::debug::ShaderDebugLattice::new_compiled();
        self.frags_shader = shaders::ShaderFragment::new_compiled();
        self.debris_shader = shaders::ShaderDebris::new_compiled();
        self.lines_shader = shaders::ShaderDebugLines::new_compiled();
        self.cage_shader = shaders::debug::ShaderDebugCage::new_compiled();

        self.interface_shader = gui::shaders::ShaderUiBasic::new_compiled();
        let sampler_uniforms = std::array::from_fn(|i| i as i32);
        self.interface_shader.bind();
        self.interface_shader
            .uniform_texture_map_sampler2Dv(sampler_uniforms);

        self.command_process_compute =
            shaders::compute::ComputeShaderProcessCommand::new_compiled();
    }
}
