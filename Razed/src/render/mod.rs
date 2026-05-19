pub mod shaders;

use std::sync::atomic::Ordering;

use ethel::render::command::{DrawGroups, GpuCommandDispatch};

use crate::data::{FrameDataBuffers, LayoutDebrisData, LayoutFragmentData};

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

    command_process_compute: shaders::compute::ComputeShaderProcessCommand,
}

impl ethel::RenderHandler<FrameDataBuffers> for Renderer {
    fn pre_frame(
        &mut self,
        screen: &mut janus::sync::Mirror<ethel::render::ScreenSpace>,
        view: &mut janus::sync::Mirror<ethel::state::camera::ViewPoint>,
        _delta: janus::context::DeltaTime,
    ) {
        view.sync().unwrap();
        screen.sync().unwrap();
        let view_mat = view.into_mat4().inverse();
        let proj = screen.projection();
        let cam_forward = view.forward();

        self.lattice_shader.bind();
        self.lattice_shader.uniform_projection_mat4(*proj);
        self.lattice_shader.uniform_view_mat4(view_mat);

        self.debris_shader.bind();
        self.debris_shader.uniform_camera_forward_vec3(cam_forward);
        self.debris_shader.uniform_projection_mat4(*proj);
        self.debris_shader.uniform_view_mat4(view_mat);

        self.frags_shader.bind();
        self.frags_shader.uniform_camera_forward_vec3(cam_forward);
        self.frags_shader.uniform_projection_mat4(*proj);
        self.frags_shader.uniform_view_mat4(view_mat);
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

            // draw dispatch - fragments, debris
            {
                frags_buf.bind_shader_storage(buf_idx);
                self.frags_shader.bind();
                GpuCommandDispatch::from_view(frags_cmd_view).dispatch();

                debris_buf.bind_shader_storage(buf_idx);
                self.debris_shader.bind();
                GpuCommandDispatch::from_view(debris_cmd_view).dispatch();
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
    }

    fn init_resources(&mut self, _resolution: ethel::render::Resolution) {
        self.lattice_shader = shaders::debug::ShaderDebugLattice::new_compiled();
        self.frags_shader = shaders::ShaderFragment::new_compiled();
        self.debris_shader = shaders::ShaderDebris::new_compiled();

        self.command_process_compute =
            shaders::compute::ComputeShaderProcessCommand::new_compiled();
    }
}
