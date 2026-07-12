use std::sync::{Arc, atomic::AtomicU32};

use crate::{render::shaders, ui::UiRenderCommandBasic};
use ethel::{
    DrawCommand, layout_buffer, layout_mesh_buffer,
    render::buffer::{PartitionedTriBuffer, TriBuffer},
    state::data::{DirectIndex, IndirectIndex},
};
use gui::draw::Quad;

use crate::structure::fragment::ANCHORS_COUNT as FRAGMENT_ANCHORS_COUNT;

pub const FRAGMENT_COMMANDS_ALLOC: usize = 131072;
pub const DEBRIS_COMMANDS_ALLOC: usize = 131072;
pub const INTERFACE_COMMANDS_ALLOC: usize = 2048;

/// Temporarily forced to 1 to save memory as generic objects are currently
/// unused
pub const GENERIC_COMMANDS_ALLOC: usize = 1;

pub const RENDERABLE_STORAGE_PARTS: usize = 8;
pub const ENTITY_ALLOCATION: usize = 8192;

pub const LATTICE_CONSTRAINT_ALLOC: usize = 32768;
pub const LATTICE_NODE_ALLOC: usize = 8192;
pub const LATTICE_STORAGE_PARTS: usize = 4;

pub const DEBRIS_ALLOC: usize = 131072;
pub const DEBRIS_STORAGE_PARTS: usize = 3;

pub const FRAGMENTS_ALLOC: usize = 131072;
pub const FRAGMENTS_STORAGE_PARTS: usize = 8;
pub const DEFORM_POINTS_ALLOC: usize = 181072;

pub const MESH_BUFFER_LEN: usize = 2048;
pub const MESH_BUFFER_SIZE: usize = 65536;

#[cfg(feature = "devmode")]
pub const DEBUG_LINES_ALLOC: usize = 16384;

pub const INTERFACE_INSTANCES_ALLOC: usize = 8192;

layout_mesh_buffer!(count: MESH_BUFFER_LEN; vertices: MESH_BUFFER_SIZE);

layout_buffer! {
    const RenderableData: RENDERABLE_STORAGE_PARTS, {
        enum MeshID: ENTITY_ALLOCATION => {
            type u32;
            bind 0;
            shader 0;
        };
        enum PodPositions: ENTITY_ALLOCATION => {
            type [f32; 4];
            bind 1;
            shader 1;
        };
        enum PodRotations: ENTITY_ALLOCATION => {
            type [f32; 4];
            bind 2;
            shader 2;
        };
        enum PodScales: ENTITY_ALLOCATION => {
            type [f32; 4];
            bind 3;
            shader 3;
        };
    }
}

layout_buffer! {
    const XpbdDebugData: LATTICE_STORAGE_PARTS, {
        enum Constraints: LATTICE_CONSTRAINT_ALLOC => {
            type [IndirectIndex; 2];
            bind 0;
            shader shaders::debug::SSBO_INDEX_POD_CONSTRAINTS;
        };

        enum IMapNodes: LATTICE_NODE_ALLOC => {
            type IndirectIndex;
            bind 1;
            shader shaders::debug::SSBO_INDEX_IMAP_NODES;
        };
        enum PodNodes: LATTICE_CONSTRAINT_ALLOC => {
            type [f32; 4];
            bind 2;
            shader shaders::debug::SSBO_INDEX_POD_NODES;
        };

        enum I_Selected: 1 => {
            type DirectIndex;
            bind 3;
            shader shaders::debug::SSBO_INDEX_I_SELECTED;
        };
    }
}

layout_buffer! {
    const FragmentData: FRAGMENTS_STORAGE_PARTS, {
        enum PodAnchors: FRAGMENTS_ALLOC => {
            type [IndirectIndex; FRAGMENT_ANCHORS_COUNT];
            bind 0;
            shader shaders::fragments::SSBO_INDEX_POD_ANCHORS;
        };
        enum PodAnchorsWeights: FRAGMENTS_ALLOC => {
            type [f32; FRAGMENT_ANCHORS_COUNT];
            bind 1;
            shader shaders::fragments::SSBO_INDEX_POD_WEIGHTS;
        };
        enum PodBindPose: FRAGMENTS_ALLOC => {
            type glam::Vec4;
            bind 2;
            shader shaders::fragments::SSBO_INDEX_POD_BINDPOSE;
        };
        enum PodMeshId: FRAGMENTS_ALLOC => {
            type ethel::mesh::Id;
            bind 3;
            shader shaders::fragments::SSBO_INDEX_POD_MESHID;
        };

        enum IMapDeforms: DEFORM_POINTS_ALLOC => {
            type IndirectIndex;
            bind 4;
            shader shaders::fragments::SSBO_INDEX_IMAP_DEFORMS;
        };
        enum PodDeformsPositions: DEFORM_POINTS_ALLOC => {
            type [f32; 4];
            bind 5;
            shader shaders::fragments::SSBO_INDEX_POD_DEFORMS_POSITIONS;
        };
        enum PodDeformsBindPose: DEFORM_POINTS_ALLOC => {
            type [f32; 4];
            bind 6;
            shader shaders::fragments::SSBO_INDEX_POD_DEFORMS_BINDPOSE;
        };
    }
}

