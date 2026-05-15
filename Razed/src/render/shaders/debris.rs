use super::commons;

use ethel::shader::{GlslUniform, ShaderKind};

macro_rules! ssbo_binding {
    (POD_Positions) => {
        0
    };
    (POD_Rotations) => {
        1
    };
    (POD_MeshID) => {
        3
    };
}

pub const SSBO_INDEX_POD_POSITIONS: u32 = ssbo_binding!(POD_Positions);
pub const SSBO_INDEX_POD_ROTATIONS: u32 = ssbo_binding!(POD_Rotations);
pub const SSBO_INDEX_POD_MESHID: u32 = ssbo_binding!(POD_MeshID);

ethel::shader_glsl! {
    struct Debris > [460] {
        common {};

        unit ShaderKind::Vertex => [
            attribs {
                ethel::shader_glsl_attribs! {
                    output fs_world: vec3;
                    output fs_normal: vec3;
                    output fs_color: vec4;
                }
            };

            uniform {
                projection: mat4 => glam::Mat4;
                view: mat4 => glam::Mat4;
            };

            type {
                commons::TYPE_MESH_METADATA
                commons::TYPE_MESH_VERTEX
            };

            ssbo {
                ethel::mesh::GLSL_SSBO_INTEGRATION[0]
                ethel::mesh::GLSL_SSBO_INTEGRATION[1]

                ethel::shader_glsl_ssbo! {
                    buf POD_Positions => {
                        [dyn_array vec4: pod_positions]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf POD_Rotations => {
                        [dyn_array vec4: pod_rotations]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf POD_MeshID => {
                        [dyn_array uint: pod_mesh_id]
                    }
                }
            };

            lib {
                commons::LIB_QUAT_CONVERT_MAT;
                commons::LIB_QUAT_MUL_QUAT;
                commons::LIB_QUAT_ROT_VEC;
            };

            src() "
                // account for degenerate 0
                uint debris_id = gl_InstanceID + 1;

                uint mesh_id = pod_mesh_id[debris_id];
                Metadata metadata = metadata[mesh_id];
                uint offset = metadata.offset;
                uint index = offset + gl_VertexID;
                Vertex vertex = vertex_storage[index];
                vec3 model = vertex.position.xyz;
                vec3 normal = normalize(vertex.normal.xyz);

                vec3 position = pod_positions[debris_id].xyz;
                vec4 rotation = pod_rotations[debris_id];

                vec3 local = rotateQuat(model, rotation);
                vec4 world = vec4(position + local, 1.0);

                fs_world = world.xyz;
                fs_color = vec4(vec3(0.8), 1.0);

                mat3 rot_m = quatToMat(rotation);
                fs_normal = normalize(normal * transpose(inverse(rot_m)));

                gl_Position = projection * view * world;
            "
        ];

        unit ShaderKind::Pixel => [
            attribs {
                commons::ATTRIBS_PIXEL_MINIMAL
            };

            uniform {
                camera_forward: vec3 => glam::Vec3;
            };

            const {
                commons::CONST_AMBIENT_LIGHT
            };

            src() "
            vec4 albedo = fs_color;

            if (albedo.a < 0.1) {
                discard;
            }

            vec3 normal = fs_normal;

            // basic directional light (camera source)
            vec3 light_dir = -camera_forward;
            float diffuse = dot(light_dir, normal);
            diffuse *= diffuse;

            float light_factor = LIGHT_AMBIENT + diffuse;

            outColor = vec4(fs_color.rgb * light_factor, fs_color.a);
            "
        ];
    }
}
