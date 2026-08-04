//TODO: move all to compute, cpu only init and deformed positions pulling

use ethel::{
    data::{
        Column, IndirectIndex,
        hash::{Cell, FxSpatialHash},
        table::TableView,
    },
    shader::{Constant, GlslStruct, GlslUniform, ShaderProgram, WriteValue},
};

use crate::structure::lattice::NodesRowTableView;

pub const PER_POINT_LATTICE_ATTACHMENTS: usize = 4;
pub const PER_CAGE_MAX_LATTICE_ATTACHMENTS: usize = 8;
pub const PER_CAGE_POINTS: usize = 8;
pub const CAGE_DIAG_EXTENT: f32 = 1.5;
pub const QUERY_LATTICE_ATTACH_MAX_RANGE: f32 = 32.0;

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
        point_lattice_attachments: [LatticeAttachments; PER_CAGE_POINTS];
        attached_lattice: [IndirectIndex; PER_CAGE_MAX_LATTICE_ATTACHMENTS];
        lattice_bind_points: [glam::Vec3; PER_CAGE_MAX_LATTICE_ATTACHMENTS];
    }
}

const CAGE_ALLOC: usize = 1;

ethel::typed_part_buffer! {
    const Cage: 8, {
        enum Pod_Rotation: CAGE_ALLOC => {
            type glam::Quat;
            bind 0;
            init with {
                glam::Quat::IDENTITY
            };
        };
        enum Pod_BindRef: CAGE_ALLOC => {
            type glam::Vec4;
            bind 1;
        };
        enum Pod_Points: CAGE_ALLOC => {
            type CagePoints;
            bind 2;
        };
        enum Pod_Points_Bind: CAGE_ALLOC => {
            type CagePoints;
            bind 3;
        };
        enum Pod_Barycenter_Bind: CAGE_ALLOC => {
            type [glam::Vec4; PER_CAGE_POINTS];
            bind 4;
        };
        enum Pod_Attachments: CAGE_ALLOC => {
            type [LatticeAttachments; PER_CAGE_POINTS];
            bind 5;
        };
        enum Pod_Lut_Lattice: CAGE_ALLOC => {
            type [IndirectIndex; PER_CAGE_MAX_LATTICE_ATTACHMENTS];
            bind 6;
        };
        enum Pod_Bind_Lattice: CAGE_ALLOC => {
            type [glam::Vec4; PER_CAGE_MAX_LATTICE_ATTACHMENTS];
            bind 7;
        };
    }
}

pub const TYPE_CAGE_POINTS_LIST: GlslStruct = CagePointsGlslStruct::as_definition();
pub const TYPE_CAGE_POINT_ATTACHMENT_NODE: GlslStruct = NodeAttachmentGlslStruct::as_definition();
pub const TYPE_CAGE_POINT_ATTACHMENTS_LIST: GlslStruct =
    LatticeAttachmentsGlslStruct::as_definition();

ethel::shader_glsl_struct! {
    struct CagePoints {
        list[8]: [glam::Vec4; PER_CAGE_POINTS] => vec4;
    }
}
ethel::shader_glsl_struct! {
    struct NodeAttachment {
        index: u32 => uint;
        weight: f32 => float;
    }
}
ethel::shader_glsl_struct! {
    struct LatticeAttachments {
        list[4]: [NodeAttachment; PER_POINT_LATTICE_ATTACHMENTS] => NodeAttachment;
    }
}

struct Cage {
    rotation: [glam::Quat; PER_CAGE_POINTS],
    world_bind_reference: [f32; 4],

    local_points: CagePoints,
    local_points_bind: CagePoints,

