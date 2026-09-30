use ethel::{render::buffer::PartitionedTriBuffer, shader::GlslStorage};
use rendrs::{geometry::DomainData, graphics::material::MaterialLocationRegistry};

use crate::{
    data::LayoutDebrisData,
    render::{ViewData, shader_commons},
};

pub fn geom_debris_pass() -> DebrisGeomPass {
    geom_debris_pass_with_shader(ComputeShaderDebrisGeomSubmit::new_compiled())
}

pub fn geom_debris_pass_with_shader(shader: ComputeShaderDebrisGeomSubmit) -> DebrisGeomPass {
    DebrisGeomPass::new(shader, [], [], |section, shader, ctx, domains| {
        let DebrisGeomCtx {
            debris_count,
            debris_data,
            view_data,
            inst_mesh_base,
            inst_mesh_count,
            //material_registry,
            ..
        } = ctx;

        if *debris_count == 0 {
            return;
        }

        let section = section.as_index();

        debris_data.bind_shader_storage_single(
            section,
            LayoutDebrisData::PodPositions as usize,
            Some(G_DEBRIS_SSBO_BIND_POD_POSITION),
        );
        debris_data.bind_shader_storage_single(
            section,
            LayoutDebrisData::PodRotations as usize,
            Some(G_DEBRIS_SSBO_BIND_POD_ROTATION),
        );
        debris_data.bind_shader_storage_single(
            section,
            LayoutDebrisData::PodMeshId as usize,
            Some(G_DEBRIS_SSBO_BIND_POD_MESHID),
        );

        shader.uniform_camera_forward_vec3v([view_data.view_dir]);
        shader.uniform_camera_position_vec3v([view_data.view_pos]);
        shader.uniform_debris_count_uintv([*debris_count]);
        shader.uniform_inst_mesh_base_uintv([*inst_mesh_base]);
        shader.uniform_inst_mesh_count_uintv([*inst_mesh_count]);

        let mut i = 0;
        while i < *inst_mesh_count {
            domains.write(DomainData::new(0, i, 64));
            i += 1;
        }
    })
}

