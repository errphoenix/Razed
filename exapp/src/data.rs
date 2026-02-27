use std::sync::{Arc, atomic::AtomicU32};

use ethel::{
    DrawCommand, layout_buffer, layout_mesh_buffer,
    render::buffer::{PartitionedTriBuffer, TriBuffer},
};

pub const RENDER_STORAGE_PARTS: usize = 8;
pub const ENTITY_ALLOCATION: usize = 8192;
pub const COMMAND_QUEUE_ALLOC: usize = 2048;

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[repr(C)]
pub struct Renderable {
    pub mesh_id: u32,
    pub data_handle: u32,
}

layout_mesh_buffer!(count: 512; vertices: 2048);

layout_buffer! {
    const EntityData: RENDER_STORAGE_PARTS, {
        enum EntityIndexMap: ENTITY_ALLOCATION => {
            type Renderable;
            bind 0;
            shader 0;
        };
        enum MeshData: ENTITY_ALLOCATION => {
            type u32;
            bind 1;
            shader 1;
        };

        enum IMapEntityData: ENTITY_ALLOCATION => {
            type u32;
            bind 2;
            shader 2;
        };
        enum PodPositions: ENTITY_ALLOCATION => {
            type [f32; 4];
            bind 3;
            shader 4;
        };
        enum PodRotations: ENTITY_ALLOCATION => {
            type [f32; 4];
            bind 4;
            shader 5;
        };
        enum PodScales: ENTITY_ALLOCATION => {
            type [f32; 4];
            bind 5;
            shader 6;
        };
    }
}

pub const XPBD_CONSTRAINTS_ALLOC: usize = 4096;
pub const XPBD_NODES_ALLOC: usize = 512;

layout_buffer! {
    const XpbdDebugData: 4, {
        enum Constraints: XPBD_CONSTRAINTS_ALLOC => {
            type [u32; 2];
            bind 0;
            shader 4;
        };

        enum IMapNodes: XPBD_NODES_ALLOC => {
            type u32;
            bind 1;
            shader 5;
        };
        enum PodNodes: XPBD_CONSTRAINTS_ALLOC => {
            type [f32; 4];
            bind 2;
            shader 6;
        };

        enum I_Selected: 1 => {
            type u32;
            bind 3;
            shader 7;
        };
    }
}

pub const FRAGMENTS_ALLOC: usize = 16384;
pub const FRAGMENTS_DATA_PARTS: usize = 7;

layout_buffer! {
    const FragmentData: FRAGMENTS_DATA_PARTS, {
        enum PodParents: FRAGMENTS_ALLOC => {
            type [u32; 4];
            bind 0;
            shader 0;
        };
        enum PodWeights: FRAGMENTS_ALLOC => {
            type [f32; 4];
            bind 1;
            shader 1;
        };
        enum PodOffsets: FRAGMENTS_ALLOC => {
            type glam::Vec4;
            bind 2;
            shader 2;
        };
        enum PodStates: FRAGMENTS_ALLOC => {
            type u32;
            bind 3;
            shader 3;
        };

        enum IMapNodes: XPBD_NODES_ALLOC => {
            type u32;
            bind 4;
            shader 6;
        };
        enum PodNodesPositions: XPBD_NODES_ALLOC => {
            type [f32; 4];
            bind 5;
            shader 7;
        };
        enum PodNodesRotors: XPBD_NODES_ALLOC => {
            type [f32; 4];
            bind 6;
            shader 8;
        };
    }
}

#[derive(Debug, Default)]
pub struct FrameDataBuffers {
    pub command: TriBuffer<DrawCommand>,
    pub scene: PartitionedTriBuffer<RENDER_STORAGE_PARTS>,
    pub fragments: PartitionedTriBuffer<FRAGMENTS_DATA_PARTS>,

    pub xpbd_debug: PartitionedTriBuffer<4>,
    pub xpbd_debug_link_count: Arc<AtomicU32>,
}

impl FrameDataBuffers {
    pub fn new() -> Self {
        let scene_data_buffer = PartitionedTriBuffer::new(LayoutEntityData::create());
        LayoutEntityData::initialise_partitions(&scene_data_buffer);

        let xpbd_visualiser = PartitionedTriBuffer::new(LayoutXpbdDebugData::create());
        LayoutXpbdDebugData::initialise_partitions(&xpbd_visualiser);

        let fragment_data = PartitionedTriBuffer::new(LayoutFragmentData::create());
        LayoutFragmentData::initialise_partitions(&fragment_data);

        Self {
            command: TriBuffer::zeroed(COMMAND_QUEUE_ALLOC),

            scene: scene_data_buffer,
            xpbd_debug: xpbd_visualiser,
            fragments: fragment_data,

            xpbd_debug_link_count: Arc::new(AtomicU32::new(0)),
        }
    }
}
