use std::sync::{Arc, atomic::AtomicU32};

use crate::{
    render::{
        self,
        graphics::{DebugRenderOptions, Groundplane, RenderParams, RenderStats},
        shading::attributes::ShadingAttributesMode,
        task::deform_cages::{CagePoints, LatticeAttachments},
    },
    simulation::cage::{
        CageSyncFrameBuffers, OffsetRotation, PER_CAGE_MAX_LATTICE_ATTACHMENTS, PER_CAGE_POINTS,
    },
    structure::DamageOverlay,
};
use ethel::{DirectIndex, IndirectIndex, TriBuffer, layout_buffer, layout_mesh_buffer};
use gui::render::{UiCommandsBuffer, UiDataBuffer};
use janus::{DeltaTime, sync::TriCell};

pub const INTERFACE_COMMANDS_ALLOC: usize = 2048;

pub const LATTICE_CONSTRAINT_ALLOC: usize = 32768;
pub const LATTICE_NODE_ALLOC: usize = 8192;
pub const DEBRIS_ALLOC: usize = 131072;
pub const FRAGMENTS_ALLOC: usize = 131072;
pub const CAGES_ALLOC: usize = FRAGMENTS_ALLOC;

#[cfg(feature = "devmode")]
pub const DEBUG_LINES_ALLOC: usize = 16384;

pub const INTERFACE_INSTANCES_ALLOC: usize = 8192;

layout_mesh_buffer!(
    count    : 2048;
    vertices : 32768;
    tris     : 32768
);

ethel::typed_part_tribuffer! {
    const Lattice: 3, {
        enum Constraints: LATTICE_CONSTRAINT_ALLOC => {
            type [IndirectIndex; 2];
            bind 0;
            shader render::debug::draw_lattices::SSBOI_CONSTRAINTS;
        };

        enum IMap_Nodes: LATTICE_NODE_ALLOC => {
            type IndirectIndex;
            bind 1;
            shader render::debug::draw_lattices::SSBOI_IMAP_NODES;
        };
        enum Nodes: LATTICE_CONSTRAINT_ALLOC => {
            type [f32; 4];
            bind 2;
            shader render::debug::draw_lattices::SSBOI_NODES;
        };
    }
}

ethel::typed_part_tribuffer! {
    const Fragments: 3, {
        enum Bind_Pose: FRAGMENTS_ALLOC => {
            type glam::Vec4;
            bind 0;
            shader render::geometry::G_FRAGS_SSBOI_BIND_POSE;
        };
        enum Mesh_Id: FRAGMENTS_ALLOC => {
            type ethel::mesh::Id;
            bind 1;
            shader render::geometry::G_FRAGS_SSBOI_MESH_ID;
        };
        enum Cage_Id: FRAGMENTS_ALLOC => {
            type IndirectIndex;
            bind 2;
            shader render::geometry::G_FRAGS_SSBOI_CAGE_ID;
        };
    }
}

ethel::typed_part_buffer! {
    const Cage: 8, {
        enum RMap: CAGES_ALLOC => {
            type IndirectIndex;
            bind 0;
        };

        enum Bind_Ref: CAGES_ALLOC => {
            type glam::Vec4;
            bind 1;
            shader render::task::deform_cages::SSBOI_BIND_REF;
        };
        enum Points: CAGES_ALLOC => {
            type CagePoints;
            bind 2;
            shader render::task::deform_cages::SSBOI_POINTS;
        };
        enum Points_Bind: CAGES_ALLOC => {
            type CagePoints;
            bind 3;
            shader render::task::deform_cages::SSBOI_POINTS_BIND;
        };
        enum Barycenters_Bind: CAGES_ALLOC => {
            type [glam::Vec4; PER_CAGE_POINTS];
            bind 4;
            shader render::task::deform_cages::SSBOI_BARYCENTERS_BIND;
        };
        enum Attachments: CAGES_ALLOC => {
            type [LatticeAttachments; PER_CAGE_POINTS];
            bind 5;
            shader render::task::deform_cages::SSBOI_ATTACHMENTS;
        };
        enum Lattice_Lut: CAGES_ALLOC => {
            type [IndirectIndex; PER_CAGE_MAX_LATTICE_ATTACHMENTS];
            bind 6;
            shader render::task::deform_cages::SSBOI_LATTICE_LUT;
        };
        enum Lattice_Bind: CAGES_ALLOC => {
            type [glam::Vec4; PER_CAGE_MAX_LATTICE_ATTACHMENTS];
            bind 7;
            shader render::task::deform_cages::SSBOI_LATTICE_BIND;
        };
    }
}

