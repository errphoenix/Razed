use ethel::data::IndirectIndex;

pub const CONTROL_POINTS_COUNT: usize = 4;
pub const PER_CAGE_POINTS: usize = 8;

ethel::table_spec! {
    struct Cage {
        lattice_bind: LatticeBind;
        lattice_attachments: [CageLatticeAttachment; CONTROL_POINTS_COUNT];
        points: [glam::Vec4; PER_CAGE_POINTS];
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct LatticeBind {
    pub positions: [glam::Vec3; CONTROL_POINTS_COUNT],
    pub barycenter: glam::Vec3,
    pub weight_sum: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CageLatticeAttachment {
    pub id: IndirectIndex,
    pub weight: f32,
    _pad: f32,
}

#[derive(Debug, Default)]
pub struct CageSystem {
    data: CageRowTable,

    /// Mapping of lattice node point ID to cage ID attached to the node.
    node_map: Vec<IndirectIndex>,
}
impl CageSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: CageRowTable::with_capacity(capacity),
            node_map: Vec::new(),
        }
    }

    pub fn data(&self) -> &CageRowTable {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut CageRowTable {
        &mut self.data
    }
}
