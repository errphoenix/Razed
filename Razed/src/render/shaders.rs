#![allow(unused_must_use)]

use ethel::shader::{Constant, GlslUniform, ShaderKind};

mod commons {
    use ethel::shader::{GlslLib, GlslStruct};

    ethel::shader_glsl_struct! {
        struct IndirectIndex {
            index: u32 => uint;
            generation: u32 => uint;
        }
    }

    ethel::shader_glsl_struct! {
        struct DirectIndex {
            index: u32 => uint;
            generation: u32 => uint;
        }
    }

    pub(super) const TYPE_MESH_METADATA: GlslStruct =
        ethel::mesh::MetadataGlslStruct::as_definition();
    pub(super) const TYPE_MESH_VERTEX: GlslStruct = ethel::mesh::VertexGlslStruct::as_definition();

    pub(super) const TYPE_INDEX_INDIRECT: GlslStruct = IndirectIndexGlslStruct::as_definition();
    pub(super) const TYPE_INDEX_DIRECT: GlslStruct = DirectIndexGlslStruct::as_definition();

    pub(super) const LIB_QUAT_CONVERT_MAT: GlslLib = ethel::shader_glsl_lib! {
        mat3 quatToMat [ q: vec4 ] => "
            mat3 m = mat3(0.0);

            float sqx = q.x * q.x;
            float sqy = q.y * q.y;
            float sqz = q.z * q.z;
            float sqw = q.w * q.w;

            float invs = 1.0 / (sqx + sqy + sqz + sqw);
            m[0][0] = (sqx - sqy - sqz + sqw) * invs;
            m[1][1] = (-sqx + sqy - sqz + sqw) * invs;
            m[2][2] = (-sqx - sqy + sqz + sqw) * invs;

            float tmp1 = q.x * q.y;
            float tmp2 = q.z * q.w;
            m[1][0] = 2.0 * (tmp1 + tmp2) * invs;
            m[0][1] = 2.0 * (tmp1 - tmp2) * invs;

            tmp1 = q.x * q.z;
            tmp2 = q.y * q.w;
            m[2][0] = 2.0 * (tmp1 - tmp2) * invs;
            m[0][2] = 2.0 * (tmp1 + tmp2) * invs;

            tmp1 = q.y * q.z;
            tmp2 = q.x * q.w;
            m[2][1] = 2.0 * (tmp1 + tmp2) * invs;
            m[1][2] = 2.0 * (tmp1 - tmp2) * invs;

            return m;
        "
    };

    pub(super) const LIB_QUAT_MUL_QUAT: GlslLib = ethel::shader_glsl_lib! {
        vec4 mulQuat [ q0: vec4, q1: vec4 ] => "
            vec4 r;
            r.x = (q0.w * q1.x) + (q0.x + q1.w) + (q0.y * q1.z) - (q0.z * q1.y);
            r.y = (q0.w * q1.y) - (q0.x * q1.z) + (q0.y * q1.w) + (q0.z * q1.x);
            r.z = (q0.w * q1.z) + (q0.x * q1.y) - (q0.y * q1.x) + (q0.z * q1.w);
            r.w = (q0.w * q1.w) - (q0.x * q1.x) - (q0.y * q1.y) - (q0.z * q1.z);
            return r;
        "
    };

    /// Depends on [`LIB_QUAT_MUL_QUAT`];
    pub(super) const LIB_QUAT_ROT_VEC: GlslLib = ethel::shader_glsl_lib! {
        vec3 rotateQuat [ p: vec3, q: vec4 ] => "
            vec4 q_conj = vec4(-q.x, -q.y, -q.z, q.w);
            vec4 p4 = vec4(p, 1.0);

            vec4 r mulQuat(q, p4);
            r = mulQuat(r, q_conj);
            return r.xyz;
        "
    };
}

mod base_pixel {
    use ethel::shader::{Constant, GlslAttribute};

    pub const ATTRIBS: GlslAttribute = ethel::shader_glsl_attribs! {
        input fs_world: vec3;
        input fs_normal: vec3;
        input fs_color: vec4;
        output outColor: vec4;
    };

    pub const CONST_AMBIENT_LIGHT: Constant<f32> = Constant::new("LIGHT_AMBIENT", 0.25);
}

mod lattice {
    use ethel::{shader::GlslStruct, state::data::IndirectIndex};

