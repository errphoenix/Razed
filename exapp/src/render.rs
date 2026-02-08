use ethel::{render::command::GpuCommandDispatch, shader::ShaderHandle};

use crate::data::FrameDataBuffers;

#[derive(Debug, Default)]
pub struct Renderer {
    shader: ShaderHandle,
}

const FOV: f32 = 80.0;

impl ethel::RenderHandler<FrameDataBuffers> for Renderer {
    fn pre_frame(
        &mut self,
        resolution: ethel::render::Resolution,
        view_point: &mut janus::sync::Mirror<ethel::state::camera::ViewPoint>,
        _delta: janus::context::DeltaTime,
    ) {
        self.shader.bind();
        let _ = view_point.sync();
        let view_mat = view_point.into_mat4();
        self.shader.uniform_mat4_glam("u_view", view_mat);

        let width = resolution.width;
        let height = resolution.height;
        let proj_mat = ethel::render::projection_perspective(width, height, FOV);
        self.shader.uniform_mat4_glam("u_projection", proj_mat);
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
    }

    fn init_resources(&mut self, _resolution: ethel::render::Resolution) {
        let mut vsh = std::io::BufReader::new(include_bytes!("../shaders/base.vsh").as_slice());
        let mut fsh = std::io::BufReader::new(include_bytes!("../shaders/base.fsh").as_slice());
        self.shader = ShaderHandle::new(&mut vsh, &mut fsh);
    }
}
