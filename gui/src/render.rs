use ethel::{
    render::buffer::TriBuffer,
    shader::{GlslStruct, ShaderKind},
};
use janus::texture::{Tex, TextureView};
use rendrs::pipeline::DrawPass;

use crate::draw::Quad;

pub type UiDataBuffer = TriBuffer<Quad>;
pub type UiCommandsBuffer = TriBuffer<UiRenderCommandBasic>;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Hash)]
pub struct UiRenderCommandBasic {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub instance_offset: u32,
    pub texture_units: [Option<TextureView>; rendrs::BATCH_UNITS],
}
impl UiRenderCommandBasic {
    pub fn bind_texture_units(&self) {
        janus::assert_gl!();

        self.texture_units
            .iter()
            .enumerate()
            .filter_map(|(i, tex)| tex.and_then(|tex| Some((i, tex))))
            .for_each(|(index, texture)| {
                texture.bind(index as u32);
            });
    }
}

pub type UiDrawPass = DrawPass<UiDrawPassCtxWrapper, 0, 0>;

#[derive(Debug)]
pub struct UiDrawPassCtx<'data> {
    pub ui_shader: &'data ShaderUiBasic,
    pub data: &'data UiDataBuffer,
    pub commands: &'data UiCommandsBuffer,
}

rendrs::context_wrapper!(for<'ctx> UiDrawPassCtx);

pub const fn pass(shader: &ShaderUiBasic) -> UiDrawPass {
    let handle_view = shader.handle().view();
    UiDrawPass::new(handle_view, [], [], |section, ctx| {
        const SSBO_INDEX: u32 = SSBO_INDEX_POD_ELEMENTS;
        let section = section.as_index();

        unsafe {
            janus::gl::Disable(janus::gl::DEPTH_TEST);
        }

        // SAFETY: safe access to the commands buffer is guaranteed by the
        // correct triple-buffer section index
        let commands = unsafe { ctx.commands.view_section(section) };
        ctx.data.bind_shader_storage(section, SSBO_INDEX, 0);

        let mut texture_masks = [0u32; rendrs::BATCH_UNITS];
        for command in commands.iter() {
            if command.instance_count == 0 {
                continue;
            }

            command.bind_texture_units();
            let offset = command.instance_offset;
            for i in 0..rendrs::BATCH_UNITS {
                let unit = command.texture_units[i];
                let has_texture = unit.is_some_and(|tex| tex.texture_id() != 0);
                texture_masks[i] = has_texture as u32;
            }

            let ui_shader = ctx.ui_shader;
            ui_shader.uniform_texture_masks_uintv(texture_masks);
            ui_shader.uniform_instance_offset_uintv([offset]);

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

        unsafe {
            janus::gl::Enable(janus::gl::DEPTH_TEST);
        }
    })
}

ethel::shader_glsl_struct! {
    struct QuadElement {
        position: [f32; 2] => vec2,
        size: [f32; 2] => vec2,
        color: [f32; 4] => vec4,
        uv: [f32; 4] => vec4,
        tex_unit: u32 => uint
    }
}

macro_rules! ssbo_binding {
    (POD_Elements) => {
        5
    };
}

pub const SSBO_INDEX_POD_ELEMENTS: u32 = ssbo_binding!(POD_Elements);
pub const TYPE_QUAD_ELEMENT: GlslStruct = QuadElementGlslStruct::as_definition();

ethel::shader_glsl! {
    struct UiBasic > [460] {
        common {};

        unit ShaderKind::Vertex => [
            attribs {
                ethel::shader_glsl_attribs! {
                    output fs_color: vec4;
                    output tex_coord: vec2;
                    output screen_point: vec2;
                }
                ethel::shader_glsl_attribs! {
                    output texture_index: uint flat: true;
                }
            };
            uniform {
                length 1, projection: mat4 => glam::Mat4;
                length 1, instance_offset: uint => u32;
            };
            type {
                TYPE_QUAD_ELEMENT
            };
            ssbo {
                ethel::shader_glsl_ssbo! {
                    buf POD_Elements => {
                        [dyn_array QuadElement: pod_elements]
                    }
                }
            };

            src() {
                "
                uint v_id = gl_VertexID;
                uint instance = gl_InstanceID + instance_offset;

                float x = float(floor(v_id / 2));
                float y = float(v_id % 2);
                vec2 p = vec2(x, y);

                QuadElement element = pod_elements[instance];
                p *= element.size;
                p += element.position;

                vec4 vertex = projection * vec4(p.x, p.y, 0.0, 1.0);
                vertex.z = 0.0;

                vec4 atlas_section = element.uv;
                float u = mix(atlas_section.x, atlas_section.z, x);
                float v = mix(atlas_section.y, atlas_section.w, y);

                tex_coord = vec2(u, v);
                fs_color = element.color;
                texture_index = element.tex_unit;
                screen_point = vertex.xy;

                gl_Position = vertex;
                ";
            }
        ];

        unit ShaderKind::Pixel => [
            attribs {
                ethel::shader_glsl_attribs! {
                    input fs_color: vec4;
                    input tex_coord: vec2;
                    input screen_point: vec2;
                    output outColor: vec4;
                }
                ethel::shader_glsl_attribs! {
                    input texture_index: uint flat: true;
                }
            };
            uniform {
                length 16, texture_masks: uint => u32;
            };
            sampler {
                on 0, for 16 => texture_map : sampler2D;
            };

            src() {
                "
                vec4 color = fs_color;
                vec2 uv = tex_coord;
                uint tex_index = texture_index;

                float tex_mask = float(texture_masks[tex_index]);
                vec4 tex_color = texture(texture_map[tex_index], uv);
                outColor = color * mix(vec4(1.0), tex_color, tex_mask);
                ";
            }
        ];
    }
}