ethel::typed_part_tribuffer! {
    const Debris: 3, {
        enum Position: DEBRIS_ALLOC => {
            type [f32; 4];
            bind 0;
        };
        enum Rotation: DEBRIS_ALLOC => {
            type glam::Quat;
            bind 1;
        };
        enum Mesh_Id: DEBRIS_ALLOC => {
            type ethel::mesh::Id;
            bind 2;
        };
    }
}

#[cfg(feature = "devmode")]
ethel::typed_part_tribuffer! {
    const DbgLines: 2, {
        enum Point: DEBUG_LINES_ALLOC => {
            type [f32; 4];
            bind 0;
            shader render::debug::draw_lines::SSBOI_POINTS;
        };
        enum Color: DEBUG_LINES_ALLOC => {
            type glam::Vec4;
            bind 1;
            shader render::debug::draw_lines::SSBOI_COLORS;
        };
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct Overlays {
    pub damage: DamageOverlay,
}

#[derive(Debug, Default)]
pub struct FrameDataBuffers {
    // count of fragment geom entities
    pub fragment_geom_count: TriCell<u32>,
    // count of debris geom entities
    pub debris_geom_count: TriCell<u32>,

    pub fragments: FragmentsPartitionedTriBuffer,
    pub debris: DebrisPartitionedTriBuffer,

    pub cages: CagePartitionedBuffer,
    pub cage_map: TriBuffer<DirectIndex>,
    pub cage_points_count: Arc<AtomicU32>,
    pub cage_feedback: TriBuffer<OffsetRotation>,
    pub cage_sync_frame: CageSyncFrameBuffers,

    pub lattice_debug: LatticePartitionedTriBuffer,
    pub lattice_constraint_count: Arc<AtomicU32>,
    #[cfg(feature = "devmode")]
    pub lines_debug: DbgLinesPartitionedTriBuffer,

    pub interface_storage: UiDataBuffer,
    pub interface_commands: UiCommandsBuffer,

    pub render_frame_last_duration: TriCell<DeltaTime>,

    pub debug_material_index: TriCell<u32>,

    pub shading_attributes_mode: TriCell<Option<ShadingAttributesMode>>,
    pub render_params: RenderParams,
    pub render_stats: TriCell<RenderStats>,
    pub debug_render_options: TriCell<DebugRenderOptions>,

    pub groundplane: TriCell<Groundplane>,
    pub overlays: TriCell<Overlays>,
    pub mouse_world_position: TriCell<Option<glam::Vec3>>,
}
impl FrameDataBuffers {
    pub fn new() -> Self {
        Self {
            fragment_geom_count: TriCell::new(0),
            debris_geom_count: TriCell::new(0),

            fragments: FragmentsPartitionedTriBuffer::new(),
            debris: DebrisPartitionedTriBuffer::new(),

            cages: CagePartitionedBuffer::new(),
            cage_map: TriBuffer::zeroed(CAGES_ALLOC),
            cage_points_count: Arc::new(AtomicU32::new(0)),
            cage_feedback: TriBuffer::zeroed(CAGES_ALLOC),
            cage_sync_frame: CageSyncFrameBuffers::new(),

            lattice_debug: LatticePartitionedTriBuffer::new(),
            lattice_constraint_count: Arc::new(AtomicU32::new(0)),
            #[cfg(feature = "devmode")]
            lines_debug: DbgLinesPartitionedTriBuffer::new(),

            interface_commands: TriBuffer::zeroed(INTERFACE_COMMANDS_ALLOC),
            interface_storage: TriBuffer::zeroed(INTERFACE_INSTANCES_ALLOC),

            render_frame_last_duration: TriCell::new(DeltaTime::default()),

            debug_material_index: TriCell::new(0),
            shading_attributes_mode: TriCell::new(None),
            render_params: RenderParams::default(),
            render_stats: TriCell::new(RenderStats::default()),
            debug_render_options: Default::default(),
            groundplane: Default::default(),
            overlays: Default::default(),
            mouse_world_position: Default::default(),
        }
    }
}