layout_buffer! {
    const DebrisData: DEBRIS_STORAGE_PARTS, {
        enum PodPositions: DEBRIS_ALLOC => {
            type [f32; 4];
            bind 0;
            shader shaders::debris::SSBO_INDEX_POD_POSITIONS;
        };
        enum PodRotations: DEBRIS_ALLOC => {
            type [f32; 4];
            bind 1;
            shader shaders::debris::SSBO_INDEX_POD_ROTATIONS;
        };
        enum PodMeshId: DEBRIS_ALLOC => {
            type ethel::mesh::Id;
            bind 2;
            shader shaders::debris::SSBO_INDEX_POD_MESHID;
        };
    }
}

#[cfg(feature = "devmode")]
layout_buffer! {
    const DebugLinesData: 2, {
        enum PodPoints: DEBUG_LINES_ALLOC => {
            type [f32; 4];
            bind 0;
            shader shaders::lines::SSBO_INDEX_POD_POINTS;
        };
        enum PodColors: DEBUG_LINES_ALLOC => {
            type [f32; 4];
            bind 1;
            shader shaders::lines::SSBO_INDEX_POD_COLORS;
        };
    }
}

#[derive(Debug, Default)]
pub struct FrameDataBuffers {
    pub fragment_commands: TriBuffer<DrawCommand>,
    pub debris_commands: TriBuffer<DrawCommand>,
    pub generic_commands: TriBuffer<DrawCommand>,
    pub interface_commands: TriBuffer<UiRenderCommandBasic>,

    pub generic_objects: PartitionedTriBuffer<RENDERABLE_STORAGE_PARTS>,
    pub fragments: PartitionedTriBuffer<FRAGMENTS_STORAGE_PARTS>,
    pub debris: PartitionedTriBuffer<DEBRIS_STORAGE_PARTS>,
    pub debris_count: Arc<AtomicU32>,
    pub cage_points_count: Arc<AtomicU32>,

    pub lattice_debug: PartitionedTriBuffer<LATTICE_STORAGE_PARTS>,
    pub lattice_constraint_count: Arc<AtomicU32>,

    #[cfg(feature = "devmode")]
    pub lines_debug: PartitionedTriBuffer<2>,

    pub interface_storage: TriBuffer<Quad>,
}

impl FrameDataBuffers {
    pub fn new() -> Self {
        let generic_objects_buffer = PartitionedTriBuffer::new(LayoutRenderableData::create());
        LayoutRenderableData::initialise_partitions(&generic_objects_buffer);

        let xpbd_visualiser = PartitionedTriBuffer::new(LayoutXpbdDebugData::create());
        LayoutXpbdDebugData::initialise_partitions(&xpbd_visualiser);

        let fragment_data = PartitionedTriBuffer::new(LayoutFragmentData::create());
        LayoutFragmentData::initialise_partitions(&fragment_data);

        let debris_data = PartitionedTriBuffer::new(LayoutDebrisData::create());
        LayoutDebrisData::initialise_partitions(&debris_data);

        #[cfg(feature = "devmode")]
        let lines_debug = PartitionedTriBuffer::new(LayoutDebugLinesData::create());
        #[cfg(feature = "devmode")]
        LayoutDebugLinesData::initialise_partitions(&debris_data);

        Self {
            fragment_commands: TriBuffer::zeroed(FRAGMENT_COMMANDS_ALLOC),
            debris_commands: TriBuffer::zeroed(DEBRIS_COMMANDS_ALLOC),
            generic_commands: TriBuffer::zeroed(GENERIC_COMMANDS_ALLOC),
            interface_commands: TriBuffer::zeroed(INTERFACE_COMMANDS_ALLOC),

            generic_objects: generic_objects_buffer,
            fragments: fragment_data,
            debris: debris_data,
            debris_count: Arc::new(AtomicU32::new(0)),
            cage_points_count: Arc::new(AtomicU32::new(0)),

            lattice_debug: xpbd_visualiser,
            lattice_constraint_count: Arc::new(AtomicU32::new(0)),

            #[cfg(feature = "devmode")]
            lines_debug,

            interface_storage: TriBuffer::zeroed(INTERFACE_INSTANCES_ALLOC),
        }
    }
}