// maybe optimize
// this is inefficient, but further optimization may not be strictly necessary
// as the gpu can likely brute-froce through the high element count
// todo: benchmarks, measurements
rendrs::geometry_submission_job! {
    Debris => {
        source {
            vertex => super::VertexBuffers;
            triangle => super::TriangleBuffers;
        };

        uniform {
            length 1, camera_forward: vec3 => glam::Vec3;
            length 1, camera_position: vec3 => glam::Vec3;
            length 1, debris_count: uint => u32;
            length 1, inst_mesh_base: uint => u32;
            length 1, inst_mesh_count: uint => u32;
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

            G_DEBRIS_SSBO_POD_POSITION
            G_DEBRIS_SSBO_POD_ROTATION
            G_DEBRIS_SSBO_POD_MESHID
        }
        share {
            uint sm_inst_mesh_inst_count_thread[64];
            uint sm_inst_mesh_inst_count;
        }

        context {
            debris_count: u32;
            debris_data: PartitionedTriBuffer<3>, for 'ctx;

            view_data: ViewData;

            inst_mesh_base: u32;
            inst_mesh_count: u32;

            // currently unused
            material_registry: MaterialLocationRegistry, for 'ctx;
        }

        "
        #define DOMAIN_THREAD_SIZE 64

        const uint inst_mesh_id = inst_mesh_base + rendrs_DomainIndex;

        uint m_tris_length;
        uint m_tris_base;
        if (rendrs_ThreadID == 0) {
            const MeshMetadata metadata = eth_meshmeta[inst_mesh_id];

            const uint m_vert_offset = metadata.vert_offset;
            const uint m_vert_length = metadata.vert_length;
            const uint m_tris_offset = metadata.tris_offset;
            m_tris_length = metadata.tris_length;

            const uint m_vert_base = AllocVertex(m_vert_length);
            m_tris_base = AllocTriangle(m_tris_length);

            for (uint i = 0; i < m_vert_length; ++i) {
                const MeshVertex m_vert = eth_vertex_buffer[m_vert_offset + i];
                const vec3 m_nor = vec3(m_vert.norm_x, m_vert.norm_y, m_vert.norm_z);
                const vec3 m_pos = vec3(m_vert.pos_x, m_vert.pos_y, m_vert.pos_z);
                const vec2 m_uv  = vec2(m_vert.uv_x, m_vert.uv_y);
                VertexData(m_vert_base + i, m_pos, m_nor, m_uv);
            }
            for (uint i = 0; i < m_tris_length; ++i) {
                MeshTriangle m_tri = eth_tris_buffer[m_tris_offset + i];
                const uint[3] indices = uint[]( m_tri.v0, m_tri.v1, m_tri.v2 );
                TriangleData(m_tris_base + i, indices, inst_mesh_id);
            }
        }

        uint thread_inst_count = 0;

        const uint thread_each = debris_count / DOMAIN_THREAD_SIZE;
        const uint thread_base = thread_each * rendrs_ThreadID;
        const uint thread_margin = debris_count % DOMAIN_THREAD_SIZE;
        const uint thread_end_bounds = thread_base + thread_each;

        for (uint i = thread_base; i < thread_end_bounds; ++i) {
            uint mesh_id = pod_mesh_id[i];
            if (mesh_id == inst_mesh_id) {
                thread_inst_count += 1;
            }
        }
        if (rendrs_ThreadID == DOMAIN_THREAD_SIZE - 1) {
            for (uint i = thread_end_bounds; i < thread_end_bounds + thread_margin; ++i) {
                uint mesh_id = pod_mesh_id[i];
                if (mesh_id == inst_mesh_id) {
                    thread_inst_count += 1;
                }
            }
        }
        sm_inst_mesh_inst_count_thread[rendrs_ThreadID] = thread_inst_count;

        barrier();

        uint i_base;
        if (rendrs_ThreadID == 0) {
            for (uint i = 0; i < DOMAIN_THREAD_SIZE; ++i) {
               sm_inst_mesh_inst_count += sm_inst_mesh_inst_count_thread[i];
            }

            i_base = AllocInstances(sm_inst_mesh_inst_count);

            AllocInstanceListData(inst_mesh_id,
                m_tris_base, m_tris_length,
                i_base, sm_inst_mesh_inst_count
            );
        }

        barrier();

        uint j = 0;
        for (uint i = thread_base; i < thread_end_bounds; ++i) {
            uint mesh_id = pod_mesh_id[i];
            vec3 position = pod_position[i].xyz;
            vec4 rotation = pod_rotation[i];
            if (mesh_id == inst_mesh_id) {
                InstanceDataTransform(i_base + j, position, rotation);
            }
            j++;
        }
        if (rendrs_ThreadID == DOMAIN_THREAD_SIZE - 1) {
            for (uint i = thread_end_bounds; i < thread_end_bounds + thread_margin; ++i) {
                uint mesh_id = pod_mesh_id[i];
                vec3 position = pod_position[i].xyz;
                vec4 rotation = pod_rotation[i];
                if (mesh_id == inst_mesh_id) {
                    InstanceDataTransform(i_base + j, position, rotation);
                }
                j++;
            }
        }

        "
    }
}

macro_rules! ssbo_binding {
    (POD_Position) => {
        5
    };
    (POD_Rotation) => {
        6
    };
    (POD_MeshID) => {
        7
    };
}

pub const G_DEBRIS_SSBO_BIND_POD_POSITION: u32 = ssbo_binding!(POD_Position);
pub const G_DEBRIS_SSBO_BIND_POD_ROTATION: u32 = ssbo_binding!(POD_Rotation);
pub const G_DEBRIS_SSBO_BIND_POD_MESHID: u32 = ssbo_binding!(POD_MeshID);

pub const G_DEBRIS_SSBO_POD_POSITION: GlslStorage = ethel::shader_glsl_ssbo! {
    buf POD_Position => {
        [dyn_array vec4: pod_position]
    }
};

pub const G_DEBRIS_SSBO_POD_ROTATION: GlslStorage = ethel::shader_glsl_ssbo! {
    buf POD_Rotation => {
        [dyn_array vec4: pod_rotation]
    }
};

pub const G_DEBRIS_SSBO_POD_MESHID: GlslStorage = ethel::shader_glsl_ssbo! {
    buf POD_MeshID => {
        [dyn_array uint: pod_mesh_id]
    }
};
