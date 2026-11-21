use super::LIB_GBANK_BUFFERS_HEADER;
use ethel::{
    TriBuffer,
    shader::{GlslLib, GlslStorage},
};
use rendrs::{geometry::DomainData, graphics::material::MaterialLocationRegistry};

use crate::render::{ViewData, geometry::FragmentsInstancedData, shader_commons};

pub fn geom_fragments_instanced_pass() -> FragmentsInstancedGeomPass {
    geom_fragments_instanced_pass_with_shader(
        ComputeShaderFragmentsInstancedGeomSubmit::new_compiled(),
    )
}

pub fn geom_fragments_instanced_pass_with_shader(
    shader: ComputeShaderFragmentsInstancedGeomSubmit,
) -> FragmentsInstancedGeomPass {
    FragmentsInstancedGeomPass::new(shader, [], [], |section, shader, ctx, domains| {
        let FragmentsInstancedGeomCtx {
            instances_count,
            data,
            map,
            view_data,
            inst_mesh_base,
            inst_mesh_count,
            dev_material,
            //material_registry,
            ..
        } = ctx;

        if *instances_count == 0 {
            return;
        }

        let section = section.as_index();

        match data {
            FragmentsInstancedData::Debris(debris) => {
                debris.bind_ssbo_position(section, Some(G_FRAGMENTS_INSTANCED_SSBOI_POSITION));
                debris.bind_ssbo_rotation(section, Some(G_FRAGMENTS_INSTANCED_SSBOI_ROTATION));
                debris.bind_ssbo_mesh_id(section, Some(G_FRAGMENTS_INSTANCED_SSBOI_MESH_ID));
                shader.uniform_ignore_rotation_bool(false);
                shader.uniform_is_indirect_bool(false);
            }
            FragmentsInstancedData::Fragments(fragments) => {
                if let Some(map) = map {
                    fragments
                        .bind_ssbo_bind_pose(section, Some(G_FRAGMENTS_INSTANCED_SSBOI_POSITION));
                    fragments.bind_ssbo_mesh_id(section, Some(G_FRAGMENTS_INSTANCED_SSBOI_MESH_ID));
                    map.bind_shader_storage(section, G_FRAGMENTS_INSTANCED_SSBOI_MAP, 0);

                    shader.uniform_ignore_rotation_bool(true);
                    shader.uniform_is_indirect_bool(true);
                }
            }
        }

        shader.uniform_camera_forward_vec3(view_data.view_dir);
        shader.uniform_camera_position_vec3(view_data.view_pos);
        shader.uniform_instances_count_uint(*instances_count);
        shader.uniform_inst_mesh_base_uint(*inst_mesh_base);
        shader.uniform_inst_mesh_count_uint(*inst_mesh_count);
        shader.uniform_dev_material_uint(*dev_material);

        let mut i = 0;
        while i < *inst_mesh_count {
            domains.write(DomainData::new(0, i, 64));
            i += 1;
        }
        domains.upload();
    })
}

