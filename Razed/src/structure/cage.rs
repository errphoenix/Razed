use ethel::data::{
    Column, IndirectIndex,
    hash::{Cell, FxSpatialHash},
    table::TableView,
};

use crate::structure::lattice::NodesRowTableView;

pub const PER_POINT_LATTICE_ATTACHMENTS: usize = 4;
pub const PER_CAGE_MAX_LATTICE_ATTACHMENTS: usize = 8;
pub const PER_CAGE_POINTS: usize = 8;
pub const CAGE_DIAG_EXTENT: f32 = 0.5;
pub const QUERY_LATTICE_ATTACH_MAX_RANGE: f32 = 16.0;

ethel::table_spec! {
    struct Cage {
        // vec4 for gpu alignment as this data is used in shaders
        // * local_points is used as the output of cage deformation
        //   compute, and ffd for fragments
        // * local_points_bind is used for ffd for fragments
        local_points: [glam::Vec4; PER_CAGE_POINTS];
        local_points_bind: [glam::Vec4; PER_CAGE_POINTS];

        // no alignment requirements as this data is cpu-only
        // this is all used to compute the covariant, it is cheap
        // enough that it is fine to keep on the cpu
        point_barycenter_lattice: [glam::Vec3; PER_CAGE_POINTS];
        lattice_attachments: [PointLatticeAttachments; PER_CAGE_POINTS];
        lattice_bind_points: [glam::Vec3; PER_CAGE_MAX_LATTICE_ATTACHMENTS];
        attached_lattice: [IndirectIndex; PER_CAGE_MAX_LATTICE_ATTACHMENTS];
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PointLatticeAttachments(pub [LatticeAttachment; PER_POINT_LATTICE_ATTACHMENTS]);

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LatticeAttachment {
    /// Index into cage's attach_lattice array
    pub index: u32,
    pub weight: f32,
}

#[derive(Debug, Default)]
pub struct CageSystem {
    data: CageRowTable,

    /// Mapping of lattice node point ID to cage ID attached to the node.
    node_map: Vec<IndirectIndex>,

    generate_query_near_buf: Vec<Cell>,
}
impl CageSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: CageRowTable::with_capacity(capacity),
            node_map: Vec::new(),
            generate_query_near_buf: Vec::with_capacity(PER_CAGE_MAX_LATTICE_ATTACHMENTS),
        }
    }

    pub fn data(&self) -> &CageRowTable {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut CageRowTable {
        &mut self.data
    }

    pub fn generate_cage_from_lattice(
        &mut self,
        cage_center: glam::Vec3,
        lattice_hash: &FxSpatialHash<IndirectIndex>,
        lattice: &NodesRowTableView,
    ) -> IndirectIndex {
        let lattice_size = lattice.size();
        self.node_map.resize(lattice_size, Default::default());

        let near_buf = &mut self.generate_query_near_buf;
        let cage_data = Self::create_cage(
            cage_center,
            lattice_hash,
            lattice,
            near_buf,
            CAGE_DIAG_EXTENT,
        );

        let points_pos = cage_data.points.map(|p| {
            let local = p.world_point - cage_center;
            glam::vec4(local.x, local.y, local.z, 1.0)
        });
        let points_barycenter = cage_data.points.map(|p| p.lattice_barycenter);
        let points_attachments = cage_data
            .points
            .map(|p| PointLatticeAttachments(p.lattice_attachments));

        self.data.insert((
            points_pos,
            points_pos,
            points_barycenter,
            points_attachments,
            cage_data.lattice_bind_pos,
            cage_data.attached_lattice,
        ))
    }

    fn create_cage(
        center: glam::Vec3,
        lattice_hash: &FxSpatialHash<IndirectIndex>,
        lattice: &NodesRowTableView,
        near_buf: &mut Vec<Cell>,
        cage_diag_half_extent: f32,
    ) -> CageData {
        let (attached_lattice, lattice_bind_pos) = {
            let mut attached_lattice = [IndirectIndex::default(); PER_CAGE_MAX_LATTICE_ATTACHMENTS];
            let mut lattice_bind_pos = [glam::Vec3::ZERO; PER_CAGE_MAX_LATTICE_ATTACHMENTS];

            let _ = lattice_hash.nearest_cells(
                lattice_hash.cell_at(center),
                PER_CAGE_MAX_LATTICE_ATTACHMENTS as u32,
                (QUERY_LATTICE_ATTACH_MAX_RANGE / lattice_hash.resolution.get()) as u32,
                near_buf,
                false,
            );
            near_buf.drain(..).enumerate().for_each(|(i, cell)| {
                let id = *lattice_hash.get(cell).unwrap();
                let position = lattice.current_pos(id);
                attached_lattice[i] = id;
                lattice_bind_pos[i] = *position;
            });

            (attached_lattice, lattice_bind_pos)
        };

        // anchor order is guaranteed to be:
        // 0: -x, -y, -z,
        // 1:  x, -y, -z,
        // 2: -x,  y, -z,
        // 3:  x,  y, -z,
        // 4: -x, -y,  z,
        // 5:  x, -y,  z,
        // 6: -x,  y,  z,
        // 7:  x,  y,  z,
        let c = cage_diag_half_extent;
        let p000 = center - glam::Vec3::splat(c);
        let p100 = center + glam::vec3(c, -c, -c);
        let p010 = center + glam::vec3(-c, c, -c);
        let p110 = center + glam::vec3(c, c, -c);
        let p001 = center + glam::vec3(-c, -c, c);
        let p101 = center + glam::vec3(c, -c, c);
        let p011 = center + glam::vec3(-c, c, c);
        let p111 = center + glam::Vec3::splat(c);

        let points = [
            Self::create_cage_point(p000, &lattice_bind_pos),
            Self::create_cage_point(p100, &lattice_bind_pos),
            Self::create_cage_point(p010, &lattice_bind_pos),
            Self::create_cage_point(p110, &lattice_bind_pos),
            Self::create_cage_point(p001, &lattice_bind_pos),
            Self::create_cage_point(p101, &lattice_bind_pos),
            Self::create_cage_point(p011, &lattice_bind_pos),
            Self::create_cage_point(p111, &lattice_bind_pos),
        ];

        CageData {
            points,
            attached_lattice,
            lattice_bind_pos,
        }
    }

    fn create_cage_point(
        point: glam::Vec3,
        lattice_points: &[glam::Vec3; PER_CAGE_MAX_LATTICE_ATTACHMENTS],
    ) -> CagePointData {
        // sort lattice nodes near cage by proximity to this cage point
        // we only work on the N nearest points that are related to the cage
        let mut sorted_lattice_points = lattice_points.clone();
        sorted_lattice_points.sort_by(|a, b| {
            let da = a.distance_squared(point);
            let db = b.distance_squared(point);
            da.total_cmp(&db).reverse()
        });

        let mut attachments = [LatticeAttachment::default(); PER_POINT_LATTICE_ATTACHMENTS];
        let mut lattice_barycenter = glam::Vec3::ZERO;
        let mut lattice_weights_sum = 0f32;

        // process nearby lattice attachment points
        sorted_lattice_points
            .iter()
            .take(PER_POINT_LATTICE_ATTACHMENTS)
            .enumerate()
            .for_each(|(i, &pos)| {
                let distance = point.distance(pos);
                let weight = 1.0 / (distance + 0.0001);
                lattice_weights_sum += weight;
                attachments[i] = LatticeAttachment {
                    index: i as u32,
                    weight,
                };
            });

        // normalize lattice attachment weights
        attachments
            .iter_mut()
            .for_each(|attachment| attachment.weight /= lattice_weights_sum);

        // compute barycenter from attached nodes
        attachments
            .iter()
            .for_each(|&LatticeAttachment { index, weight }| {
                let pos = lattice_points[index as usize];
                lattice_barycenter += pos * weight;
            });

        CagePointData {
            world_point: point,
            lattice_attachments: attachments,
            lattice_barycenter,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CageData {
    pub points: [CagePointData; PER_CAGE_POINTS],
    pub attached_lattice: [IndirectIndex; PER_CAGE_MAX_LATTICE_ATTACHMENTS],
    pub lattice_bind_pos: [glam::Vec3; PER_CAGE_MAX_LATTICE_ATTACHMENTS],
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CagePointData {
    world_point: glam::Vec3,
    lattice_attachments: [LatticeAttachment; PER_POINT_LATTICE_ATTACHMENTS],
    lattice_barycenter: glam::Vec3,
}
