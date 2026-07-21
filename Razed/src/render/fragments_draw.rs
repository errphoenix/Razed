use ethel::{
    render::{
        buffer::{PartitionedTriBuffer, TriBuffer},
        command::{DrawArraysIndirectCommand, GpuCommandDispatch},
    },
    shader::{GlslUniform, ShaderKind, ShaderProgram},
};
use rendrs::pipeline::DrawPass;

use crate::{data, render::shader_commons};

pub type FragmentsDrawPass = DrawPass<FragmentsDrawCtxWrapper, 0, 0>;

#[derive(Debug)]
pub struct FragmentsDrawCtx<'data> {
    pub fragments_data: &'data PartitionedTriBuffer<{ data::FRAGMENTS_STORAGE_PARTS }>,
    pub fragments_commands: &'data TriBuffer<DrawArraysIndirectCommand>,
}

rendrs::context_wrapper!(for<'ctx> FragmentsDrawCtx);

pub const fn pass(shader: &ShaderFragment) -> FragmentsDrawPass {
    let handle_view = shader.handle().view();
    FragmentsDrawPass::new(handle_view, [], [], |section, ctx| {
        let section = section.as_index();
        ctx.fragments_data.bind_shader_storage(section);
        let commands = ctx.fragments_commands.view_section(section);
        GpuCommandDispatch::from_view(commands).dispatch();
    })
}

macro_rules! ssbo_binding {
    (POD_Anchors) => {
        0
    };
    (POD_BindPose) => {
        1
    };
    (POD_MeshID) => {
        2
    };
    (IMap_Deforms) => {
        5
    };
    (POD_Deforms_Positions) => {
        6
    };
    (POD_Deforms_BindPose) => {
        7
    };
}

pub const SSBO_INDEX_POD_ANCHORS: u32 = ssbo_binding!(POD_Anchors);
pub const SSBO_INDEX_POD_BINDPOSE: u32 = ssbo_binding!(POD_BindPose);
pub const SSBO_INDEX_POD_MESHID: u32 = ssbo_binding!(POD_MeshID);
pub const SSBO_INDEX_IMAP_DEFORMS: u32 = ssbo_binding!(IMap_Deforms);
pub const SSBO_INDEX_POD_DEFORMS_POSITIONS: u32 = ssbo_binding!(POD_Deforms_Positions);
pub const SSBO_INDEX_POD_DEFORMS_BINDPOSE: u32 = ssbo_binding!(POD_Deforms_BindPose);

ethel::shader_glsl! {
    struct Fragment > [460] {
        common {
            type {
                shader_commons::TYPE_MESH_METADATA
                shader_commons::TYPE_MESH_VERTEX
            };

            ssbo {
                ethel::mesh::GLSL_SSBO_INTEGRATION[0]
                ethel::mesh::GLSL_SSBO_INTEGRATION[1]
            };
        };

        unit ShaderKind::Pixel => [
            attribs {
                shader_commons::ATTRIBS_PIXEL_MINIMAL
            };

            uniform {
                length 1, camera_forward: vec3 => glam::Vec3;
            };

            const {
                shader_commons::CONST_AMBIENT_LIGHT
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
                shader_commons::TYPE_INDEX_INDIRECT
                shader_commons::TYPE_INDEX_DIRECT
            };

            ssbo {
                ethel::shader_glsl_ssbo! {
                    buf POD_Anchors => {
                        [dyn_array IndirectIndex: pod_anchors => each 8]
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

            lib {
                ethel::shader_glsl_lib! {
                    mat3 cofactor3 [ m: mat3 ] => "
                        vec3 a = m[0];
                        vec3 b = m[1];
                        vec3 c = m[2];
                        return mat3(
                            cross(b, c),
                            cross(c, a),
                            cross(a, b)
                        );
                    "
                };
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

            // anchor order is guaranteed to be:
            // 0: -x, -y, -z,
            // 1:  x, -y, -z,
            // 2: -x,  y, -z,
            // 3:  x,  y, -z,
            // 4: -x, -y,  z,
            // 5:  x, -y,  z,
            // 6: -x,  y,  z,
            // 7:  x,  y,  z,

            // bind-time positions
            vec3 b000 = pod_deforms_pose[i0].xyz;
            vec3 b100 = pod_deforms_pose[i1].xyz;
            vec3 b010 = pod_deforms_pose[i2].xyz;
            vec3 b110 = pod_deforms_pose[i3].xyz;
            vec3 b001 = pod_deforms_pose[i4].xyz;
            vec3 b101 = pod_deforms_pose[i5].xyz;
            vec3 b011 = pod_deforms_pose[i6].xyz;
            vec3 b111 = pod_deforms_pose[i7].xyz;

            // real-time positions
            vec3 p000 = pod_deforms_positions[i0].xyz;
            vec3 p100 = pod_deforms_positions[i1].xyz;
            vec3 p010 = pod_deforms_positions[i2].xyz;
            vec3 p110 = pod_deforms_positions[i3].xyz;
            vec3 p001 = pod_deforms_positions[i4].xyz;
            vec3 p101 = pod_deforms_positions[i5].xyz;
            vec3 p011 = pod_deforms_positions[i6].xyz;
            vec3 p111 = pod_deforms_positions[i7].xyz;

            // determine AABB of deformation cage
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

            // axis-aligned interpolation factors
            float ifx = vdx / cdx;
            float ify = vdy / cdy;
            float ifz = vdz / cdz;

            // double cage trilinear interpolation:
            // - isolate 4 points by interpolating ifx
            // - isolate 2 points by interpolating ify
            // - isolate final point by interpolating ifz
            //
            // occurs for BIND cage, then REAL cage, to determine the delta
            // between bind-time and real-time states.
            //
            // the delta is then used to apply the final displacement on
            // the vertex, effectively applying the deformation.

            // bind-time cage interp.
            vec3 bc00 = mix(b000, b100, ifx);
            vec3 bc01 = mix(b001, b101, ifx);
            vec3 bc10 = mix(b010, b110, ifx);
            vec3 bc11 = mix(b011, b111, ifx);
            vec3 bc0 = mix(bc00, bc10, ify);
            vec3 bc1 = mix(bc01, bc11, ify);
            vec3 b_final = mix(bc0, bc1, ifz);

            // real-time cage interp.
            vec3 rc00 = mix(p000, p100, ifx);
            vec3 rc01 = mix(p001, p101, ifx);
            vec3 rc10 = mix(p010, p110, ifx);
            vec3 rc11 = mix(p011, p111, ifx);
            vec3 rc0 = mix(rc00, rc10, ify);
            vec3 rc1 = mix(rc01, rc11, ify);
            vec3 r_final = mix(rc0, rc1, ifz);

            vec3 displacement_delta = r_final - b_final;
            vec4 world = vec4(w_rest + displacement_delta, 1.0);

            // derive normal
            float b_idx = 1.0 / length(b100 - b000);
            float b_idy = 1.0 / length(b010 - b000);
            float b_idz = 1.0 / length(b001 - b000);
            vec3 bl_xy0 = mix(p100 - p000, p110 - p010, ify);
            vec3 bl_yx0 = mix(p010 - p000, p110 - p100, ifx);
            vec3 bl_zx0 = mix(p001 - p000, p101 - p100, ifx);
            vec3 bl_xy1 = mix(p101 - p001, p111 - p011, ify);
            vec3 bl_yx1 = mix(p011 - p001, p111 - p101, ifx);
            vec3 bl_zx1 = mix(p011 - p010, p111 - p110, ifx);
            vec3 dPds = mix(bl_xy0, bl_xy1, ifz);
            vec3 dPdt = mix(bl_yx0, bl_yx1, ifz);
            vec3 dPdu = mix(bl_zx0, bl_zx1, ify);
            mat3 J = mat3(dPds, dPdt, dPdu);
            mat3 F = J * mat3(
                vec3(b_idx, 0.0, 0.0),
                vec3(0.0, b_idy, 0.0),
                vec3(0.0, 0.0, b_idz)
            );
            vec3 d_normal = normalize(cofactor3(F) * normal);

            fs_world = world.xyz;
            fs_normal = d_normal;
            fs_color = vec4(vec3(0.8), 1.0);

            gl_Position = projection * view * world;
            "
        ];
    }
}
