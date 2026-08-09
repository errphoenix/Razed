use ethel::{
    data::DirectIndex,
    render::{
        buffer::{PartitionedTriBuffer, TriBuffer},
        command::{DrawArraysIndirectCommand, GpuCommandDispatch},
    },
    shader::{GlslUniform, ShaderKind, ShaderProgram},
};
use rendrs::{
    graphics::material::{MaterialGroup, MaterialLocationRegistry},
    pipeline::DrawPass,
};

use crate::{
    data::{CagePartitionedBuffer, FRAGMENTS_STORAGE_PARTS},
    render::shader_commons,
};

pub type FragmentsDrawPass = DrawPass<FragmentsDrawCtxWrapper, 1, 0>;

#[derive(Debug)]
pub struct FragmentsDrawCtx<'data> {
    pub cages_data: &'data CagePartitionedBuffer,
    pub cages_map: &'data TriBuffer<DirectIndex>,
    pub fragments_data: &'data PartitionedTriBuffer<{ FRAGMENTS_STORAGE_PARTS }>,
    pub fragments_commands: &'data TriBuffer<DrawArraysIndirectCommand>,

    pub material_registry: &'data MaterialLocationRegistry,
}

rendrs::context_wrapper!(for<'ctx> FragmentsDrawCtx);

pub const fn pass(shader: &ShaderFragment, dev_materials: &MaterialGroup) -> FragmentsDrawPass {
    let handle_view = shader.handle().view();
    FragmentsDrawPass::new(
        handle_view,
        [dev_materials.sampler()],
        [],
        |section, ctx| {
            let section = section.as_index();

            ctx.cages_data
                .bind_ssbo_pod_points(Some(SSBO_INDEX_POD_CAGES_LOCALPOINTS));
            ctx.cages_data
                .bind_ssbo_pod_points_bind(Some(SSBO_INDEX_POD_CAGES_LOCALPOINTS_BIND));
            ctx.cages_data
                .bind_ssbo_pod_bindref(Some(SSBO_INDEX_POD_CAGES_BINDREF));

            ctx.cages_map
                .bind_shader_storage(section, SSBO_INDEX_IMAP_CAGES, 0);

            ctx.fragments_data.bind_shader_storage(section);
            // SAFETY: safe access to the commands buffer is guaranteed by the
            // correct triple-buffer section index
            let commands = unsafe { ctx.fragments_commands.view_section(section) };
            GpuCommandDispatch::from_view(commands).dispatch();
        },
    )
}

macro_rules! ssbo_binding {
    (POD_BindPose) => {
        0
    };
    (POD_MeshID) => {
        1
    };
    (POD_CageID) => {
        2
    };

    (POD_Cages_LocalPoints) => {
        3
    };
    (POD_Cages_LocalPoints_Bind) => {
        4
    };
    (POD_Cages_BindRef) => {
        5
    };

    (IMap_Cages) => {
        6
    };
}

pub const SSBO_INDEX_POD_BINDPOSE: u32 = ssbo_binding!(POD_BindPose);
pub const SSBO_INDEX_POD_MESHID: u32 = ssbo_binding!(POD_MeshID);
pub const SSBO_INDEX_POD_CAGEID: u32 = ssbo_binding!(POD_CageID);
pub const SSBO_INDEX_POD_CAGES_LOCALPOINTS: u32 = ssbo_binding!(POD_Cages_LocalPoints);
pub const SSBO_INDEX_POD_CAGES_LOCALPOINTS_BIND: u32 = ssbo_binding!(POD_Cages_LocalPoints_Bind);
pub const SSBO_INDEX_POD_CAGES_BINDREF: u32 = ssbo_binding!(POD_Cages_BindRef);
pub const SSBO_INDEX_IMAP_CAGES: u32 = ssbo_binding!(IMap_Cages);

use ShaderFragmentVariants::*;

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

            variants {
                WindowedAttenuation;
            };
        };

        unit ShaderKind::Pixel => [
            attribs {
                shader_commons::ATTRIBS_PIXEL_MINIMAL
            };

            uniform {
                length 1, camera_forward: vec3 => glam::Vec3;
                length 1, camera_position: vec3 => glam::Vec3;
                length 16, texture_map: sampler2DArray => i32;
            };

            type {
                rendrs::graphics::material::shader::TYPE_MATERIAL_ENTRY_LOCATION
                rendrs::graphics::material::shader::TYPE_MATERIAL_LOCATION
            };

            const {
                shader_commons::CONST_AMBIENT_LIGHT
            };

            lib {
                rendrs::pack::DERIVE_COTANGENT;
                >Default => rendrs::graphics::light::LIB_LIGHT_ATTENUATE_DISTANCE_FALLOFF;
                >WindowedAttenuation => rendrs::graphics::light::LIB_LIGHT_ATTENUATE_ISQ_WINDOWED_CURVE;
            };

            src() {
                "
                const uint DEV_MATERIAL_GROUP = 0;
                const float DIFFUSE_ALPHA_PAGE = 6.0;
                const float NORMAL_EMISSIVE_PAGE = 7.0;
                const float ORMD_PAGE = 8.0;
                const float UV_SCALE = 0.35;

                vec2 scaled_uv = fs_uv * UV_SCALE;

                vec4 qDiffuseAlpha = texture(
                    texture_map[DEV_MATERIAL_GROUP],
                    vec3(scaled_uv, DIFFUSE_ALPHA_PAGE)
                );
                vec4 qNormalEmissive = texture(
                    texture_map[DEV_MATERIAL_GROUP],
                    vec3(scaled_uv, NORMAL_EMISSIVE_PAGE)
                );
                vec4 qOrmd = texture(
                    texture_map[DEV_MATERIAL_GROUP],
                    vec3(scaled_uv, ORMD_PAGE)
                );

                //vec3 diffuse = qDiffuseAlpha.rgb;
                vec3 diffuse = vec3(0.725);
                float alpha = qDiffuseAlpha.a;
                vec3 normalMap = qNormalEmissive.rgb;
                float emissive = qNormalEmissive.a;
                float occlusion = qOrmd.r;
                float roughness = qOrmd.g;
                float metallic = qOrmd.b;
                float displacement = qOrmd.a;

                if (alpha < 0.1) {
                    discard;
                }

                mat3 TBN = deriveCotangent(fs_normal, fs_world, fs_uv);
                normalMap = normalMap * 2.0 - 1.0;
                vec3 normal = normalize(TBN * normalMap);

                // camera source point light
                const float LIGHT_MAX_DIST = 128.0;
                vec3 light_dir = camera_position - fs_world;
                float light_d = max(dot(normalize(light_dir), normal), 0.0);

            ";
            match {
                _ => {
                    "
                    float light_dist = length(light_dir);
                    light_d *= lightAttenuate(light_dist, LIGHT_MAX_DIST);
                    ";
                };
                WindowedAttenuation => {
                    "
                    float light_dist_sq = dot(light_dir, light_dir);
                    float light_dist = sqrt(light_dist_sq);
                    light_d *= lightAttenuate(light_dist_sq, light_dist, LIGHT_MAX_DIST, 0.01);
                    ";
                };
            }
                "

                float L = LIGHT_AMBIENT + light_d;
                outColor = vec4(diffuse * L, 1.0);
                ";
            }
        ];

        unit ShaderKind::Vertex => [
            attribs {
                ethel::shader_glsl_attribs! {
                    output fs_world: vec3;
                    output fs_normal: vec3;
                    output fs_uv: vec2;
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
                    buf POD_CageID => {
                        [dyn_array IndirectIndex: pod_cage_id]
                    }
                }

                ethel::shader_glsl_ssbo! {
                    buf POD_Cages_LocalPoints => {
                        [dyn_array vec4: pod_cages_localpoints => each 8]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf POD_Cages_LocalPoints_Bind => {
                        [dyn_array vec4: pod_cages_localpoints_bind => each 8]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf POD_Cages_BindRef => {
                        [dyn_array vec4: pod_cages_bindref]
                    }
                }

                ethel::shader_glsl_ssbo! {
                    buf IMap_Cages => {
                        [dyn_array DirectIndex: imap_cages]
                    }
                }
            };

            lib {
                shader_commons::LIB_MAT3_COFACTOR;
            };

            src() {
                "
                // account for degenerate 0
                uint fragment_id = gl_DrawID + 1;

                uint mesh_id = pod_mesh_id[fragment_id];
                Metadata metadata = metadata[mesh_id];
                uint offset = metadata.offset;
                uint index = offset + gl_VertexID;
                Vertex vertex = vertex_storage[index];
                vec3 model = vec3(
                    vertex.pos_x,
                    vertex.pos_y,
                    vertex.pos_z
                );
                vec3 normal = vec3(
                    vertex.norm_x,
                    vertex.norm_y,
                    vertex.norm_z
                );
                // pass uv to pixel shader
                fs_uv = vec2(
                    vertex.uv_x,
                    vertex.uv_y
                );

                // fragment bind pos
                vec3 bind_pose = pod_bind_pose[fragment_id].xyz;
                vec3 w_rest = bind_pose + model; // bind vertex

                IndirectIndex cage_id = pod_cage_id[fragment_id];
                DirectIndex cage_did = imap_cages[cage_id.index];
                uint cage_index = cage_did.index;

                vec4[8] localpoints = pod_cages_localpoints[cage_index];
                vec4[8] localpoints_bind = pod_cages_localpoints_bind[cage_index];
                vec3 cage_bindref = pod_cages_bindref[cage_index].xyz;

                // anchor order is guaranteed to be:
                // 0: -x, -y, -z,
                // 1:  x, -y, -z,
                // 2: -x,  y, -z,
                // 3:  x,  y, -z,
                // 4: -x, -y,  z,
                // 5:  x, -y,  z,
                // 6: -x,  y,  z,
                // 7:  x,  y,  z,

                // bind-time localpoints
                vec3 b000 = localpoints_bind[0].xyz;
                vec3 b100 = localpoints_bind[1].xyz;
                vec3 b010 = localpoints_bind[2].xyz;
                vec3 b110 = localpoints_bind[3].xyz;
                vec3 b001 = localpoints_bind[4].xyz;
                vec3 b101 = localpoints_bind[5].xyz;
                vec3 b011 = localpoints_bind[6].xyz;
                vec3 b111 = localpoints_bind[7].xyz;
                // real-time positions
                vec3 p000 = localpoints[0].xyz;
                vec3 p100 = localpoints[1].xyz;
                vec3 p010 = localpoints[2].xyz;
                vec3 p110 = localpoints[3].xyz;
                vec3 p001 = localpoints[4].xyz;
                vec3 p101 = localpoints[5].xyz;
                vec3 p011 = localpoints[6].xyz;
                vec3 p111 = localpoints[7].xyz;

                mat3 bind_matrix = mat3(
                    b100 - b000,
                    b010 - b000,
                    b001 - b000
                );
                float det = determinant(bind_matrix);
                vec3 uvw;
                if (abs(det) > 1e-6) {
                    uvw = inverse(bind_matrix) * (model - b000);
                } else {
                    uvw = vec3(0.0);
                }

                float ifx = uvw.x;
                float ify = uvw.y;
                float ifz = uvw.z;

                // real-time cage interpolation
                vec3 rc00 = mix(p000, p100, ifx);
                vec3 rc01 = mix(p001, p101, ifx);
                vec3 rc10 = mix(p010, p110, ifx);
                vec3 rc11 = mix(p011, p111, ifx);
                vec3 rc0 = mix(rc00, rc10, ify);
                vec3 rc1 = mix(rc01, rc11, ify);
                vec3 local_deformed = mix(rc0, rc1, ifz);

                vec4 world = vec4(bind_pose + local_deformed, 1.0);

                // derive normal
                vec3 e_x0 = p100 - p000;
                vec3 e_x1 = p110 - p010;
                vec3 e_x2 = p101 - p001;
                vec3 e_x3 = p111 - p011;
                vec3 e_y0 = p010 - p000;
                vec3 e_y1 = p110 - p100;
                vec3 e_y2 = p011 - p001;
                vec3 e_y3 = p111 - p101;
                vec3 e_z0 = p001 - p000;
                vec3 e_z1 = p101 - p100;
                vec3 e_z2 = p011 - p010;
                vec3 e_z3 = p111 - p110;
                vec3 tx = normalize(
                    mix(
                        mix(e_x0, e_x1, ify),
                        mix(e_x2, e_x3, ify),
                        ifz
                    )
                );
                vec3 ty = normalize(
                    mix(
                        mix(e_y0, e_y1, ifx),
                        mix(e_y2, e_y3, ifx),
                        ifz
                    )
                );
                vec3 tz = normalize(
                    mix(
                        mix(e_z0, e_z1, ifx),
                        mix(e_z2, e_z3, ifx),
                        ify
                    )
                );

                vec3 tangent = tx;
                vec3 bitangent = normalize(ty - dot(ty, tangent) * tangent);
                vec3 normal_tb = cross(tangent, bitangent);

                mat3 tbn_local = mat3(tangent, bitangent, normal_tb);
                mat3 tbn_bind = mat3(normalize(b100-b000),normalize(b010-b000),normalize(b001-b000));
                mat3 F = tbn_local * inverse(tbn_bind);
                vec3 d_normal = normalize(cofactor3(F) * normal);

                fs_world = world.xyz;
                fs_normal = d_normal;

                gl_Position = projection * view * world;
                ";
            }
        ];
    }
}
