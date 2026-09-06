use ethel::{
    data::DirectIndex,
    render::buffer::{PartitionedTriBuffer, TriBuffer},
    shader::GlslStorage,
};
use rendrs::{geometry::DomainData, graphics::material::MaterialLocationRegistry};

use crate::{
    data::{CagePartitionedBuffer, FRAGMENTS_STORAGE_PARTS},
    render::shader_commons,
};

pub fn geom_fragments_pass() -> FragmentsGeomPass {
    geom_fragments_pass_with_shader(ComputeShaderFragmentsGeomSubmit::new_compiled())
}

pub fn geom_fragments_pass_with_shader(
    shader: ComputeShaderFragmentsGeomSubmit,
) -> FragmentsGeomPass {
    FragmentsGeomPass::new(shader, [], [], |section, _shader, ctx, out| {
        let frag_count = ctx.frag_count;
        let FragmentsGeomCtx {
            cages_data,
            cages_map,
            fragments_data,
            //material_registry,
            ..
        } = ctx;

        let section = section.as_index();

        cages_data.bind_ssbo_pod_points(Some(G_FRAGS_SSBO_BIND_POD_CAGES_LPOINTS));
        cages_data.bind_ssbo_pod_points_bind(Some(G_FRAGS_SSBO_BIND_POD_CAGES_LPOINTS_BIND));
        cages_data.bind_ssbo_pod_bindref(Some(G_FRAGS_SSBO_BIND_POD_CAGES_BINDREF));
        cages_map.bind_shader_storage(section, G_FRAGS_SSBO_BIND_IMAP_CAGES, 0);
        fragments_data.bind_shader_storage(section);

        let mut i = 0;
        while i < frag_count {
            out.write(DomainData::new(0, i, 64));
            i += 2;
        }
    })
}

macro_rules! ssbo_binding {
    (POD_BindPose) => {
        5
    };
    (POD_MeshID) => {
        6
    };
    (POD_CageID) => {
        7
    };

    (POD_Cages_LPoints) => {
        8
    };
    (POD_Cages_LPoints_Bind) => {
        9
    };
    (POD_Cages_BindRef) => {
        12
    };
    (IMap_Cages) => {
        13
    };
}

pub const G_FRAGS_SSBO_BIND_POD_BINDPOSE: u32 = ssbo_binding!(POD_BindPose);
pub const G_FRAGS_SSBO_BIND_POD_MESHID: u32 = ssbo_binding!(POD_MeshID);
pub const G_FRAGS_SSBO_BIND_POD_CAGEID: u32 = ssbo_binding!(POD_CageID);
pub const G_FRAGS_SSBO_BIND_POD_CAGES_LPOINTS: u32 = ssbo_binding!(POD_Cages_LPoints);
pub const G_FRAGS_SSBO_BIND_POD_CAGES_LPOINTS_BIND: u32 = ssbo_binding!(POD_Cages_LPoints_Bind);
pub const G_FRAGS_SSBO_BIND_POD_CAGES_BINDREF: u32 = ssbo_binding!(POD_Cages_BindRef);
pub const G_FRAGS_SSBO_BIND_IMAP_CAGES: u32 = ssbo_binding!(IMap_Cages);

