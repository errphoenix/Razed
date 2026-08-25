use std::sync::{Arc, atomic::AtomicU32};

use crate::{
    render::{self, pass::CagePoints},
    structure::cage::{
        CageSyncFrameBuffers, OffsetRotation, PER_CAGE_MAX_LATTICE_ATTACHMENTS, PER_CAGE_POINTS,
    },
};
use ethel::{
    DrawCommand, layout_buffer, layout_mesh_buffer,
    render::buffer::{PartitionedTriBuffer, TriBuffer},
    state::data::{DirectIndex, IndirectIndex},
};
use gui::render::{UiCommandsBuffer, UiDataBuffer};
use janus::{context::DeltaTime, sync::TriCell};

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
pub const FRAGMENTS_STORAGE_PARTS: usize = 3;
pub const CAGES_ALLOC: usize = FRAGMENTS_ALLOC;

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
            shader render::pass::debug_lattice_draw::SSBO_INDEX_POD_CONSTRAINTS;
        };

        enum IMapNodes: LATTICE_NODE_ALLOC => {
            type IndirectIndex;
            bind 1;
            shader render::pass::debug_lattice_draw::SSBO_INDEX_IMAP_NODES;
        };
        enum PodNodes: LATTICE_CONSTRAINT_ALLOC => {
            type [f32; 4];
            bind 2;
            shader render::pass::debug_lattice_draw::SSBO_INDEX_POD_NODES;
        };

        enum I_Selected: 1 => {
            type DirectIndex;
            bind 3;
            shader render::pass::debug_lattice_draw::SSBO_INDEX_I_SELECTED;
        };
    }
}

layout_buffer! {
    const FragmentData: FRAGMENTS_STORAGE_PARTS, {
        enum PodBindPose: FRAGMENTS_ALLOC => {
            type glam::Vec4;
            bind 0;
            shader render::pass::fragments_draw::SSBO_INDEX_POD_BINDPOSE;
        };
        enum PodMeshId: FRAGMENTS_ALLOC => {
            type ethel::mesh::Id;
            bind 1;
            shader render::pass::fragments_draw::SSBO_INDEX_POD_MESHID;
        };
        enum PodCageIds: FRAGMENTS_ALLOC => {
            type [IndirectIndex; FRAGMENT_ANCHORS_COUNT];
            bind 2;
            shader render::pass::fragments_draw::SSBO_INDEX_POD_CAGEID;
        };
    }
}

ethel::typed_part_buffer! {
    const Cage: 9, {
        enum RMap: CAGES_ALLOC => {
            type IndirectIndex;
            bind 0;
        };

        enum Pod_BindRef: CAGES_ALLOC => {
            type glam::Vec4;
            bind 1;
            shader render::pass::cage_deform_compute::SSBO_INDEX_POD_BIND_REF;
        };
        enum Pod_Points: CAGES_ALLOC => {
            type CagePoints;
            bind 2;
            shader render::pass::cage_deform_compute::SSBO_INDEX_POD_POINTS;
        };
        enum Pod_Points_Bind: CAGES_ALLOC => {
            type CagePoints;
            bind 3;
            shader render::pass::cage_deform_compute::SSBO_INDEX_POD_POINTS_BIND;
        };
        enum Pod_Barycenter_Bind: CAGES_ALLOC => {
            type [glam::Vec4; PER_CAGE_POINTS];
            bind 4;
            shader render::pass::cage_deform_compute::SSBO_INDEX_POD_BARYCENTER_BIND;
        };
        enum Pod_Attachments: CAGES_ALLOC => {
            type [render::pass::LatticeAttachments; PER_CAGE_POINTS];
            bind 5;
            shader render::pass::cage_deform_compute::SSBO_INDEX_POD_ATTACHMENTS;
        };
        enum Pod_Lut_Lattice: CAGES_ALLOC => {
            type [IndirectIndex; PER_CAGE_MAX_LATTICE_ATTACHMENTS];
            bind 6;
            shader render::pass::cage_deform_compute::SSBO_INDEX_POD_LUT_LATTICE;
        };
        enum Pod_Bind_Lattice: CAGES_ALLOC => {
            type [glam::Vec4; PER_CAGE_MAX_LATTICE_ATTACHMENTS];
            bind 7;
            shader render::pass::cage_deform_compute::SSBO_INDEX_POD_BIND_LATTICE;
        };
    }
}