    point_barycenter: [glam::Vec4; PER_CAGE_POINTS],
    point_lattice_attachments: [LatticeAttachments; PER_CAGE_POINTS],
    attached_lattice: [IndirectIndex; PER_CAGE_MAX_LATTICE_ATTACHMENTS],
    lattice_bind_points: [glam::Vec4; PER_CAGE_MAX_LATTICE_ATTACHMENTS],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CagePoints(pub [glam::Vec4; PER_CAGE_POINTS]);

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LatticeAttachments(pub [NodeAttachment; PER_POINT_LATTICE_ATTACHMENTS]);

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct NodeAttachment {
    /// Index into cage's attach_lattice array
    pub index: u32,
    pub weight: f32,
}
impl WriteValue for NodeAttachment {
    fn write_value(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(to, "NodeAttachment({}, {})", self.index, self.weight)
    }
}

macro_rules! ssbo_binding {
    (Pod_Rotation) => {
        0
    };
    (Pod_BindRef) => {
        1
    };
    (Pod_Points) => {
        2
    };
    (Pod_Points_Bind) => {
        3
    };
    (Pod_Barycenter_Bind) => {
        4
    };
    (Pod_Attachments) => {
        5
    };
    (Pod_Lut_Lattice) => {
        6
    };
    (Pod_Bind_Lattice) => {
        7
    };
    (IMap_Lattice) => {
        8
    };
    (Pod_Lattice_Position) => {
        9
    };
}

pub const SSBO_INDEX_POD_ROTATION: u32 = ssbo_binding!(Pod_Rotation);
pub const SSBO_INDEX_POD_BIND_REF: u32 = ssbo_binding!(Pod_BindRef);
pub const SSBO_INDEX_POD_POINTS: u32 = ssbo_binding!(Pod_Points);
pub const SSBO_INDEX_POD_POINTS_BIND: u32 = ssbo_binding!(Pod_Points_Bind);
pub const SSBO_INDEX_POD_BARYCENTER_BIND: u32 = ssbo_binding!(Pod_Barycenter_Bind);
pub const SSBO_INDEX_POD_ATTACHMENTS: u32 = ssbo_binding!(Pod_Attachments);
pub const SSBO_INDEX_POD_LUT_LATTICE: u32 = ssbo_binding!(Pod_Lut_Lattice);
pub const SSBO_INDEX_POD_BIND_LATTICE: u32 = ssbo_binding!(Pod_Bind_Lattice);
pub const SSBO_INDEX_IMAP_LATTICE: u32 = ssbo_binding!(IMap_Lattice);
pub const SSBO_INDEX_POD_LATTICE_POSITION: u32 = ssbo_binding!(Pod_Lattice_Position);

pub const CAGE_DEFORM_WORKGROUP_SIZE: u32 = 64;
pub const CAGE_DEFORM_PER_GROUP_CAGE_COUNT: u32 = 8;
pub const CAGE_DEFORM_PER_POINT_ATTACH_COUNT: u32 = PER_POINT_LATTICE_ATTACHMENTS as u32;

ethel::shader_glsl_compute! {
    struct CageDeform > [460] {
        workgroup [64, 1, 1];

        uniform {
            length 1, total_cage_count: uint => u32;
        };

        type {
            crate::render::shader_commons::TYPE_INDEX_INDIRECT
            crate::render::shader_commons::TYPE_INDEX_DIRECT
            TYPE_CAGE_POINT_ATTACHMENT_NODE
            TYPE_CAGE_POINT_ATTACHMENTS_LIST
            TYPE_CAGE_POINTS_LIST
        };

        ssbo {
            ethel::shader_glsl_ssbo! {
                buf Pod_Rotation => {
                    [dyn_array vec4: pod_cage_rotation => each 8] // 8 is per-cage-points
                }
            }
            ethel::shader_glsl_ssbo! {
                buf Pod_BindRef => {
                    [dyn_array vec4: pod_cage_bindref]
                }
            }
            ethel::shader_glsl_ssbo! {
                buf Pod_Points => {
                    [dyn_array vec4: pod_cage_points => each 8] // 8 is per-cage-points
                }
            }
            ethel::shader_glsl_ssbo! {
                buf Pod_Points_Bind => {
                    [dyn_array vec4: pod_cage_points_bind => each 8] // 8 is per-cage-points
                }
            }
            ethel::shader_glsl_ssbo! {
                buf Pod_Barycenter_Bind => {
                    [dyn_array vec4: pod_cage_barycenter_bind => each 8] // 8 is per-cage-points
                }
            }
            ethel::shader_glsl_ssbo! {
                buf Pod_Attachments => {
                    [dyn_array LatticeAttachments: pod_cage_attachments => each 8] // 8 is per-cage-points
                }
            }
            ethel::shader_glsl_ssbo! {
                buf Pod_Lut_Lattice => {
                    [dyn_array IndirectIndex: pod_cage_lut_lattice => each 8] // 8 is per-cage-max-attachments
                }
            }
            ethel::shader_glsl_ssbo! {
                buf Pod_Bind_Lattice => {
                    [dyn_array vec4: pod_cage_lattice_bind => each 8] // 8 is per-cage-max-attachments
                }
            }
            ethel::shader_glsl_ssbo! {
                buf IMap_Lattice => {
                    [dyn_array DirectIndex: imap_lattice]
                }
            }
            ethel::shader_glsl_ssbo! {
                buf Pod_Lattice_Position => {
                    [dyn_array vec4: pod_lattice_position]
                }
            }
        };

        const {
            Constant::new("PER_GROUP_CAGE_COUNT", CAGE_DEFORM_PER_GROUP_CAGE_COUNT)
            Constant::new("PER_POINT_ATTACH_COUNT", CAGE_DEFORM_PER_POINT_ATTACH_COUNT)
        };

        lib {
            crate::render::shader_commons::LIB_QUAT_MUL_QUAT;
            crate::render::shader_commons::LIB_QUAT_ROT_VEC;
            crate::render::shader_commons::LIB_VEC3_OUTER;
            crate::render::shader_commons::LIB_MAT3_CONVERT_QUAT;
            crate::render::pass::LIB_SVD_EXTRACT_ROTATION;
        };

        share {
            vec3 sm_lattice_pos[PER_GROUP_CAGE_COUNT][8];
        };

        src() {
            "
            uint local       = gl_LocalInvocationID.x;
            uint cage_local_index  = local / PER_GROUP_CAGE_COUNT;
            uint point_local_index = local % PER_GROUP_CAGE_COUNT;
            uint cage_global_index = gl_WorkGroupID.x * PER_GROUP_CAGE_COUNT + cage_local_index;

            // since the number shader invocation per cage is the same as the
            // number of maximum attached lattice nodes, we can cooperatively
            // load 1 real-time lattice position only once and store it in the
            // workgroup's shared memory, saving a double-lookup.
            uint logic_local_index = point_local_index;
            IndirectIndex cage_lut_lattice[8] = pod_cage_lut_lattice[cage_global_index];
            if (cage_global_index < total_cage_count) {
                IndirectIndex id   = cage_lut_lattice[logic_local_index];
                DirectIndex direct = imap_lattice[id.index];
                vec3 node_position = pod_lattice_position[direct.index].xyz;
                sm_lattice_pos[cage_local_index][logic_local_index] = node_position;
            } else {
                // if the thread is outside the working range just fake it
                // till the barrier, zero it out because why not.
                sm_lattice_pos[cage_local_index][logic_local_index] = vec3(0.0);
            }

            barrier();

            if (cage_global_index >= total_cage_count) return;

            vec3 cage_bind_ref = pod_cage_bindref[cage_global_index].xyz;
            vec4 cage_lattice_binds[8] = pod_cage_lattice_bind[cage_global_index]; // shared per-cage lattice bind-pos cache
            vec4 point_barycenter_binds[8] = pod_cage_barycenter_bind[cage_global_index]; // per-point bind-lattice barycenter
            vec3 bind_barycenter = point_barycenter_binds[point_local_index].xyz;

            LatticeAttachments cage_attachments[8] = pod_cage_attachments[cage_global_index];
            NodeAttachment point_attachments[PER_POINT_ATTACH_COUNT] = cage_attachments[point_local_index].list;

            vec3 shared_cage_lattice_pos[8] = sm_lattice_pos[cage_local_index];

            // compute real-time barycenter
            vec3 real_barycenter = vec3(0.0);
            for (uint i = 0; i < PER_POINT_ATTACH_COUNT; ++i) {
                NodeAttachment attachment = point_attachments[i];
                vec3 real_node_pos = shared_cage_lattice_pos[attachment.index];
                real_barycenter += real_node_pos * attachment.weight;
            }
            real_barycenter -= cage_bind_ref;

            mat3 covariance = mat3(0.0);
            for (uint i = 0; i < PER_POINT_ATTACH_COUNT; ++i) {
                NodeAttachment attachment = point_attachments[i];

                vec3 real_node_pos = shared_cage_lattice_pos[attachment.index];
                vec3 bind_node_pos = cage_lattice_binds[attachment.index].xyz;

                covariance += outer(
                    real_node_pos - real_barycenter,
                    bind_node_pos - bind_barycenter
                ) * attachment.weight;
            }
            const mat3 MAT3_IDENTITY = mat3(vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0), vec3(0.0, 0.0, 1.0));
            covariance += MAT3_IDENTITY * 0.0001;

            mat3 rotation_mat = svdExtractRotation(covariance);
            vec4 rotation = matToQuat(rotation_mat);

            vec4 cage_rotations[8] = pod_cage_rotation[cage_global_index];
            cage_rotations[point_local_index] = rotation;

            vec4 cage_points_bind[8] = pod_cage_points_bind[cage_global_index];
            vec3 point_bind = cage_points_bind[point_local_index].xyz;
            vec3 deformed = rotateQuat(point_bind - bind_barycenter, rotation) + real_barycenter;

            pod_cage_points[cage_global_index][point_local_index] = vec4(deformed, 1.0);
            ";
        }
    }
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
            for (i, &node) in lattice_nodes.iter().enumerate() {
                let node_pos = *lattice_data.current_pos(node);
                lattice_current_pos[i] = node_pos;
            }

