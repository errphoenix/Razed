use ethel::{
    DrawCommand, layout_buffer, layout_mesh_buffer,
    render::buffer::{PartitionedTriBuffer, TriBuffer},
};

pub const RENDER_STORAGE_PARTS: usize = 8;
pub const ENTITY_ALLOCATION: usize = 1024;
pub const COMMAND_QUEUE_ALLOC: usize = 1024;

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
    const XpbdDebugData: 3, {
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
    }
}

#[derive(Debug, Default)]
pub struct FrameDataBuffers {
    pub command: TriBuffer<DrawCommand>,
    pub scene: PartitionedTriBuffer<RENDER_STORAGE_PARTS>,
    pub xpbd_debug: PartitionedTriBuffer<3>,
}

impl FrameDataBuffers {
    pub fn new() -> Self {
        let scene_data_buffer = PartitionedTriBuffer::new(LayoutEntityData::create());
        LayoutEntityData::initialise_partitions(&scene_data_buffer);

        let xpbd_visualiser = PartitionedTriBuffer::new(LayoutXpbdDebugData::create());
        LayoutXpbdDebugData::initialise_partitions(&xpbd_visualiser);

        Self {
            command: TriBuffer::zeroed(COMMAND_QUEUE_ALLOC),
            scene: scene_data_buffer,
            xpbd_debug: xpbd_visualiser,
        }
    }
}