rendrs::geometry_submission_job! {
    Fragments => {
        type {
            shader_commons::ETH_TYPE_MESH_METADATA
            shader_commons::ETH_TYPE_MESH_VERTEX
            shader_commons::ETH_TYPE_MESH_TRIANGLE
            shader_commons::ETH_TYPE_INDEX_INDIRECT
            shader_commons::ETH_TYPE_INDEX_DIRECT
        }
        ssbo {
            shader_commons::ETH_MESH_SSBO_STATIC // bind 10
            shader_commons::ETH_MESH_SSBO_TRIS   // bind 11

            G_FRAGS_SSBO_POD_BINDPOSE
            G_FRAGS_SSBO_POD_MESHID
            G_FRAGS_SSBO_POD_CAGEID
            G_FRAGS_SSBO_POD_CAGES_LPOINTS
            G_FRAGS_SSBO_POD_CAGES_LPOINTS_BIND
            G_FRAGS_SSBO_POD_CAGES_BINDREF
            G_FRAGS_SSBO_IMAP_CAGES
        }
        lib {
            shader_commons::LIB_MAT3_COFACTOR
        }
        share {
            uint sm_vert_base[2];
            uint sm_tris_base[2];

            vec3 sm_cage_pose[2];
            vec3 sm_cage_anchor[2];
            vec3 sm_cage_lpoint0[2][8];
            vec3 sm_cage_lpoint1[2][8];
            vec3 sm_cage_edges[2][12];
        }

        context {
            frag_count: u32;
            cages_data: CagePartitionedBuffer, for 'ctx;
            cages_map: TriBuffer<DirectIndex>, for 'ctx;
            fragments_data: PartitionedTriBuffer<{ FRAGMENTS_STORAGE_PARTS }>, for 'ctx;

            // currently unused
            material_registry: MaterialLocationRegistry, for 'ctx;
        }

        //todo: optimize ffd, share
        "
        const uint FRAG_DOMAIN = 32;

        // todo: decouple; geom_id is stored in triangle,
        // should be global, not frag-specific. oka for now
        uint fragment_id = rendrs_GeometryID;
        uint sub_domain = rendrs_DomainThreadID / FRAG_DOMAIN;
        fragment_id += sub_domain;

        uint mesh_id = pod_mesh_id[fragment_id];
        MeshMetadata metadata = eth_meshmeta[mesh_id];
        uint local_thread = rendrs_DomainThreadID % FRAG_DOMAIN;

        vec3 bind_pose = pod_bind_pose[fragment_id].xyz;
        IndirectIndex cage_id = pod_cage_id[fragment_id];
        DirectIndex cage_did = imap_cages[cage_id.index];
        uint cage_index = cage_did.index;
        vec4[8] localpoints = pod_cages_localpoints[cage_index];
        vec4[8] localpoints_bind = pod_cages_localpoints_bind[cage_index];
        vec3 cage_bindref = pod_cages_bindref[cage_index].xyz;

        uint m_tris_offset = metadata.tris_offset;
        uint m_tris_length = metadata.tris_length;
        uint m_vert_offset = metadata.vert_offset;
        uint m_vert_length = metadata.vert_length;

        if (local_thread == 0) {
            sm_vert_base[sub_domain] = AllocVertex(m_vert_length);
            sm_tris_base[sub_domain] = AllocTriangle(m_tris_length);

            IndirectIndex cage_id = pod_cage_id[fragment_id];
            DirectIndex cage_did = imap_cages[cage_id.index];
            uint cage_index = cage_did.index;

            sm_cage_pose[sub_domain]    = pod_bind_pose[fragment_id].xyz;
            sm_cage_anchor[sub_domain]  = pod_cages_bindref[cage_index].xyz;

            for (uint i = 0; i < 8; ++i) {
                sm_cage_lpoint0[sub_domain][i] = pod_cages_localpoints_bind[cage_index][i].xyz;
                sm_cage_lpoint1[sub_domain][i] = pod_cages_localpoints[cage_index][i].xyz;
            }

            sm_cage_edges[sub_domain]   = vec3[](
                sm_cage_lpoint1[sub_domain][1] - sm_cage_lpoint1[sub_domain][0],
                sm_cage_lpoint1[sub_domain][2] - sm_cage_lpoint1[sub_domain][0],
                sm_cage_lpoint1[sub_domain][4] - sm_cage_lpoint1[sub_domain][0],
                sm_cage_lpoint1[sub_domain][3] - sm_cage_lpoint1[sub_domain][2],
                sm_cage_lpoint1[sub_domain][3] - sm_cage_lpoint1[sub_domain][1],
                sm_cage_lpoint1[sub_domain][5] - sm_cage_lpoint1[sub_domain][1],
                sm_cage_lpoint1[sub_domain][5] - sm_cage_lpoint1[sub_domain][4],
                sm_cage_lpoint1[sub_domain][6] - sm_cage_lpoint1[sub_domain][4],
                sm_cage_lpoint1[sub_domain][6] - sm_cage_lpoint1[sub_domain][2],
                sm_cage_lpoint1[sub_domain][7] - sm_cage_lpoint1[sub_domain][6],
                sm_cage_lpoint1[sub_domain][7] - sm_cage_lpoint1[sub_domain][5],
                sm_cage_lpoint1[sub_domain][7] - sm_cage_lpoint1[sub_domain][3]
            );
        }

        groupMemoryBarrier();

        // anchor order is guaranteed to be:
        // 0: -x, -y, -z,
        // 1:  x, -y, -z,
        // 2: -x,  y, -z,
        // 3:  x,  y, -z,
        // 4: -x, -y,  z,
        // 5:  x, -y,  z,
        // 6: -x,  y,  z,
        // 7:  x,  y,  z,
        const uint THREAD_VERTEX_PRINT = 3;
        const uint THREAD_TRIANGLE_PRINT = 1;

        if (local_thread * THREAD_VERTEX_PRINT < m_vert_length) {
            // bind-time basis orthogonal matrix
            mat3 B = mat3(
                sm_cage_lpoint0[sub_domain][1] - sm_cage_lpoint0[sub_domain][0],
                sm_cage_lpoint0[sub_domain][2] - sm_cage_lpoint0[sub_domain][0],
                sm_cage_lpoint0[sub_domain][4] - sm_cage_lpoint0[sub_domain][0]
            );
            float B_det = determinant(B);
            mat3 B_inv = transpose(B); // B is orthogonal, transpose(B) = inverse(B)

            uint d_vert_base = max(local_thread - 1, 0) * THREAD_VERTEX_PRINT;
            uint d_vert_print_checked = min(m_vert_length - d_vert_base, THREAD_VERTEX_PRINT);
            for (uint vert = 0; vert < d_vert_print_checked; ++vert) {
                uint d_vert_i = d_vert_base + vert;
                uint m_vert_i = d_vert_i + m_vert_offset;
                MeshVertex m_vert = eth_vertex_buffer[m_vert_i];
                vec3 m_nor = vec3(m_vert.norm_x, m_vert.norm_y, m_vert.norm_z);
                vec3 m_pos = vec3(m_vert.pos_x, m_vert.pos_y, m_vert.pos_z);
                vec2 m_uv  = vec2(m_vert.uv_x, m_vert.uv_y);

                vec3 u_pos = m_pos + bind_pose; // fragment-local pos

                vec3 W = vec3(0.0);
                if (abs(B_det) > 1e-6)
                    W = B_inv * (u_pos - sm_cage_lpoint0[sub_domain][0]);

                vec3 rc00  = mix(sm_cage_lpoint1[sub_domain][0], sm_cage_lpoint1[sub_domain][1], W.x);
                vec3 rc01  = mix(sm_cage_lpoint1[sub_domain][4], sm_cage_lpoint1[sub_domain][5], W.x);
                vec3 rc10  = mix(sm_cage_lpoint1[sub_domain][2], sm_cage_lpoint1[sub_domain][3], W.x);
                vec3 rc11  = mix(sm_cage_lpoint1[sub_domain][6], sm_cage_lpoint1[sub_domain][7], W.x);
                vec3 rc0   = mix(rc00, rc10, W.y);
                vec3 rc1   = mix(rc01, rc11, W.y);
                vec3 delta = mix(rc0,  rc1,  W.z);

                vec3 w_pos = u_pos + delta;

                // derive normal (todo: rewrite, optimize)
                vec3 d_cage_edges[12] = sm_cage_edges[sub_domain];
                vec3 Tx = normalize(mix(
                    mix(d_cage_edges[0], d_cage_edges[3], W.y),
                    mix(d_cage_edges[6], d_cage_edges[9], W.y),
                    W.z
                ));
                vec3 Ty = normalize(mix(
                    mix(d_cage_edges[1], d_cage_edges[4],  W.x),
                    mix(d_cage_edges[7], d_cage_edges[10], W.x),
                    W.z
                ));
                vec3 Tz = normalize(mix(
                    mix(d_cage_edges[2], d_cage_edges[5],  W.x),
                    mix(d_cage_edges[8], d_cage_edges[11], W.x),
                    W.y
                ));
                vec3 T = Tx;
                vec3 B = normalize(Ty - dot(Ty, T) * T);
                vec3 N = cross(T, B);
                mat3 TBN_1 = mat3(T, B, N);
                mat3 TBN_0 = mat3(normalize(d_cage_edges[0]), normalize(d_cage_edges[1]), normalize(d_cage_edges[2]));
                mat3 TBN   = TBN_1 * transpose(TBN_0);
                mat3 TBN_a = cofactor3(TBN);

                vec3 w_nor = normalize(TBN_a * m_nor);
                vec3 w_tan = vec3(1.0, 0.0, 0.0); // todo

                VertexData(sm_vert_base[sub_domain] + d_vert_i, w_pos, w_nor, w_tan, m_uv);
            }
        }

        if (local_thread * THREAD_TRIANGLE_PRINT < m_tris_length) {
            // check unnecessary, print is 1
            //uint d_tris_base = max(local_thread - 1, 0) * THREAD_TRIANGLE_PRINT;
            //uint d_tris_print_checked = min(m_tris_length - d_tris_base, THREAD_TRIANGLE_PRINT);

            uint d_tri_i = local_thread * THREAD_TRIANGLE_PRINT;
            uint m_tri_i = m_tris_offset + d_tri_i;

            MeshTriangle m_tri = eth_tris_buffer[m_tri_i];
            uint t_v0 = m_tri.v0 - m_vert_offset;
            uint t_v1 = m_tri.v1 - m_vert_offset;
            uint t_v2 = m_tri.v2 - m_vert_offset;
            TriangleData(sm_tris_base[sub_domain] + d_tri_i, uint[]( t_v0, t_v1, t_v2 ), fragment_id);
        }
        "
    }
}

