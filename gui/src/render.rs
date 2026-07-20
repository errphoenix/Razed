use ethel::shader::{GlslStruct, GlslUniform, ShaderKind, ShaderProgram};

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

            src() "
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
            "
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
                length 16, texture_map: sampler2D => i32;
                length 16, texture_masks: uint => u32;
            };

            src() "
                vec4 color = fs_color;
                vec2 uv = tex_coord;
                uint tex_index = texture_index;

                float tex_mask = float(texture_masks[tex_index]);
                vec4 tex_color = texture(texture_map[tex_index], uv);
                outColor = color * mix(vec4(1.0), tex_color, tex_mask);
            "
        ];
    }
}
