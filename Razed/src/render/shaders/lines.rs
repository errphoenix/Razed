use super::commons;

use ethel::shader::{GlslStruct, GlslUniform, ShaderKind};
use ethel::state::data::IndirectIndex;

#[cfg(feature = "devmode")]
#[derive(Clone, Debug, Default)]
pub struct DebugLinesData {
    pub positions: Vec<glam::Vec3>,
    pub colors: Vec<glam::Vec4>,
}

#[cfg(feature = "devmode")]
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
                projection: mat4 => glam::Mat4;
                view: mat4 => glam::Mat4;
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

            src() "
                uint point_id = gl_VertexID;

                vec4 point = points[point_id];
                vec4 color = colors[point_id];

                fs_color = color;

                gl_Position = projection * view * vec4(point.xyz, 1.0);
            "
        ];

        unit ShaderKind::Pixel => [
            attribs {
                ethel::shader_glsl_attribs! {
                    input fs_color: vec4;
                    output out_Color: vec4;
                }
            };

            src() "
                out_Color = fs_color;
            "
        ];
    }
}
