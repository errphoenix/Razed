use ethel::shader::{GlslStruct, GlslUniform, ShaderKind};

ethel::shader_glsl_struct! {
    struct QuadElement {
        position: [f32; 2] => vec2;
        size: [f32; 2] => vec2;
        color: [f32; 4] => vec4;
        uv: [f32; 4] => vec4;
        tex_unit: u32 => uint;
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
                    output texture_index: uint;
                }
            };
            uniform {
                projection: mat4 => glam::Mat4;
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

            src() "
                uint v_id = gl_VertexID;
                uint instance = gl_InstanceID;

                float x = float(floor(v_id / 2));
                float y = float(v_id % 2);
                vec2 p = vec2(x, y);

                QuadElement element = pod_elements[instance];
                p *= element.size;
                p += element.position;

                vec4 vertex = projection * vec4(p.x, p.y, 0.0, 1.0);

                fs_color = element.color;
                quad_uv = element.uv;
                texture_index = element.tex_unit;
                screen_point = vertex.xy;

                gl_Position = vertex;
            "
        ];

        unit ShaderKind::Pixel => [
            attribs {
                ethel::shader_glsl_attribs! {
                    input fs_color: vec4;
                    input tex_coord: vec2;
                    input screen_point: vec2;
                    input texture_index: uint;
                    output outColor: vec4;
                }
            };
            sampler {
                sampler2D texture_map array 16;
            };

            src() "
            "
        ];
    }
}
