use std::sync::atomic::Ordering;

use ethel::{render::command::GpuCommandDispatch, shader::ShaderHandle};

use crate::data::FrameDataBuffers;

#[derive(Debug, Default)]
pub struct Renderer {
    base_shader: ShaderHandle,
    xpbd_dbg_shader: ShaderHandle,
    line_dbg_shader: ShaderHandle,
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
        let view_mat = view.into_mat4();
        let proj = screen.projection();

        self.xpbd_dbg_shader.bind();
        self.xpbd_dbg_shader.uniform_mat4_glam("u_view", view_mat);
        self.xpbd_dbg_shader
            .uniform_mat4_glam("u_projection", *proj);

        self.line_dbg_shader.bind();
        self.line_dbg_shader.uniform_mat4_glam("u_view", view_mat);
        self.line_dbg_shader
            .uniform_mat4_glam("u_projection", *proj);

        self.base_shader.bind();
        self.base_shader.uniform_mat4_glam("u_view", view_mat);
        self.base_shader.uniform_mat4_glam("u_projection", *proj);
    }

    fn render_frame(
        &self,
        frame_data: &FrameDataBuffers,
        section: ethel::render::buffer::StorageSection,
    ) {
        let buf_idx = section.as_index();

        let scene = &frame_data.scene;
        scene.bind_shader_storage(buf_idx);

        unsafe {
            janus::gl::Clear(janus::gl::COLOR_BUFFER_BIT | janus::gl::DEPTH_BUFFER_BIT);
        }

        let cmds = &frame_data.command;
        GpuCommandDispatch::from_view(cmds.view_section(buf_idx)).dispatch();

        {
            self.xpbd_dbg_shader.bind();

            let xpbd_dbg = &frame_data.xpbd_debug;
            xpbd_dbg.bind_shader_storage(buf_idx);

            let xpbd_count = frame_data.xpbd_debug_link_count.load(Ordering::Acquire) as i32;
            unsafe {
                janus::gl::DrawArraysInstanced(janus::gl::LINES, 0, 2, xpbd_count);
            }
        }
        {
            self.line_dbg_shader.bind();
            unsafe {
                janus::gl::DrawArrays(janus::gl::LINES, 0, 6);
            }
        }
    }

    fn init_resources(&mut self, _resolution: ethel::render::Resolution) {
        const VSH_BASE_SOURCE: &[u8] = include_bytes!("../shaders/base.vsh");
        const FSH_BASE_SOURCE: &[u8] = include_bytes!("../shaders/base.fsh");

        let mut vsh = std::io::BufReader::new(VSH_BASE_SOURCE);
        let mut fsh = std::io::BufReader::new(FSH_BASE_SOURCE);
        self.base_shader = ShaderHandle::new(&mut vsh, &mut fsh);

        const VSH_CONSTRAINT_SOURCE: &[u8] = include_bytes!("../shaders/constraint.vsh");
        let mut vsh = std::io::BufReader::new(VSH_CONSTRAINT_SOURCE);
        let mut fsh = std::io::BufReader::new(FSH_BASE_SOURCE);
        self.xpbd_dbg_shader = ShaderHandle::new(&mut vsh, &mut fsh);

        const VSH_LINE_SOURCE: &[u8] = include_bytes!("../shaders/line.vsh");
        const FSH_SOLID_SOURCE: &[u8] = include_bytes!("../shaders/solid.fsh");
        let mut vsh = std::io::BufReader::new(VSH_LINE_SOURCE);
        let mut fsh = std::io::BufReader::new(FSH_SOLID_SOURCE);
        self.line_dbg_shader = ShaderHandle::new(&mut vsh, &mut fsh);
    }
}
