use ethel::{
    DrawCommand, layout_buffer, layout_mesh_buffer,
    render::buffer::{PartitionedTriBuffer, TriBuffer},
};

pub const RENDER_STORAGE_PARTS: usize = 8;
pub const ENTITY_ALLOCATION: usize = 1024;
pub const COMMAND_QUEUE_ALLOC: usize = 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[repr(C)]
pub struct Entity {
    pub mesh_id: u32,
    pub position_handle: u32,
    pub rotation_handle: u32,
    pub scale_handle: u32,
}

layout_mesh_buffer!(count: 512; vertices: 2048);

layout_buffer! {
    const EntityData: RENDER_STORAGE_PARTS, {
        enum EntityIndexMap: ENTITY_ALLOCATION => {
            type Entity;
            bind 0;
            shader 0;
        };
        enum MeshData: ENTITY_ALLOCATION => {
            type u32;
            bind 1;
            shader 1;
        };

        enum IMapPositions: ENTITY_ALLOCATION => {
            type u32;
            bind 2;
            shader 2;
        };
        enum IMapRotations: ENTITY_ALLOCATION => {
            type u32;
            bind 3;
            shader 3;
        };
        enum IMapScales: ENTITY_ALLOCATION => {
            type u32;
            bind 4;
            shader 4;
        };

        enum PodPositions: ENTITY_ALLOCATION => {
            type [f32; 4];
            bind 5;
            shader 5;
        };
        enum PodRotations: ENTITY_ALLOCATION => {
            type [f32; 4];
            bind 6;
            shader 6;
        };
        enum PodScales: ENTITY_ALLOCATION => {
            type [f32; 4];
            bind 7;
            shader 7;
        };
    }
}

#[derive(Debug, Default)]
pub struct FrameDataBuffers {
    pub command: TriBuffer<DrawCommand>,
    pub scene: PartitionedTriBuffer<RENDER_STORAGE_PARTS>,
}

impl FrameDataBuffers {
    pub fn new() -> Self {
        let scene_data_buffer = PartitionedTriBuffer::new(LayoutEntityData::create());
        LayoutEntityData::initialise_partitions(&scene_data_buffer);

        Self {
            command: TriBuffer::zeroed(COMMAND_QUEUE_ALLOC),
            scene: scene_data_buffer,
        }
    }
}
