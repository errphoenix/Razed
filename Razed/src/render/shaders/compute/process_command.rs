//! Command buffer processing & population
//!
//! The purpose of this compute shader is to map a contiguous vector/array of
//! [`MeshID`], which represents the mesh ID of an arbitrary group of
//! entities, each to its own dedicated indirect draw command; populating draw
//! count, vertex/element count, etc.
//!
//! The value of a [`MeshID`] is directly mapped to the contiguous command
//! buffer: a mesh of ID `I` will be mapped to the entry of the command buffer
//! at that same index `I`.
//!
//! Given that, the compute shader expects the command buffer to be
//! zero-initialised (all fields `0`) for atleast the length of the contiguous
//! [`MeshID`] collection.
//!
//! [`MeshID`]: ethel::mesh::Id

use ethel::shader::GlslStruct;

use crate::render::shaders::commons;

ethel::shader_glsl_struct! {
    struct DrawArraysIndirectCommand {
        count: u32 => uint;
        instance_count: u32 => uint;
        first_vertex: u32 => uint;
        base_instance: u32 => uint;
    }
}

ethel::shader_glsl_struct! {
    struct DrawElementsIndirectCommand {
        count: u32 => uint;
        instance_count: u32 => uint;
        first_vertex: u32 => uint;
        base_vertex: i32 => int;
        base_instance: u32 => uint;
    }
}

pub(super) const TYPE_COMMAND_ARRAYS: GlslStruct =
    DrawArraysIndirectCommandGlslStruct::as_definition();
pub(super) const TYPE_COMMAND_ELEMENTS: GlslStruct =
    DrawElementsIndirectCommandGlslStruct::as_definition();

ethel::shader_glsl_compute! {
    struct ProcessCommand > [460] {
        workgroup [32, 32, 1];

        type {
            TYPE_COMMAND_ARRAYS
            TYPE_COMMAND_ELEMENTS
            commons::TYPE_MESH_METADATA
            commons::TYPE_MESH_VERTEX
        };

        ssbo {
            ethel::mesh::GLSL_SSBO_INTEGRATION[0]
            ethel::mesh::GLSL_SSBO_INTEGRATION[1]

            ethel::shader_glsl_ssbo! {
                buf Command_Buffer on 0 => {
                    [dyn_array TYPE_COMMAND_ARRAYS: command_buffer]
                }
            }
            ethel::shader_glsl_ssbo! {
                buf POD_MeshID on 1 => {
                    [dyn_array uint: pod_mesh_id]
                }
            }
        };

        src() "
            uvec2 g_wg_row = gl_NumWorkGroups.x * gl_WorkGroupSize.x;
            uvec2 g_wg_id = gl_GlobalInvocationID;

            uint g_wg = g_wg_id.y * g_wg_row + g_wg_id.x;

            uint mesh_id = pod_mesh_id[g_wg + 1];
            if (mesh_id == 0) {
                return;
            }

            uint vertex_len = metadata[mesh_id].length;

            command_buffer[mesh_id].count = vertex_len;
            atomicAdd(command_buffer[mesh_id].instance_count, 1);
        "
    }
}
