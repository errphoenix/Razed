use std::sync::{Arc, atomic::AtomicU32};

use ethel::{
    DrawCommand, layout_buffer, layout_mesh_buffer,
    render::buffer::{InitStrategy, PartitionedTriBuffer, TriBuffer},
    state::data::IndirectIndex,
};

use crate::structure::deforms::{
    CONTROL_POINTS_COUNT as DEFORM_CONTROL_POINTS_COUNT, ControlPoint,
};
use crate::structure::fragment::ANCHORS_COUNT as FRAGMENT_ANCHORS_COUNT;

pub const RENDER_STORAGE_PARTS: usize = 8;
pub const ENTITY_ALLOCATION: usize = 8192;
pub const COMMAND_QUEUE_ALLOC: usize = 2048;

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[repr(C)]
pub struct Renderable {
    pub mesh_id: u32,
    pub data_handle: IndirectIndex,
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
        enum PodAnchors: FRAGMENTS_ALLOC => {
            type [u32; FRAGMENT_ANCHORS_COUNT];
            bind 0;
            shader 0;
        };
        enum PodAnchorsWeights: FRAGMENTS_ALLOC => {
            type [f32; FRAGMENT_ANCHORS_COUNT];
            bind 1;
            shader 1;
        };
        enum PodBindPose: FRAGMENTS_ALLOC => {
            type glam::Vec4;
            bind 2;
            shader 2;
        };
        enum PodStates: FRAGMENTS_ALLOC => {
            type u32;
            bind 3;
            shader 3;
        };

        enum IMapDeforms: DEFORM_POINTS_ALLOC => {
            type u32;
            bind 4;
            shader 6;
        };
        enum PodDeformsPositions: DEFORM_POINTS_ALLOC => {
            type [f32; 4];
            bind 5;
            shader 7;
        };
        enum PodDeformsBindPose: DEFORM_POINTS_ALLOC => {
            type [f32; 4];
            bind 6;
            shader 8;
        };
    }
}

pub const DEBRIS_ALLOC: usize = 16384;
pub const DEBRIS_DATA_PARTS: usize = 2;

layout_buffer! {
    const DebrisData: DEBRIS_DATA_PARTS, {
        enum PodPositions: DEBRIS_ALLOC => {
            type [f32; 4];
            bind 0;
            shader 0;
        };
        enum PodRotations: DEBRIS_ALLOC => {
            type [f32; 4];
            bind 1;
            shader 1;
        };
    }
}

pub const DEFORM_POINTS_ALLOC: usize = 32000;

#[derive(Debug, Default)]
pub struct FrameDataBuffers {
    pub command: TriBuffer<DrawCommand>,
    pub scene: PartitionedTriBuffer<RENDER_STORAGE_PARTS>,
    pub fragments: PartitionedTriBuffer<FRAGMENTS_DATA_PARTS>,
    pub debris: PartitionedTriBuffer<DEBRIS_DATA_PARTS>,
    pub debris_count: Arc<AtomicU32>,

    pub xpbd_debug: PartitionedTriBuffer<4>,
    pub xpbd_debug_link_count: Arc<AtomicU32>,

    pub deform_debug: TriBuffer<glam::Vec4>,
    pub deform_debug_controls: TriBuffer<[ControlPoint; DEFORM_CONTROL_POINTS_COUNT]>,
    pub deform_debug_count: Arc<AtomicU32>,
}

impl FrameDataBuffers {
    pub fn new() -> Self {
        let scene_data_buffer = PartitionedTriBuffer::new(LayoutEntityData::create());
        LayoutEntityData::initialise_partitions(&scene_data_buffer);

        let xpbd_visualiser = PartitionedTriBuffer::new(LayoutXpbdDebugData::create());
        LayoutXpbdDebugData::initialise_partitions(&xpbd_visualiser);

        let fragment_data = PartitionedTriBuffer::new(LayoutFragmentData::create());
        LayoutFragmentData::initialise_partitions(&fragment_data);

        let debris_data = PartitionedTriBuffer::new(LayoutDebrisData::create());
        LayoutDebrisData::initialise_partitions(&debris_data);

        let deform_debug = TriBuffer::new(
            DEFORM_POINTS_ALLOC,
            InitStrategy::FillWith(|| glam::Vec4::NAN),
        );
        let deform_debug_controls = TriBuffer::new(
            DEFORM_POINTS_ALLOC,
            InitStrategy::FillWith(|| [ControlPoint::default(); DEFORM_CONTROL_POINTS_COUNT]),
        );

        Self {
            command: TriBuffer::zeroed(COMMAND_QUEUE_ALLOC),

            scene: scene_data_buffer,
            fragments: fragment_data,
            debris: debris_data,
            debris_count: Arc::new(AtomicU32::new(0)),

            xpbd_debug: xpbd_visualiser,
            xpbd_debug_link_count: Arc::new(AtomicU32::new(0)),

            deform_debug,
            deform_debug_controls,
            deform_debug_count: Arc::new(AtomicU32::new(0)),
        }
    }
}
