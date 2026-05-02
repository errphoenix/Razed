mod shaders;

use std::sync::atomic::Ordering;

use ethel::{DrawCommand, render::command::GpuCommandDispatch};

use crate::{
    data::{FrameDataBuffers, LayoutFragmentData},
    render::shaders::compute::ComputeShaderProcessCommand,
};

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

        // self.xpbd_dbg_shader.bind();
        // self.xpbd_dbg_shader.uniform_mat4_glam("u_view", view_mat);
        // self.xpbd_dbg_shader
        //     .uniform_mat4_glam("u_projection", *proj);

        // self.line_dbg_shader.bind();
        // self.line_dbg_shader.uniform_mat4_glam("u_view", view_mat);
        // self.line_dbg_shader
        //     .uniform_mat4_glam("u_projection", *proj);

        // self.base_shader.bind();
        // self.base_shader
        //     .uniform_vec3_glam("u_camera_forward", cam_forward);
        // self.base_shader.uniform_mat4_glam("u_view", view_mat);
        // self.base_shader.uniform_mat4_glam("u_projection", *proj);

        // self.deform_dbg_shader.bind();
        // self.deform_dbg_shader.uniform_mat4_glam("u_view", view_mat);
        // self.deform_dbg_shader
        //     .uniform_mat4_glam("u_projection", *proj);

        // self.debris_shader.bind();
        // self.debris_shader
        //     .uniform_vec3_glam("u_camera_forward", cam_forward);
        // self.debris_shader.uniform_mat4_glam("u_view", view_mat);
        // self.debris_shader.uniform_mat4_glam("u_projection", *proj);

        // self.frags_shader.bind();
        // self.frags_shader
        //     .uniform_vec3_glam("u_camera_forward", cam_forward);
        // self.frags_shader.uniform_mat4_glam("u_view", view_mat);
        // self.frags_shader.uniform_mat4_glam("u_projection", *proj);
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

        let frags = &frame_data.fragments;
        frags.bind_shader_storage(buf_idx);

        let cmd_buf = &frame_data.command;
        let cmd_view = cmd_buf.view_section(buf_idx);

        {
            const COMPUTE_WG_SIZE_XY: u32 = shaders::compute::process_command::WORKGROUP_SIZE_XY;

            let cmd_len = cmd_view.length() / size_of::<DrawCommand>() as u32;
            let wg_d_count = (cmd_len as f32 / COMPUTE_WG_SIZE_XY as f32).round() as u32;

            self.command_process_compute
                .set_workgroups_size(wg_d_count, 0, 0);

            {
                let i_mesh_id = shaders::compute::process_command::SSBO_INDEX_FRAGMENTS_MESH_IDS;
                frame_data.fragments.bind_shader_storage_single(
                    buf_idx,
                    LayoutFragmentData::PodMeshId as usize,
                    Some(i_mesh_id),
                );

                let i_cmd_buf = shaders::compute::process_command::SSBO_INDEX_COMMAND_BUFFER;
                frame_data
                    .command
                    .bind_shader_storage(buf_idx, i_cmd_buf as usize, 0);
            }

            self.command_process_compute.dispatch();

            janus::gl::barrier_shader_storage();
        }

        GpuCommandDispatch::from_view(cmd_view).dispatch();

        {
            self.debris_shader.bind();
            let debris = &frame_data.debris;
            debris.bind_shader_storage(buf_idx);

            let debris_count = frame_data.debris_count.load(Ordering::Acquire) as i32;

            unsafe {
                janus::gl::DrawArraysInstanced(janus::gl::TRIANGLES, 0, 36, debris_count);
            }
        }
        {
            self.lattice_shader.bind();
            let xpbd_dbg = &frame_data.xpbd_debug;
            xpbd_dbg.bind_shader_storage(buf_idx);

            let xpbd_count = frame_data.xpbd_debug_link_count.load(Ordering::Acquire) as i32;

            unsafe {
                janus::gl::DrawArraysInstanced(janus::gl::LINES, 0, 2, xpbd_count);
            }
        }
        // {
        //     self.line_dbg_shader.bind();
        //     unsafe {
        //         janus::gl::DrawArrays(janus::gl::LINES, 0, 6);
        //     }
        // }
        // {
        //     const DEFORM_POINTS_SSBO: usize = 0;
        //     self.deform_dbg_shader.bind();
        //     frame_data
        //         .deform_debug
        //         .bind_shader_storage(buf_idx, DEFORM_POINTS_SSBO);
        //     let count = frame_data
        //         .deform_debug_count
        //         .load(Ordering::Acquire)
        //         .saturating_sub(1);
        //     unsafe {
        //         janus::gl::PointSize(2.0);
        //         janus::gl::DrawArrays(janus::gl::POINTS, 0, count as i32);
        //     }
        // }
    }

    fn init_resources(&mut self, _resolution: ethel::render::Resolution) {
        self.lattice_shader = shaders::debug::ShaderDebugLattice::new_compiled();
        self.frags_shader = shaders::ShaderFragment::new_compiled();
        self.debris_shader = shaders::ShaderDebris::new_compiled();

        self.command_process_compute =
            shaders::compute::ComputeShaderProcessCommand::new_compiled();

        // const VSH_BASE_SOURCE: &[u8] = include_bytes!("../shaders/base.vsh");
        // const FSH_BASE_SOURCE: &[u8] = include_bytes!("../shaders/base.fsh");
        // let mut vsh = std::io::BufReader::new(VSH_BASE_SOURCE);
        // let mut fsh = std::io::BufReader::new(FSH_BASE_SOURCE);
        // self.base_shader = ShaderHandle::new(&mut vsh, &mut fsh);

        // const VSH_CONSTRAINT_SOURCE: &[u8] = include_bytes!("../shaders/constraint.vsh");
        // const FSH_SOLID_SOURCE: &[u8] = include_bytes!("../shaders/solid.fsh");
        // let mut vsh = std::io::BufReader::new(VSH_CONSTRAINT_SOURCE);
        // let mut fsh = std::io::BufReader::new(FSH_SOLID_SOURCE);
        // self.xpbd_dbg_shader = ShaderHandle::new(&mut vsh, &mut fsh);

        // const VSH_LINE_SOURCE: &[u8] = include_bytes!("../shaders/line.vsh");
        // let mut vsh = std::io::BufReader::new(VSH_LINE_SOURCE);
        // let mut fsh = std::io::BufReader::new(FSH_SOLID_SOURCE);
        // self.line_dbg_shader = ShaderHandle::new(&mut vsh, &mut fsh);

        // const VSH_FRAG_SOURCE: &[u8] = include_bytes!("../shaders/fragment.vsh");
        // let mut vsh = std::io::BufReader::new(VSH_FRAG_SOURCE);
        // let mut fsh = std::io::BufReader::new(FSH_BASE_SOURCE);
        // self.frags_shader = ShaderHandle::new(&mut vsh, &mut fsh);

        // const VSH_DEFORM_SOURCE: &[u8] = include_bytes!("../shaders/cage.vsh");
        // let mut vsh = std::io::BufReader::new(VSH_DEFORM_SOURCE);
        // let mut fsh = std::io::BufReader::new(FSH_SOLID_SOURCE);
        // self.deform_dbg_shader = ShaderHandle::new(&mut vsh, &mut fsh);

        // const VSH_DEBRIS_SOURCE: &[u8] = include_bytes!("../shaders/debris.vsh");
        // let mut vsh = std::io::BufReader::new(VSH_DEBRIS_SOURCE);
        // let mut fsh = std::io::BufReader::new(FSH_BASE_SOURCE);
        // self.debris_shader = ShaderHandle::new(&mut vsh, &mut fsh);
    }
}
