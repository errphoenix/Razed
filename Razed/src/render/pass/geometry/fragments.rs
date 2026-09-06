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
        fragment_id += rendrs_DomainThreadID / FRAG_DOMAIN;

        uint mesh_id = pod_mesh_id[fragment_id];
        MeshMetadata metadata = eth_meshmeta[mesh_id];
        uint local_thread = rendrs_DomainThreadID % FRAG_DOMAIN;

        const uint THREAD_VERTEX_PRINT = 6;
        const uint THREAD_TRIANGLE_PRINT = 2;

        if (local_thread * THREAD_VERTEX_PRINT >= metadata.length) {
            return; // todo: do not return
        }

        uint vi_base = metadata.offset + local_thread * THREAD_VERTEX_PRINT;
        MeshVertex vertex[THREAD_VERTEX_PRINT];
        for (uint i = 0; i < THREAD_VERTEX_PRINT; ++i) {
            vertex[i] = eth_vertex_buffer[vi_base + i];
        }

        vec3 bind_pose = pod_bind_pose[fragment_id].xyz;
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
        vec3 b000 = localpoints_bind[0].xyz; vec3 b100 = localpoints_bind[1].xyz;
        vec3 b010 = localpoints_bind[2].xyz; vec3 b110 = localpoints_bind[3].xyz;
        vec3 b001 = localpoints_bind[4].xyz; vec3 b101 = localpoints_bind[5].xyz;
        vec3 b011 = localpoints_bind[6].xyz; vec3 b111 = localpoints_bind[7].xyz;
        // real-time positions
        vec3 p000 = localpoints[0].xyz; vec3 p100 = localpoints[1].xyz;
        vec3 p010 = localpoints[2].xyz; vec3 p110 = localpoints[3].xyz;
        vec3 p001 = localpoints[4].xyz; vec3 p101 = localpoints[5].xyz;
        vec3 p011 = localpoints[6].xyz; vec3 p111 = localpoints[7].xyz;

        // bind-time basis orthogonal matrix
        vec3 bx = b100 - b000; vec3 by = b010 - b000; vec3 bz = b001 - b000;
        mat3 B = mat3(bx, by, bz);
        float B_det = determinant(B);
        mat3 B_inv = inverse(B);

        // normal derivation parameters (cage-local)
        vec3 e_x0 = bx; vec3 e_x1 = p110 - p010; vec3 e_x2 = p101 - p001; vec3 e_x3 = p111 - p011;
        vec3 e_y0 = by; vec3 e_y1 = p110 - p100; vec3 e_y2 = p011 - p001; vec3 e_y3 = p111 - p101;
        vec3 e_z0 = bz; vec3 e_z1 = p101 - p100; vec3 e_z2 = p011 - p010; vec3 e_z3 = p111 - p110;

        for (uint tri = 0; tri < THREAD_TRIANGLE_PRINT; ++tri) {
            uint base = tri * 3;
            MeshVertex v0_src = vertex[base];
            MeshVertex v1_src = vertex[base + 1];
            MeshVertex v2_src = vertex[base + 2];

            vec3 D_pos[3];
            vec3 D_nor[3];
            vec3 D_tan[3];
            for (uint i = 0; i < 3; ++i) {
                uint j = i + base;
                MeshVertex v_src = vertex[j];
                vec3 n_src = vec3(v_src.norm_x, v_src.norm_y, v_src.norm_z);
                vec3 p_src = vec3(v_src.pos_x, v_src.pos_y, v_src.pos_z);
                vec3 b_src = p_src + bind_pose;

                vec3 W = vec3(0.0);
                if (abs(B_det) > 1e-6)
                    W = B_inv * (b_src - b000);

                // real-time cage interpolation
                vec3 rc00  = mix(p000, p100, W.x);
                vec3 rc01  = mix(p001, p101, W.x);
                vec3 rc10  = mix(p010, p110, W.x);
                vec3 rc11  = mix(p011, p111, W.x);
                vec3 rc0   = mix(rc00, rc10, W.y);
                vec3 rc1   = mix(rc01, rc11, W.y);
                vec3 delta = mix(rc0,  rc1,  W.z);
                D_pos[i] = b_src + delta;

                // derive normal (todo: rewrite, optimize)
                vec3 Tx = normalize(mix(mix(e_x0, e_x1, W.y), mix(e_x2, e_x3, W.y), W.z));
                vec3 Ty = normalize(mix(mix(e_y0, e_y1, W.x), mix(e_y2, e_y3, W.x), W.z));
                vec3 Tz = normalize(mix(mix(e_z0, e_z1, W.x), mix(e_z2, e_z3, W.x), W.y));
                vec3 T  = Tx;
                vec3 B  = normalize(Ty - dot(Ty, T) * T);
                vec3 N  = cross(T, B);
                mat3 TBN_local = mat3(T, B, N);
                mat3 TBN_bind  = mat3(normalize(bx), normalize(by), normalize(bz));
                mat3 TBN       = TBN_local * inverse(TBN_bind);
                mat3 TBN_cof   = cofactor3(TBN);

                D_nor[i] = normalize(TBN_cof * n_src);
                D_tan[i] = vec3(0.0); //todo
            }

            Triangle(
                Vertex(D_pos[0], D_nor[0], D_tan[0], vec2(v0_src.uv_x, v0_src.uv_y)),
                Vertex(D_pos[1], D_nor[1], D_tan[1], vec2(v1_src.uv_x, v1_src.uv_y)),
                Vertex(D_pos[2], D_nor[2], D_tan[2], vec2(v2_src.uv_x, v2_src.uv_y)),
                fragment_id
            );
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
