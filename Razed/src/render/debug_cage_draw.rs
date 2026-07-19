use ethel::shader::{GlslUniform, ShaderKind, ShaderProgram};

macro_rules! ssbo_binding {
    (POD_Deform_Points) => {
        7
    };
}

pub const SSBO_INDEX_POD_DEFORM_POINTS: u32 = ssbo_binding!(POD_Deform_Points);

ethel::shader_glsl! {
    struct DebugCage > [460] {
        common {};

        unit ShaderKind::Vertex => [
            uniform {
                length 1, projection: mat4 => glam::Mat4;
                length 1, view: mat4 => glam::Mat4;
            };

            ssbo {
                ethel::shader_glsl_ssbo! {
                    buf POD_Deform_Points => {
                        [dyn_array vec4: pod_deforms]
                    }
                }
            };

            src() "
                uint id = gl_VertexID + 1;
                vec3 deform = pod_deforms[id].xyz;
                gl_Position = projection * view * vec4(deform, 1.0);
            "
        ];

        unit ShaderKind::Pixel => [
            attribs {
                ethel::shader_glsl_attribs! {
                    output out_Color: vec4;
                }
            };

            src() "
                out_Color = vec4(1.0, 0.0, 1.0, 1.0);
            "
        ];
    }
}