// maybe optimize
// this is inefficient, but further optimization may not be strictly necessary
// as the gpu can likely brute-froce through the high element count
// todo: benchmarks, measurements
rendrs::geometry_submission_job! {
    FragmentsInstanced => {
        source {
            vertex => super::VertexBuffers;
            triangle => super::TriangleBuffers;
        };

        uniform {
            on 0 => camera_forward  : vec3 from glam::Vec3;
            on 1 => camera_position : vec3 from glam::Vec3;
            on 2 => instances_count : uint from u32;
            on 3 => inst_mesh_base  : uint from u32;
            on 4 => inst_mesh_count : uint from u32;
            on 5 => dev_material    : uint from u32;
            on 6 => ignore_rotation : bool from bool;
            on 7 => is_indirect     : bool from bool;
        }
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
            G_FRAGMENTS_INSTANCED_SSBO_POSITION
            G_FRAGMENTS_INSTANCED_SSBO_ROTATION
            G_FRAGMENTS_INSTANCED_SSBO_MESH_ID
            G_FRAGMENTS_INSTANCED_SSBO_MAP
        }
        lib {
            INTERNAL_LIB_GET_INDEX
        }
        share {
            uint sm_inst_mesh_inst_count_thread[64];
            uint sm_inst_mesh_inst_count;
            uint sm_inst_base;
            uint sm_m_tris_length;
            uint sm_m_tris_base;
        }

        context {
            instances_count: u32;
            data: FragmentsInstancedData<'ctx>;
            map: Option<&'ctx TriBuffer<u32>>;

            view_data: ViewData;

            inst_mesh_base: u32;
            inst_mesh_count: u32;

            dev_material: u32;

            // currently unused
            material_registry: MaterialLocationRegistry, for 'ctx;
        }

        "
        #define DOMAIN_THREAD_SIZE 64

        const uint inst_mesh_id = inst_mesh_base + rendrs_WorkGroupID;

        if (rendrs_ThreadID == 0) {
            const MeshMetadata metadata = eth_meshmeta[inst_mesh_id];

            const uint m_vert_offset = metadata.vert_offset;
            const uint m_vert_length = metadata.vert_length;
            const uint m_tris_offset = metadata.tris_offset;
            sm_m_tris_length = metadata.tris_length;

            const uint m_vert_base = AllocVertex(m_vert_length);
            sm_m_tris_base = AllocTriangle(sm_m_tris_length);

            for (uint i = 0; i < m_vert_length; ++i) {
                const MeshVertex m_vert = eth_vertex_buffer[m_vert_offset + i];
                const vec3 m_nor = vec3(m_vert.norm_x, m_vert.norm_y, m_vert.norm_z);
                const vec3 m_pos = vec3(m_vert.pos_x, m_vert.pos_y, m_vert.pos_z);
                const vec2 m_uv  = vec2(m_vert.uv_x, m_vert.uv_y);
                VertexData(m_vert_base + i, m_pos, m_nor, m_uv);
            }
            for (uint i = 0; i < sm_m_tris_length; ++i) {
                MeshTriangle m_tri = eth_tris_buffer[m_tris_offset + i];
                const uint[3] indices = uint[](
                    m_tri.v0 - m_vert_offset + m_vert_base,
                    m_tri.v1 - m_vert_offset + m_vert_base,
                    m_tri.v2 - m_vert_offset + m_vert_base
                );
                TriangleData(sm_m_tris_base + i, indices, rendrs_GeometryID, dev_material);
            }
        }

        const uint q = instances_count / DOMAIN_THREAD_SIZE;
        const uint r = instances_count % DOMAIN_THREAD_SIZE;
        const uint thread_base = rendrs_ThreadID * q + min(rendrs_ThreadID, r);
        const uint thread_this = q + uint(rendrs_ThreadID < r);
        const uint thread_end = thread_base + thread_this;

        uint thread_inst_count = 0;
        for (uint i = thread_base; i < thread_end; ++i) {
            const uint I = internal_GetIndex(i + 1);
            uint mesh_id = fi_mesh_id[I];
            if (mesh_id == inst_mesh_id) {
                thread_inst_count += 1;
            }
        }
        sm_inst_mesh_inst_count_thread[rendrs_ThreadID] = thread_inst_count;

        barrier();

        if (rendrs_ThreadID == 0) {
            sm_inst_mesh_inst_count = 0;
            for (uint i = 0; i < DOMAIN_THREAD_SIZE; ++i) {
               sm_inst_mesh_inst_count += sm_inst_mesh_inst_count_thread[i];
            }

            sm_inst_base = AllocInstances(sm_inst_mesh_inst_count);

            AllocInstanceListData(rendrs_GeometryID,
                sm_m_tris_base, sm_m_tris_length,
                sm_inst_base, sm_inst_mesh_inst_count
            );
        }

        barrier();

        uint j = 0;
        for (uint k = 0; k < rendrs_ThreadID; ++k) {
            j += sm_inst_mesh_inst_count_thread[k];
        }

        for (uint i = thread_base; i < thread_end; ++i) {
            const uint I = internal_GetIndex(i + 1);
            uint mesh_id = fi_mesh_id[I];
            if (mesh_id == inst_mesh_id) {
                vec3 position = fi_position[I].xyz;

                vec4 rotation;
                if (ignore_rotation)
                    rotation = vec4(0.0, 0.0, 0.0, 1.0);
                else
                    rotation = fi_rotation[I];

                InstanceDataTransform(sm_inst_base + j, position, rotation);
                j++;
            }
        }
        "
    }
}

macro_rules! ssbo_binding {
    (Position) => {
        5
    };
    (Rotation) => {
        6
    };
    (Mesh_ID) => {
        7
    };
    (Map) => {
        8
    };
}

pub const G_FRAGMENTS_INSTANCED_SSBOI_POSITION: u32 = ssbo_binding!(Position);
pub const G_FRAGMENTS_INSTANCED_SSBOI_ROTATION: u32 = ssbo_binding!(Rotation);
pub const G_FRAGMENTS_INSTANCED_SSBOI_MESH_ID: u32 = ssbo_binding!(Mesh_ID);
pub const G_FRAGMENTS_INSTANCED_SSBOI_MAP: u32 = ssbo_binding!(Map);

pub const G_FRAGMENTS_INSTANCED_SSBO_POSITION: GlslStorage = ethel::shader_glsl_ssbo! {
    buf Position => {
        [dyn_array vec4 : fi_position]
    }
};
pub const G_FRAGMENTS_INSTANCED_SSBO_ROTATION: GlslStorage = ethel::shader_glsl_ssbo! {
    buf Rotation => {
        [dyn_array vec4 : fi_rotation]
    }
};
pub const G_FRAGMENTS_INSTANCED_SSBO_MESH_ID: GlslStorage = ethel::shader_glsl_ssbo! {
    buf Mesh_ID => {
        [dyn_array uint : fi_mesh_id]
    }
};
pub const G_FRAGMENTS_INSTANCED_SSBO_MAP: GlslStorage = ethel::shader_glsl_ssbo! {
    buf Map => {
        [dyn_array uint : fi_map]
    }
};

const INTERNAL_LIB_GET_INDEX: GlslLib = ethel::shader_glsl_lib! {
    uint internal_GetIndex [ thread : uint ] => "
        if (is_indirect) {
            return fi_map[thread];
        } else {
            return thread;
        }
    "
};