pub const G_FRAGS_SSBO_POD_BINDPOSE: GlslStorage = ethel::shader_glsl_ssbo! {
    buf POD_BindPose => {
        [dyn_array vec4: pod_bind_pose]
    }
};
pub const G_FRAGS_SSBO_POD_MESHID: GlslStorage = ethel::shader_glsl_ssbo! {
    buf POD_MeshID => {
        [dyn_array uint: pod_mesh_id]
    }
};
pub const G_FRAGS_SSBO_POD_CAGEID: GlslStorage = ethel::shader_glsl_ssbo! {
    buf POD_CageID => {
        [dyn_array IndirectIndex: pod_cage_id]
    }
};
pub const G_FRAGS_SSBO_POD_CAGES_LPOINTS: GlslStorage = ethel::shader_glsl_ssbo! {
    buf POD_Cages_LPoints => {
        [dyn_array vec4: pod_cages_localpoints => each 8]
    }
};
pub const G_FRAGS_SSBO_POD_CAGES_LPOINTS_BIND: GlslStorage = ethel::shader_glsl_ssbo! {
    buf POD_Cages_LPoints_Bind => {
        [dyn_array vec4: pod_cages_localpoints_bind => each 8]
    }
};
pub const G_FRAGS_SSBO_POD_CAGES_BINDREF: GlslStorage = ethel::shader_glsl_ssbo! {
    buf POD_Cages_BindRef => {
        [dyn_array vec4: pod_cages_bindref]
    }
};
pub const G_FRAGS_SSBO_IMAP_CAGES: GlslStorage = ethel::shader_glsl_ssbo! {
    buf IMap_Cages => {
        [dyn_array DirectIndex: imap_cages]
    }
};