layout_buffer! {
    const DebrisData: DEBRIS_STORAGE_PARTS, {
        enum PodPositions: DEBRIS_ALLOC => {
            type [f32; 4];
            bind 0;
            shader render::pass::debris_draw::SSBO_INDEX_POD_POSITIONS;
        };
        enum PodRotations: DEBRIS_ALLOC => {
            type [f32; 4];
            bind 1;
            shader render::pass::debris_draw::SSBO_INDEX_POD_ROTATIONS;
        };
        enum PodMeshId: DEBRIS_ALLOC => {
            type ethel::mesh::Id;
            bind 2;
            shader render::pass::debris_draw::SSBO_INDEX_POD_MESHID;
        };
    }
}

#[cfg(feature = "devmode")]
layout_buffer! {
    const DebugLinesData: 2, {
        enum PodPoints: DEBUG_LINES_ALLOC => {
            type [f32; 4];
            bind 0;
            shader render::pass::debug_lines_draw::SSBO_INDEX_POD_POINTS;
        };
        enum PodColors: DEBUG_LINES_ALLOC => {
            type [f32; 4];
            bind 1;
            shader render::pass::debug_lines_draw::SSBO_INDEX_POD_COLORS;
        };
    }
}

#[derive(Debug, Default)]
pub struct FrameDataBuffers {
    pub fragment_commands: TriBuffer<DrawCommand>,
    pub debris_commands: TriBuffer<DrawCommand>,
    pub generic_commands: TriBuffer<DrawCommand>,

    pub generic_objects: PartitionedTriBuffer<RENDERABLE_STORAGE_PARTS>,
    pub fragments: PartitionedTriBuffer<FRAGMENTS_STORAGE_PARTS>,
    pub debris: PartitionedTriBuffer<DEBRIS_STORAGE_PARTS>,
    pub debris_count: Arc<AtomicU32>,

    pub cages: CagePartitionedBuffer,
    pub cage_map: TriBuffer<DirectIndex>,
    pub cage_points_count: Arc<AtomicU32>,
    pub cage_feedback: TriBuffer<OffsetRotation>,
    pub cage_sync_frame: CageSyncFrameBuffers,

    pub lattice_debug: PartitionedTriBuffer<LATTICE_STORAGE_PARTS>,
    pub lattice_constraint_count: Arc<AtomicU32>,
    #[cfg(feature = "devmode")]
    pub lines_debug: PartitionedTriBuffer<2>,

    pub interface_storage: UiDataBuffer,
    pub interface_commands: UiCommandsBuffer,

    pub render_frame_last_duration: TriCell<DeltaTime>,

    pub debug_material_index: TriCell<u32>,
}

impl FrameDataBuffers {
    pub fn new() -> Self {
        let generic_objects_buffer = PartitionedTriBuffer::new(LayoutRenderableData::create());
        LayoutRenderableData::initialise_partitions_tri(&generic_objects_buffer);

        let xpbd_visualiser = PartitionedTriBuffer::new(LayoutXpbdDebugData::create());
        LayoutXpbdDebugData::initialise_partitions_tri(&xpbd_visualiser);

        let fragment_data = PartitionedTriBuffer::new(LayoutFragmentData::create());
        LayoutFragmentData::initialise_partitions_tri(&fragment_data);

        let debris_data = PartitionedTriBuffer::new(LayoutDebrisData::create());
        LayoutDebrisData::initialise_partitions_tri(&debris_data);

        #[cfg(feature = "devmode")]
        let lines_debug = PartitionedTriBuffer::new(LayoutDebugLinesData::create());
        #[cfg(feature = "devmode")]
        LayoutDebugLinesData::initialise_partitions_tri(&debris_data);

        Self {
            fragment_commands: TriBuffer::zeroed(FRAGMENT_COMMANDS_ALLOC),
            debris_commands: TriBuffer::zeroed(DEBRIS_COMMANDS_ALLOC),
            generic_commands: TriBuffer::zeroed(GENERIC_COMMANDS_ALLOC),

            generic_objects: generic_objects_buffer,
            fragments: fragment_data,
            debris: debris_data,
            debris_count: Arc::new(AtomicU32::new(0)),

            cages: CagePartitionedBuffer::new(),
            cage_map: TriBuffer::zeroed(CAGES_ALLOC),
            cage_points_count: Arc::new(AtomicU32::new(0)),
            cage_feedback: TriBuffer::zeroed(CAGES_ALLOC),
            cage_sync_frame: CageSyncFrameBuffers::new(),

            lattice_debug: xpbd_visualiser,
            lattice_constraint_count: Arc::new(AtomicU32::new(0)),
            #[cfg(feature = "devmode")]
            lines_debug,

            interface_commands: TriBuffer::zeroed(INTERFACE_COMMANDS_ALLOC),
            interface_storage: TriBuffer::zeroed(INTERFACE_INSTANCES_ALLOC),

            render_frame_last_duration: TriCell::new(DeltaTime::default()),

            debug_material_index: TriCell::new(0),
        }
    }
}