            let bind_ref = glam::Vec3::from_array(*reference);

            for ((attachments, covariant), (bind_bary, real_bary)) in attachments
                .iter()
                .zip(cov)
                .zip(bind_barys.iter().zip(real_barys))
            {
                *real_bary = glam::Vec3::ZERO;
                for &NodeAttachment { index, weight } in &attachments.0 {
                    let node_pos = lattice_current_pos[index as usize];
                    *real_bary += node_pos * weight;
                }
                *real_bary -= bind_ref;

                let mut cov3 = glam::Mat3::ZERO;
                for &NodeAttachment { index, weight } in &attachments.0 {
                    let node_real_pos = lattice_current_pos[index as usize] - bind_ref;
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
        let points_attachments = cage_data
            .points
            .map(|p| LatticeAttachments(p.lattice_attachments));
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
        let mut sorted_lattice_points = {
            let mut i = 0;
            lattice_points.map(|p| {
                let e = (i, p);
                i += 1;
                e
            })
        };
        sorted_lattice_points.sort_by(|(_, a), (_, b)| {
            let da = a.distance_squared(point);
            let db = b.distance_squared(point);
            da.total_cmp(&db)
        });

        let mut attachments = [NodeAttachment::default(); PER_POINT_LATTICE_ATTACHMENTS];
        let mut lattice_barycenter = glam::Vec3::ZERO;
        let mut lattice_weights_sum = 0f32;

        // process nearby lattice attachment points
        sorted_lattice_points
            .iter()
            .take(PER_POINT_LATTICE_ATTACHMENTS)
            .enumerate()
            .for_each(|(i, &(original_index, pos))| {
                let distance = point.distance_squared(pos);
                let weight = 1.0 / (distance + 0.0001);
                lattice_weights_sum += weight;
                attachments[i] = NodeAttachment {
                    index: original_index as u32,
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
            .for_each(|&NodeAttachment { index, weight }| {
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
    pub lattice_attachments: [NodeAttachment; PER_POINT_LATTICE_ATTACHMENTS],
    pub lattice_barycenter: glam::Vec3,
    pub weight_sum: f32,
}
