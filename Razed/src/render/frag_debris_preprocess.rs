//! Command buffer processing & population
//!
//! The purpose of this pre-processing pass is to map a contiguous
//! vector/array of [`MeshID`], which represents the mesh ID of an arbitrary
//! group of entities, each to its own dedicated indirect draw command;
//! populating draw count, vertex/element count, etc.
//!
//! The value of a [`MeshID`] is directly mapped to the contiguous command
//! buffer: a mesh of ID `I` will be mapped to the entry of the command buffer
//! at that same index `I`.
//!
//! Given that, the pre-process pass expects the command buffer to be
//! zero-initialised (all fields `0`) for atleast the length of the contiguous
//! [`MeshID`] collection.
//!
//! [`MeshID`]: ethel::mesh::Id

use ethel::{
    render::{
        buffer::{PartitionedTriBuffer, TriBuffer},
        command::DrawArraysIndirectCommand,
    },
    shader::{GlslStruct, ShaderProgram},
};
use rendrs::pipeline::ComputePass;

use crate::{
    data::{LayoutDebrisData, LayoutFragmentData},
    render::shader_commons,
};

pub type FragmentDebrisPreprocessComputePass<'ctx> =
    ComputePass<FragmentDebrisPreprocessCtx<'ctx>, 0, 0>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FragmentDebrisPreprocessTarget {
    Fragments,
    Debris,
}

#[derive(Debug)]
pub struct FragmentDebrisPreprocessCtx<'data> {
    pub target: FragmentDebrisPreprocessTarget,

    pub fragment_commands: &'data TriBuffer<DrawArraysIndirectCommand>,
    pub fragment_data: &'data PartitionedTriBuffer<8>,

    pub debris_commands: &'data TriBuffer<DrawArraysIndirectCommand>,
    pub debris_data: &'data PartitionedTriBuffer<3>,
}

pub const fn pass(shader: &ComputeShaderProcessCommand) -> FragmentDebrisPreprocessComputePass {
    let handle_view = shader.compute_handle().view();
    FragmentDebrisPreprocessComputePass::new(handle_view, [], [], |section, ctx| {
        let section = section.as_index();

        const MESH_BIND: u32 = SSBO_INDEX_FRAGMENTS_MESH_IDS;
        const CMD_BIND: u32 = SSBO_INDEX_COMMAND_BUFFER;

        let wg_count = match ctx.target {
            FragmentDebrisPreprocessTarget::Fragments => {
                let cmds = ctx.fragment_commands.view_section(section);
                ctx.fragment_data.bind_shader_storage_single(
                    section,
                    LayoutFragmentData::PodMeshId as usize,
                    Some(MESH_BIND),
                );
                ctx.fragment_commands
                    .bind_shader_storage(section, CMD_BIND, 0);
                cmds.length().div_ceil(WORKGROUP_INVOCATIONS)
            }
            FragmentDebrisPreprocessTarget::Debris => {
                let cmds = ctx.debris_commands.view_section(section);
                ctx.debris_data.bind_shader_storage_single(
                    section,
                    LayoutDebrisData::PodMeshId as usize,
                    Some(MESH_BIND),
                );
                ctx.debris_commands
                    .bind_shader_storage(section, CMD_BIND, 0);
                cmds.length().div_ceil(WORKGROUP_INVOCATIONS)
            }
        };

        [wg_count, 1, 1]
    })
}

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

pub const TYPE_COMMAND_ARRAYS: GlslStruct = DrawArraysIndirectCommandGlslStruct::as_definition();
pub const TYPE_COMMAND_ELEMENTS: GlslStruct =
    DrawElementsIndirectCommandGlslStruct::as_definition();

pub const WORKGROUP_SIZE_XY: u32 = 1;
pub const WORKGROUP_INVOCATIONS: u32 = WORKGROUP_SIZE_XY * WORKGROUP_SIZE_XY;

macro_rules! ssbo_binding {
    (Command_Buffer) => {
        0
    };
    (POD_MeshID) => {
        1
    };
}

pub const SSBO_INDEX_COMMAND_BUFFER: u32 = ssbo_binding!(Command_Buffer);
pub const SSBO_INDEX_FRAGMENTS_MESH_IDS: u32 = ssbo_binding!(POD_MeshID);

ethel::shader_glsl_compute! {
    struct ProcessCommand > [460] {
        workgroup [1, 1, 1];

        type {
            TYPE_COMMAND_ARRAYS
            TYPE_COMMAND_ELEMENTS
            shader_commons::TYPE_MESH_METADATA
            shader_commons::TYPE_MESH_VERTEX
        };

        ssbo {
            ethel::mesh::GLSL_SSBO_INTEGRATION[0]
            ethel::mesh::GLSL_SSBO_INTEGRATION[1]

            ethel::shader_glsl_ssbo! {
                buf Command_Buffer => {
                    [dyn_array DrawArraysIndirectCommand: command_buffer]
                }
            }
            ethel::shader_glsl_ssbo! {
                buf POD_MeshID => {
                    [dyn_array uint: pod_mesh_id]
                }
            }
        };

        src() "
            uint g_wg_row = gl_NumWorkGroups.x * gl_WorkGroupSize.x;
            uvec2 g_wg_id = gl_GlobalInvocationID.xy;
            uint g_wg = g_wg_id.y * g_wg_row + g_wg_id.x;

            uint mesh_id = pod_mesh_id[g_wg + 1];
            if (mesh_id == 0) {
                return;
            }

            uint vertex_len = metadata[mesh_id].length;

            command_buffer[g_wg].count = vertex_len;
        "
    }
}
