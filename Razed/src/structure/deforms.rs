use ethel::state::data::{
    Column, DirectIndex, IndirectIndex,
    hash::{Cell, FxSpatialHash},
    table::TableView,
};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
};

use crate::{
    procedural::VoxelGrid,
    structure::lattice::{DamagedNode, NodesRowTableView},
};

pub const CONTROL_POINTS_COUNT: usize = 4;

ethel::table_spec! {
    struct Deforms {
        deformed: glam::Vec3; // current deformed points
        pose: glam::Vec3; // the base points of the bind pose

        controllers: [ControlPoint; CONTROL_POINTS_COUNT];
        binds: [glam::Vec3; CONTROL_POINTS_COUNT];

        bind_info: BindInfo;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct BindInfo {
    pub barycenter: glam::Vec3,
    pub weight_sum: f32,
}

pub const CONTROL_POINT_MAX_RANGE: f32 = 16.0;

#[derive(Debug, Default)]
pub struct DeformSystem {
    data: DeformsRowTable,

    // before deletion
    damaged_buffer: Vec<DirectIndex>,
    // after deletion
    deleted_points: Vec<IndirectIndex>,

    /// Mapping of node controller points to owning deform point by IDs.
    node_map: Vec<Vec<IndirectIndex>>,
}

impl DeformSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: DeformsRowTable::with_capacity(capacity),
            damaged_buffer: Vec::new(),
            deleted_points: Vec::new(),
            node_map: Vec::new(),
        }
    }

    pub fn data(&self) -> &DeformsRowTable {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut DeformsRowTable {
        &mut self.data
    }

    pub fn node_mapping(&self) -> &[Vec<IndirectIndex>] {
        &self.node_map
    }

    /// Slice of indirect indices to all points that may be deleted.
    ///
    /// Tip: they probably should. You can do this with
    /// [`delete_dead_points`](Self::delete_dead_points);
    pub fn deleted_points_frame(&self) -> &[IndirectIndex] {
        &self.deleted_points
    }

    pub fn delete_dead_points(&mut self) {
        self.data.free_many(&self.deleted_points);
    }

    pub fn clear_damage_buffers(&mut self) {
        self.damaged_buffer.clear();
        self.deleted_points.clear();
    }

    pub fn sync_lattice_damage(&mut self, damage: &[DamagedNode]) {
        damage.iter().filter(|d| d.constraints_left == 0).for_each(
            |&DamagedNode { id, .. }| {
                let attached = &mut self.node_map[id.as_index()];
                self.data.free_many(attached);
                attached.clear();
            },
        );
    }

    pub fn deform(&mut self, lattice: &NodesRowTableView) {
        fn outer_product(a: glam::Vec3, b: glam::Vec3) -> glam::Mat3 {
            glam::mat3(a * b.x, a * b.y, a * b.z)
        }

        fn decompose_rotation_svd(cov: glam::Mat3) -> glam::Mat3 {
            fn nalg_to_glam(nalg: nalgebra::Matrix3<f32>) -> glam::Mat3 {
                let c0 = nalg.column(0);
                let c1 = nalg.column(1);
                let c2 = nalg.column(2);
                glam::Mat3::from_cols(
                    glam::vec3(c0.x, c0.y, c0.z),
                    glam::vec3(c1.x, c1.y, c1.z),
                    glam::vec3(c2.x, c2.y, c2.z),
                )
            }
            fn glam_to_nalg(glam: glam::Mat3) -> nalgebra::Matrix3<f32> {
                let c0 = glam.x_axis;
                let c1 = glam.y_axis;
                let c2 = glam.z_axis;
                nalgebra::Matrix3::from_columns(&[
                    nalgebra::Vector3::new(c0.x, c0.y, c0.z),
                    nalgebra::Vector3::new(c1.x, c1.y, c1.z),
                    nalgebra::Vector3::new(c2.x, c2.y, c2.z),
                ])
            }

            let nalg_cov = glam_to_nalg(cov);
            let nalg_svd = nalg_cov.svd(true, true);

            let u = nalg_to_glam(nalg_svd.u.unwrap());
            let v_t = nalg_to_glam(nalg_svd.v_t.unwrap());

            u * v_t
        }

        let deforms = &mut self.data.deformed;
        let pose = &self.data.pose;
        let bind_info = &self.data.bind_info;
        let controllers = &self.data.controllers;
        let node_binds = &self.data.binds;

        deforms
            .par_iter_mut()
            .zip(pose.par_iter().zip(bind_info))
            .zip(controllers.par_iter().zip(node_binds))
            .skip(1)
            .for_each(
                |((deform, (&pose, &bind_info)), (controllers, controller_binds))| {
                    let w_sum = bind_info.weight_sum;
                    let b_bar = bind_info.barycenter;

                    let p_bar = controllers
                        .iter()
                        .filter(|c| c.id.as_int() != 0 && c.weight > 0.001)
                        .fold(glam::Vec3::ZERO, |acc, &ControlPoint { id, weight }| {
                            acc + lattice.current_pos(id) * weight
                        })
                        / w_sum;

                    let covariance = controllers
                        .iter()
                        .zip(controller_binds)
                        .filter(|(c, _)| c.id.as_int() != 0 && c.weight > 0.001)
                        .fold(
                            glam::Mat3::ZERO,
                            |acc, (&ControlPoint { id, weight }, &bind_pos)| {
                                let real_pos = lattice.current_pos(id);
                                acc + outer_product(real_pos - p_bar, bind_pos - b_bar) * weight
                            },
                        )
                        + glam::Mat3::IDENTITY * 0.000005;

                    let rotation = decompose_rotation_svd(covariance);
                    *deform = rotation * (pose - b_bar) + p_bar;
                },
            );
    }

    pub fn generate_points(
        &mut self,
        origin: glam::Vec3,
        fragments: &VoxelGrid,
        node_hash: &FxSpatialHash<IndirectIndex>,
        lattice: &NodesRowTableView,
    ) -> std::ops::Range<usize> {
        let lattice_size = lattice.size();
        self.node_map.resize_with(lattice_size, || Vec::new());

        let (mx, my, mz) = fragments.dimensions();
        let total = mx * my * mz + mx + my + mz;

        let mut points = Vec::<DeformPoint>::with_capacity(total as usize);
        let mut near_buf = Vec::<Cell>::with_capacity(CONTROL_POINTS_COUNT);

        let bounds_x = (mx + 2) / 2;
        let bounds_y = (my + 2) / 2;
        let bounds_z = (mz + 2) / 2;
        for x in -bounds_x..bounds_x {
            for y in -bounds_y..bounds_y {
                for z in -bounds_z..bounds_z {
                    let cell = Cell { x, y, z };
                    let point = fragments.point_at_or_approx(cell) + origin;
                    points.push(self.create_deform(point, node_hash, lattice, &mut near_buf));
                }
            }
        }

        let r0 = self.data.len();
        points.drain(..).for_each(|deform| {
            let weight_sum = deform
                .controllers
                .iter()
                .filter(|c| c.id.as_int() != 0)
                .fold(0f32, |sum, &ControlPoint { weight, .. }| sum + weight);
            let bind_info = BindInfo {
                weight_sum,
                barycenter: deform
                    .controllers
                    .iter()
                    .zip(deform.binds)
                    .filter(|(c, _)| c.id.as_int() != 0)
                    .fold(
                        glam::Vec3::ZERO,
                        |acc, (&ControlPoint { weight, .. }, bind_pos)| acc + bind_pos * weight,
                    )
                    / weight_sum,
            };

            let handle = self.data.insert((
                deform.point,
                deform.point,
                deform.controllers,
                deform.binds,
                bind_info,
            ));

            deform
                .controllers
                .iter()
                .for_each(|ControlPoint { id, .. }| {
                    if id.as_int() != 0 {
                        self.node_map[id.as_index()].push(handle);
                    }
                });
        });
        let r1 = self.data.len();
        println!("done; {} deform points", r1 - r0);

        r0..r1
    }

    fn create_deform(
        &mut self,
        point: glam::Vec3,
        node_hash: &FxSpatialHash<IndirectIndex>,
        lattice: &NodesRowTableView,
        near_buf: &mut Vec<Cell>,
    ) -> DeformPoint {
        near_buf.clear();
        let max_range = (CONTROL_POINT_MAX_RANGE / node_hash.resolution.get()) as u32;
        let _ = node_hash.nearest_cells(
            node_hash.cell_at(point),
            CONTROL_POINTS_COUNT as u32,
            max_range,
            near_buf,
            false,
        );

        let mut controllers = [ControlPoint::default(); CONTROL_POINTS_COUNT];
        let mut binds = [glam::Vec3::ZERO; CONTROL_POINTS_COUNT];

        near_buf
            .drain(..)
            .take(CONTROL_POINTS_COUNT)
            .enumerate()
            .for_each(|(i, cell)| {
                let node = *node_hash.get(cell).expect("query is of populated node");

                // SAFETY:
                // we assume node_hash has been loaded with the nodes of
                // lattice, thus all handles are valid.
                let position = *unsafe { lattice.current_pos_unchecked(node) };
                let ds = point.distance(position);

                binds[i] = position;
                controllers[i] = ControlPoint {
                    id: node,
                    weight: 1.0 / (ds + f32::EPSILON),
                };
            });

        let w_t = controllers
            .iter()
            .fold(0f32, |w_t, ControlPoint { weight: w, .. }| w_t + *w);
        controllers
            .iter_mut()
            .for_each(|ControlPoint { weight, .. }| {
                *weight /= w_t;
            });

        DeformPoint {
            point,
            controllers,
            binds,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ControlPoint {
    pub id: IndirectIndex,
    pub weight: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct DeformPoint {
    point: glam::Vec3,
    controllers: [ControlPoint; CONTROL_POINTS_COUNT],
    binds: [glam::Vec3; CONTROL_POINTS_COUNT],
}
