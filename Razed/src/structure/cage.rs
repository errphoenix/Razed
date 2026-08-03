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
        // calculated on gpu compute shader, used for ffd
        rotation: [glam::Quat; PER_CAGE_POINTS];
        covariant: [glam::Mat4; PER_CAGE_POINTS];

        world_bind_reference: [f32; 3];

        // vec4 for gpu alignment as this data is used in shaders
        // * local_points is used as the output of cage deformation
        //   compute, and ffd for fragments
        // * local_points_bind is used for ffd for fragments
        local_points: CagePoints;
        local_points_bind: CagePoints;

        // no alignment requirements as this data is cpu-only
        // for covariant computation
        point_barycenter_lattice_bind: [glam::Vec3; PER_CAGE_POINTS];
        point_barycenter_lattice_real: [glam::Vec3; PER_CAGE_POINTS];
        point_lattice_attachments: [PointLatticeAttachments; PER_CAGE_POINTS];
        attached_lattice: [IndirectIndex; PER_CAGE_MAX_LATTICE_ATTACHMENTS];
        lattice_bind_points: [glam::Vec3; PER_CAGE_MAX_LATTICE_ATTACHMENTS];
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CagePoints(pub [glam::Vec4; PER_CAGE_POINTS]);

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PointLatticeAttachments {
    pub attached_nodes: [LatticeAttachment; PER_POINT_LATTICE_ATTACHMENTS],
    pub weight_sum: f32,
}

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

    pub fn apply_rotations(&mut self) {
        let bind_points = &self.data.local_points_bind;
        let rotations = &self.data.rotation;
        let point_bind_barys = &self.data.point_barycenter_lattice_bind;
        let point_real_barys = &self.data.point_barycenter_lattice_real;
        let points = &mut self.data.local_points;

        for (((points, binds), rotations), (bind_barys, real_barys)) in points
            .iter_mut()
            .zip(bind_points)
            .zip(rotations)
            .zip(point_bind_barys.iter().zip(point_real_barys))
        {
            points
                .0
                .iter_mut()
                .zip(binds.0.iter())
                .zip(rotations)
                .zip(bind_barys.iter().zip(real_barys))
                .for_each(|(((pos, bind), &rot), (&b_bary, &r_bary))| {
                    let bind = glam::vec3(bind.x, bind.y, bind.z);
                    let deformed = rot * (bind - b_bary) + r_bary;
                    *pos = glam::vec4(deformed.x, deformed.y, deformed.z, 1.0);
                });
        }
    }

    pub fn compute_covariants(&mut self, lattice_data: &NodesRowTableView) {
        fn outer_product(a: glam::Vec3, b: glam::Vec3) -> glam::Mat3 {
            glam::mat3(a * b.x, a * b.y, a * b.z)
        }

        let lattice = &self.data.attached_lattice;
        let lattice_binds = &self.data.lattice_bind_points;
        let point_attached_lattice = &self.data.point_lattice_attachments;
        let bind_barycenters = &self.data.point_barycenter_lattice_bind;
        let cage_reference = &self.data.world_bind_reference;
        let real_barycenters = &mut self.data.point_barycenter_lattice_real;
        let covariants = &mut self.data.covariant;

        for (
            cov,
            (
                (((lattice_nodes, lattice_bind_points), attachments), (bind_barys, real_barys)),
                reference,
            ),
        ) in covariants.iter_mut().zip(
            lattice
                .iter()
                .zip(lattice_binds)
                .zip(point_attached_lattice)
                .zip(bind_barycenters.iter().zip(real_barycenters))
                .zip(cage_reference)
                .skip(1),
        ) {
            let mut lattice_current_pos = [glam::Vec3::ZERO; PER_CAGE_MAX_LATTICE_ATTACHMENTS];
            let bind_ref = glam::Vec3::from_array(*reference);

            for ((attachments, covariant), (bind_bary, real_bary)) in attachments
                .iter()
                .zip(cov)
                .zip(bind_barys.iter().zip(real_barys))
            {
                *real_bary = glam::Vec3::ZERO;
                for &LatticeAttachment { index, weight } in &attachments.attached_nodes {
                    let node_id = lattice_nodes[index as usize];
                    let node_pos = *lattice_data.current_pos(node_id) - bind_ref;
                    lattice_current_pos[index as usize] = node_pos;
                    *real_bary += node_pos * weight;
                }
                *real_bary /= attachments.weight_sum;

                let mut cov3 = glam::Mat3::ZERO;
                for &LatticeAttachment { index, weight } in &attachments.attached_nodes {
                    let node_real_pos = lattice_current_pos[index as usize];
                    let node_bind_pos = lattice_bind_points[index as usize];

                    let real_com = node_real_pos - *real_bary;
                    let bind_com = node_bind_pos - bind_bary;

                    cov3 += outer_product(real_com, bind_com) * weight;
                }
                cov3 += glam::Mat3::IDENTITY * 0.00005;

                *covariant = glam::Mat4::from_mat3(cov3);
            }
        }
    }

    pub fn generate_cage(
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
        let points_barycenter_bind = cage_data.points.map(|p| p.lattice_barycenter - cage_center);
        let points_attachments = cage_data.points.map(|p| PointLatticeAttachments {
            attached_nodes: p.lattice_attachments,
            weight_sum: p.weight_sum,
        });
        let lattice_bind_pos = cage_data.lattice_bind_pos.map(|p| p - cage_center);
        let cage_reference = cage_center.to_array();

        self.data.insert((
            [glam::Quat::IDENTITY; PER_CAGE_POINTS],
            [glam::Mat4::IDENTITY; PER_CAGE_POINTS],
            cage_reference,
            CagePoints(points_pos),
            CagePoints(points_pos),
            points_barycenter_bind,
            points_barycenter_bind,
            points_attachments,
            cage_data.attached_lattice,
            lattice_bind_pos,
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
            near_buf
                .drain(..)
                .take(PER_CAGE_MAX_LATTICE_ATTACHMENTS)
                .enumerate()
                .for_each(|(i, cell)| {
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

        let lattice_weights_sums = points.map(|p| p.weight_sum);

        CageData {
            points,
            attached_lattice,
            lattice_bind_pos,
            lattice_weights_sums,
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
            weight_sum: lattice_weights_sum,
            lattice_barycenter,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CageData {
    pub points: [CagePointData; PER_CAGE_POINTS],
    pub attached_lattice: [IndirectIndex; PER_CAGE_MAX_LATTICE_ATTACHMENTS],
    pub lattice_bind_pos: [glam::Vec3; PER_CAGE_MAX_LATTICE_ATTACHMENTS],
    pub lattice_weights_sums: [f32; PER_CAGE_POINTS],
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CagePointData {
    pub world_point: glam::Vec3,
    pub lattice_attachments: [LatticeAttachment; PER_POINT_LATTICE_ATTACHMENTS],
    pub lattice_barycenter: glam::Vec3,
    pub weight_sum: f32,
}