    ethel::shader_glsl_struct! {
        struct Constraint {
            nodes[2]: [IndirectIndex; 2] => IndirectIndex;
        }
    }

    pub const TYPE_CONSTRAINT: GlslStruct = ConstraintGlslStruct::as_definition();
}

ethel::shader_glsl! {
    struct Fragment > [460] {
        common {
            type {
                commons::TYPE_MESH_METADATA
                commons::TYPE_MESH_VERTEX
            };

            ssbo {
                ethel::mesh::GLSL_SSBO_INTEGRATION[0]
                ethel::mesh::GLSL_SSBO_INTEGRATION[1]
            };
        };

        unit ShaderKind::Pixel => [
            attribs {
                base_pixel::ATTRIBS
            };

            uniform {
                camera_forward: vec3 => glam::Vec3;
            };

            const {
                base_pixel::CONST_AMBIENT_LIGHT
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
                commons::TYPE_INDEX_INDIRECT
                commons::TYPE_INDEX_DIRECT
            };

            ssbo {
                ethel::shader_glsl_ssbo! {
                    buf POD_Anchors on 0 => {
                        [dyn_array IndirectIndex: pod_anchors => each 8]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf POD_Weights on 1 => {
                        [dyn_array vec4: pod_weights => each 2]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf POD_BindPose on 2 => {
                        [dyn_array vec4: pod_bind_pose]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf POD_MeshID on 3 => {
                        [dyn_array uint: pod_mesh_id]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf IMap_Deforms on 6 => {
                        [dyn_array IndirectIndex: imap_deforms]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf POD_Deforms_Positions on 7 => {
                        [dyn_array vec4: pod_deforms_positions]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf POD_Deforms_BindPose on 8 => {
                        [dyn_array vec4: pod_deforms_pose]
                    }
                }
            };

            src() "
            // account for degenerate 0
            uint fragment_id = gl_InstanceID + 1;

            uint mesh_id = pod_mesh_id[fragment_id];
            //uint mesh_id = 5;
            Metadata metadata = metadata[mesh_id];
            uint offset = metadata.offset;
            uint index = offset + gl_VertexID;
            Vertex vertex = vertex_storage[index];
            vec3 model = vertex.position.xyz;
            vec3 normal = normalize(vertex.normal.xyz);

            IndirectIndex[8] anchors = pod_anchors[fragment_id];
            vec4[2] weights = pod_weights[fragment_id];
            vec3 bind_pose = pod_bind_pose[fragment_id].xyz;

            // common ids and weights gather
            uint i0 = imap_deforms[anchors[0].index].index;
            uint i1 = imap_deforms[anchors[1].index].index;
            uint i2 = imap_deforms[anchors[2].index].index;
            uint i3 = imap_deforms[anchors[3].index].index;
            uint i4 = imap_deforms[anchors[4].index].index;
            uint i5 = imap_deforms[anchors[5].index].index;
            uint i6 = imap_deforms[anchors[6].index].index;
            uint i7 = imap_deforms[anchors[7].index].index;

            float w0 = weights[0].x;
            float w1 = weights[0].y;
            float w2 = weights[0].z;
            float w3 = weights[0].w;
            float w4 = weights[1].x;
            float w5 = weights[1].y;
            float w6 = weights[1].z;
            float w7 = weights[1].w;

            vec3 p0 = pod_deforms_positions[i0].xyz;
            vec3 p1 = pod_deforms_positions[i1].xyz;
            vec3 p2 = pod_deforms_positions[i2].xyz;
            vec3 p3 = pod_deforms_positions[i3].xyz;
            vec3 p4 = pod_deforms_positions[i4].xyz;
            vec3 p5 = pod_deforms_positions[i5].xyz;
            vec3 p6 = pod_deforms_positions[i6].xyz;
            vec3 p7 = pod_deforms_positions[i7].xyz;

            vec3 b0 = pod_deforms_pose[i0].xyz;
            vec3 b1 = pod_deforms_pose[i1].xyz;
            vec3 b2 = pod_deforms_pose[i2].xyz;
            vec3 b3 = pod_deforms_pose[i3].xyz;
            vec3 b4 = pod_deforms_pose[i4].xyz;
            vec3 b5 = pod_deforms_pose[i5].xyz;
            vec3 b6 = pod_deforms_pose[i6].xyz;
            vec3 b7 = pod_deforms_pose[i7].xyz;

            vec3 w_rest = bind_pose + model;

            float d0 = distance(w_rest, b0) + 0.000001;
            float d1 = distance(w_rest, b1) + 0.000001;
            float d2 = distance(w_rest, b2) + 0.000001;
            float d3 = distance(w_rest, b3) + 0.000001;
            float d4 = distance(w_rest, b4) + 0.000001;
            float d5 = distance(w_rest, b5) + 0.000001;
            float d6 = distance(w_rest, b6) + 0.000001;
            float d7 = distance(w_rest, b7) + 0.000001;

            const float RIGIDITY = 4.0;
            float vw0 = 1.0 / pow(d0, RIGIDITY);
            float vw1 = 1.0 / pow(d1, RIGIDITY);
            float vw2 = 1.0 / pow(d2, RIGIDITY);
            float vw3 = 1.0 / pow(d3, RIGIDITY);
            float vw4 = 1.0 / pow(d4, RIGIDITY);
            float vw5 = 1.0 / pow(d5, RIGIDITY);
            float vw6 = 1.0 / pow(d6, RIGIDITY);
            float vw7 = 1.0 / pow(d7, RIGIDITY);

            float vwt = vw0 + vw1 + vw2 + vw3 + vw4 + vw5 + vw6 + vw7;
            vw0 /= vwt;
            vw1 /= vwt;
            vw2 /= vwt;
            vw3 /= vwt;
            vw4 /= vwt;
            vw5 /= vwt;
            vw6 /= vwt;
            vw7 /= vwt;

            vec3 deform = vec3(0.0);
            if (i0 != 0) deform += vw0 * (p0 - b0);
            if (i1 != 0) deform += vw1 * (p1 - b1);
            if (i2 != 0) deform += vw2 * (p2 - b2);
            if (i3 != 0) deform += vw3 * (p3 - b3);
            if (i4 != 0) deform += vw4 * (p4 - b4);
            if (i5 != 0) deform += vw5 * (p5 - b5);
            if (i6 != 0) deform += vw6 * (p6 - b6);
            if (i7 != 0) deform += vw7 * (p7 - b7);

            vec4 world = vec4(deform + w_rest, 1.0);
            fs_world = world.xyz;
            fs_normal = mix(normal, normalize(abs(world.xyz)), 0.35);
            fs_color = vec4(vec3(0.8), 1.0);

            gl_Position = projection * view * world;
            "
        ];
    }
}

ethel::shader_glsl! {
    struct Lattice > [460] {
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

            type {
                commons::TYPE_INDEX_INDIRECT
                commons::TYPE_INDEX_DIRECT

                lattice::TYPE_CONSTRAINT
            };

            ssbo {
                ethel::shader_glsl_ssbo! {
                    buf POD_Constraints on 4 => {
                        [dyn_array Constraint: constraints]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf IMap_Nodes on 5 => {
                        [dyn_array IndirectIndex: imap_nodes]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf POD_Nodes on 6 => {
                        [dyn_array vec4: pod_nodes]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf I_Selected on 7 => {
                        DirectIndex: i_selected;
                    }
                }
            };

            src() "
                uint constraint_id = gl_InstanceID;
                uint node_offset = gl_VertexID;

                Constraint constraint = constraints[constraint_id];
                IndirectIndex node_id = constraint.nodes[node_offset];
                IndirectIndex node_ii = imap_nodes[node_id.index];

                fs_color = vec4(0.0, 1.0, 0.0, 0.28);
                if (constraint_id == i_selected.index) {
                    fs_color = vec4(1.0, 0.0, 0.0, 1.0);
                }

                vec3 position = pod_nodes[node_ii.index].xyz;
                gl_Position = projection * view * vec4(position, 1.0);
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
                ethel::shader_glsl_ssbo! {
                    buf POD_Positions on 0 => {
                        [dyn_array vec4: pod_positions]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf POD_Rotations on 1 => {
                        [dyn_array vec4: pod_rotations]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf POD_MeshID on 3 => {
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
                base_pixel::ATTRIBS
            };

            uniform {
                camera_forward: vec3 => glam::Vec3;
            };

            const {
                base_pixel::CONST_AMBIENT_LIGHT
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
