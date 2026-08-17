use ethel::{render::buffer::PartitionedTriBuffer, shader::ShaderKind};
use rendrs::pipeline::DrawPass;

use crate::data::LayoutDebugLinesData;

#[derive(Clone, Debug, Default)]
pub struct DebugLinesData {
    pub positions: Vec<glam::Vec3>,
    pub colors: Vec<glam::Vec4>,
}

#[allow(unused)]
impl DebugLinesData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            positions: Vec::with_capacity(capacity),
            colors: Vec::with_capacity(capacity),
        }
    }

    pub fn add(&mut self, position: glam::Vec3, color: glam::Vec4) {
        self.positions.push(position);
        self.colors.push(color);
    }

    pub fn add_position(&mut self, position: glam::Vec3) {
        self.positions.push(position);
    }

    pub fn add_color(&mut self, color: glam::Vec4) {
        self.colors.push(color);
    }

    pub fn clear(&mut self) {
        self.positions.clear();
        self.colors.clear();
    }

    /// Sets the `color` to all points for those that were not manually set.
    pub fn set_color_fallback(&mut self, color: glam::Vec4) {
        let len = self.positions.len();
        self.colors.resize(len, color);
    }

    pub fn len(&self) -> usize {
        self.positions.len().min(self.colors.len())
    }
}

pub type DebugLinesDrawPass = DrawPass<DebugLinesDrawCtxWrapper, 0, 0>;

#[derive(Debug)]
pub struct DebugLinesDrawCtx<'data> {
    pub lines_data: &'data PartitionedTriBuffer<2>,
    pub lines_buffer: &'data DebugLinesData,
}

rendrs::context_wrapper!(for<'ctx> DebugLinesDrawCtx);

pub const fn pass(shader: &ShaderDebugLines) -> DebugLinesDrawPass {
    let handle_view = shader.handle().view();
    DebugLinesDrawPass::new(handle_view, [], [], |section, ctx| {
        let section = section.as_index();
        unsafe {
            ctx.lines_data.blit_part_padded(
                section,
                LayoutDebugLinesData::PodPoints as usize,
                &ctx.lines_buffer.positions,
                0,
                4,
            );
            ctx.lines_data.blit_part(
                section,
                LayoutDebugLinesData::PodColors as usize,
                &ctx.lines_buffer.colors,
                0,
            );
        }

        ctx.lines_data.bind_shader_storage(section);
        let count = ctx.lines_buffer.len() as i32;
        unsafe {
            janus::gl::DrawArrays(janus::gl::LINES, 0, count);
        }
    })
}

macro_rules! ssbo_binding {
    (POD_Points) => {
        1
    };
    (POD_Colors) => {
        2
    };
}

pub const SSBO_INDEX_POD_POINTS: u32 = ssbo_binding!(POD_Points);
pub const SSBO_INDEX_POD_COLORS: u32 = ssbo_binding!(POD_Colors);

ethel::shader_glsl! {
    struct DebugLines > [460] {
        common {};

        unit ShaderKind::Vertex => [
            attribs {
                ethel::shader_glsl_attribs! {
                    output fs_color: vec4;
                }
            };

            uniform {
                length 1, projection: mat4 => glam::Mat4;
                length 1, view: mat4 => glam::Mat4;
            };

            ssbo {
                ethel::shader_glsl_ssbo! {
                    buf POD_Points => {
                        [dyn_array vec4: points]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf POD_Colors => {
                        [dyn_array vec4: colors]
                    }
                }
            };

            src() {
                "
                uint point_id = gl_VertexID;

                vec4 point = points[point_id];
                vec4 color = colors[point_id];

                fs_color = color;

                gl_Position = projection * view * vec4(point.xyz, 1.0);
                ";
            }
        ];

        unit ShaderKind::Pixel => [
            attribs {
                ethel::shader_glsl_attribs! {
                    input fs_color: vec4;
                    output out_Color: vec4;
                }
            };

            src() {
                "
                out_Color = fs_color;
                ";
            }
        ];
    }
}
