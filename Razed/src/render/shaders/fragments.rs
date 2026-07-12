use super::commons;

use ethel::shader::{GlslUniform, ShaderKind};

macro_rules! ssbo_binding {
    (POD_Anchors) => {
        0
    };
    (POD_Weights) => {
        1
    };
    (POD_BindPose) => {
        2
    };
    (POD_MeshID) => {
        3
    };
    (IMap_Deforms) => {
        6
    };
    (POD_Deforms_Positions) => {
        7
    };
    (POD_Deforms_BindPose) => {
        8
    };
}

pub const SSBO_INDEX_POD_ANCHORS: u32 = ssbo_binding!(POD_Anchors);
pub const SSBO_INDEX_POD_WEIGHTS: u32 = ssbo_binding!(POD_Weights);
pub const SSBO_INDEX_POD_BINDPOSE: u32 = ssbo_binding!(POD_BindPose);
pub const SSBO_INDEX_POD_MESHID: u32 = ssbo_binding!(POD_MeshID);
pub const SSBO_INDEX_IMAP_DEFORMS: u32 = ssbo_binding!(IMap_Deforms);
pub const SSBO_INDEX_POD_DEFORMS_POSITIONS: u32 = ssbo_binding!(POD_Deforms_Positions);
pub const SSBO_INDEX_POD_DEFORMS_BINDPOSE: u32 = ssbo_binding!(POD_Deforms_BindPose);

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
                commons::ATTRIBS_PIXEL_MINIMAL
            };

            uniform {
                length 1, camera_forward: vec3 => glam::Vec3;
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

            outColor = vec4(fs_color.rgb * light_factor, 1.0);
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
                length 1, projection: mat4 => glam::Mat4;
                length 1, view: mat4 => glam::Mat4;
            };

            type {
                commons::TYPE_INDEX_INDIRECT
                commons::TYPE_INDEX_DIRECT
            };

            ssbo {
                ethel::shader_glsl_ssbo! {
                    buf POD_Anchors => {
                        [dyn_array IndirectIndex: pod_anchors => each 8]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf POD_Weights => {
                        [dyn_array vec4: pod_weights => each 2]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf POD_BindPose => {
                        [dyn_array vec4: pod_bind_pose]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf POD_MeshID => {
                        [dyn_array uint: pod_mesh_id]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf IMap_Deforms => {
                        [dyn_array IndirectIndex: imap_deforms]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf POD_Deforms_Positions => {
                        [dyn_array vec4: pod_deforms_positions]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf POD_Deforms_BindPose => {
                        [dyn_array vec4: pod_deforms_pose]
                    }
                }
            };

            src() "
            // account for degenerate 0
            uint fragment_id = gl_DrawID + 1;

            uint mesh_id = pod_mesh_id[fragment_id];
            Metadata metadata = metadata[mesh_id];
            uint offset = metadata.offset;
            uint index = offset + gl_VertexID;
            Vertex vertex = vertex_storage[index];
            vec3 model = vertex.position.xyz;
            vec3 normal = normalize(vertex.normal.xyz);

            // fragment bind pos
            vec3 bind_pose = pod_bind_pose[fragment_id].xyz;
            vec3 w_rest = bind_pose + model; // bind vertex

            IndirectIndex[8] anchors = pod_anchors[fragment_id];

            // gather anchor data (index, real pos, bind pos)
            uint i0 = imap_deforms[anchors[0].index].index;
            uint i1 = imap_deforms[anchors[1].index].index;
            uint i2 = imap_deforms[anchors[2].index].index;
            uint i3 = imap_deforms[anchors[3].index].index;
            uint i4 = imap_deforms[anchors[4].index].index;
            uint i5 = imap_deforms[anchors[5].index].index;
            uint i6 = imap_deforms[anchors[6].index].index;
            uint i7 = imap_deforms[anchors[7].index].index;
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

            // determine vertex relative to bind cage
            // todo: move to precompute, possibly
            const float M = 1000000.0;
            vec3 cage_min = vec3( M);
            vec3 cage_max = vec3(-M);
            for (int i = 0; i < 8; i++) {
                uint anchor_i = anchors[i].index;
                uint cage_imap = imap_deforms[anchor_i].index;
                vec3 point = pod_deforms_pose[cage_imap].xyz;

                cage_min.x = min(cage_min.x, point.x);
                cage_min.y = min(cage_min.y, point.y);
                cage_min.z = min(cage_min.z, point.z);
                cage_max.x = max(cage_max.x, point.x);
                cage_max.y = max(cage_max.y, point.y);
                cage_max.z = max(cage_max.z, point.z);
            }
            float cdx = cage_max.x - cage_min.x;
            float cdy = cage_max.y - cage_min.y;
            float cdz = cage_max.z - cage_min.z;
            float vdx = w_rest.x - cage_min.x;
            float vdy = w_rest.y - cage_min.y;
            float vdz = w_rest.z - cage_min.z;

            float ifx = vdx / cdx;
            float ify = vdy / cdy;
            float ifz = vdz / cdz;

            // anchor order is guaranteed to be:
            // 0: -x, -y, -z,
            // 1:  x, -y, -z,
            // 2: -x,  y, -z,
            // 3:  x,  y, -z,
            // 4: -x, -y,  z,
            // 5:  x, -y,  z,
            // 6: -x,  y,  z,
            // 7:  x,  y,  z,

            vec3 p000 = p0;
            vec3 p100 = p1;
            vec3 p010 = p2;
            vec3 p110 = p3;
            vec3 p001 = p4;
            vec3 p101 = p5;
            vec3 p011 = p6;
            vec3 p111 = p7;

            // interpolate x, isolate to 4 points
            vec3 p00 = mix(p000, p100, ifx);
            vec3 p10 = mix(p010, p110, ifx);
            vec3 p01 = mix(p001, p101, ifx);
            vec3 p11 = mix(p011, p111, ifx);
            // interpolate y, isolate to 2 points
            vec3 fp0 = mix(p00, p10, ify);
            vec3 fp1 = mix(p01, p11, ify);
            // interpolate z, isolate final point
            vec3 final_point = mix(fp0, fp1, ifz);

            vec4 world = vec4(final_point, 1.0);

            // float d0 = distance(w_rest, b0) + 0.000001;
            // float d1 = distance(w_rest, b1) + 0.000001;
            // float d2 = distance(w_rest, b2) + 0.000001;
            // float d3 = distance(w_rest, b3) + 0.000001;
            // float d4 = distance(w_rest, b4) + 0.000001;
            // float d5 = distance(w_rest, b5) + 0.000001;
            // float d6 = distance(w_rest, b6) + 0.000001;
            // float d7 = distance(w_rest, b7) + 0.000001;

            // float vw0 = 1.0 / d0;
            // float vw1 = 1.0 / d1;
            // float vw2 = 1.0 / d2;
            // float vw3 = 1.0 / d3;
            // float vw4 = 1.0 / d4;
            // float vw5 = 1.0 / d5;
            // float vw6 = 1.0 / d6;
            // float vw7 = 1.0 / d7;

            // float vwt = vw0 + vw1 + vw2 + vw3 + vw4 + vw5 + vw6 + vw7;
            // vw0 /= vwt;
            // vw1 /= vwt;
            // vw2 /= vwt;
            // vw3 /= vwt;
            // vw4 /= vwt;
            // vw5 /= vwt;
            // vw6 /= vwt;
            // vw7 /= vwt;

            // vec3 deform = vec3(0.0);
            // if (i0 != 0) deform += vw0 * (p0 - b0);
            // if (i1 != 0) deform += vw1 * (p1 - b1);
            // if (i2 != 0) deform += vw2 * (p2 - b2);
            // if (i3 != 0) deform += vw3 * (p3 - b3);
            // if (i4 != 0) deform += vw4 * (p4 - b4);
            // if (i5 != 0) deform += vw5 * (p5 - b5);
            // if (i6 != 0) deform += vw6 * (p6 - b6);
            // if (i7 != 0) deform += vw7 * (p7 - b7);

            // vec4 world = vec4(deform + w_rest, 1.0);
            // fs_world = world.xyz;
            // //fs_normal = mix(normal, normalize(abs(world.xyz)), 0.35);
            // fs_normal = normal;
            // fs_color = vec4(vec3(0.8), 1.0);

            fs_world = world.xyz;
            fs_normal = normal;
            fs_color = vec4(vec3(0.8), 1.0);

            gl_Position = projection * view * world;
            "
        ];
    }
}
