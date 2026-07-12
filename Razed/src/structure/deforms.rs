use ethel::state::data::{
    Column, DirectIndex, IndirectIndex,
    hash::{Cell, FxSpatialHash},
    table::TableView,
};

use crate::{procedural::VoxelGrid, structure::lattice::NodesRowTableView};

pub const CONTROL_POINTS_COUNT: usize = 4;
pub const CONTROL_POINTS_MIN_THRESHOLD: usize = 1;

ethel::table_spec! {
    struct Deforms {
        deformed: glam::Vec3; // current deformed points
        pose: glam::Vec3; // the base points of the bind pose

        controllers: [ControlPoint; CONTROL_POINTS_COUNT];
        binds: [glam::Vec3; CONTROL_POINTS_COUNT];
    }
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

    pub fn sync_lattice_damage(
        &mut self,
        broken_nodes: &[IndirectIndex],
        lattice: &NodesRowTableView,
    ) {
        broken_nodes.iter().for_each(|node| {
            if let Some(deforms) = self.node_map.get_mut(node.as_index()) {
                deforms.iter_mut().for_each(|deform| {
                    if deform.as_int() != 0 {
                        if let Some(direct) = self.data.solve_indirect(*deform) {
                            for ControlPoint { id, weight } in
                                &mut self.data.controllers[direct.as_index()]
                            {
                                if *id == *node {
                                    *id = IndirectIndex::default();
                                    *weight = 0.0;
                                    self.damaged_buffer.push(direct);
                                    break;
                                }
                            }
                        }
                    }
                });
            }
        });

        self.damaged_buffer.retain(|direct| {
            let controllers = &mut self.data.controllers[direct.as_index()];
            let binds = &mut self.data.binds[direct.as_index()];

            // update bind pose to current position
            let deformed = self.data.deformed[direct.as_index()];
            self.data.pose[direct.as_index()] = deformed;

            controllers.iter_mut().zip(binds.iter_mut()).for_each(
                |(ControlPoint { id, weight }, bind)| {
                    if id.as_int() != 0 {
                        *bind = *lattice.current_pos(*id);
                        let ds = deformed.distance_squared(*bind);
                        *weight = 1.0 / (ds + f32::EPSILON);
                    }
                },
            );

            let w_t = controllers.iter_mut().fold(0f32, |s, ctl| s + ctl.weight);
            controllers
                .iter_mut()
                .for_each(|ControlPoint { weight, .. }| *weight /= w_t);
            let count = controllers
                .iter()
                .filter(|ctl| ctl.id.as_int() != 0)
                .count();

            count <= CONTROL_POINTS_MIN_THRESHOLD
        });

        // temporarily solve indices (direct to indirect)
        self.damaged_buffer.iter_mut().for_each(|i| {
            let indirect = self.data.handles()[i.as_index()];
            *i = DirectIndex::from_index(indirect.as_index(), indirect.generation());
        });

        // use indirect to free
        self.damaged_buffer.drain(..).for_each(|indirect| {
            let ii = IndirectIndex::from_index(indirect.as_index(), indirect.generation());
            self.data.free(ii);
            self.deleted_points.push(ii);
        });
    }

    pub fn deform(&mut self, lattice: &NodesRowTableView) {
        fn decompose_rotation_polar<const ITER: usize>(cov: glam::Mat3) -> glam::Mat3 {
            let mut r = cov;
            for _ in 0..ITER {
                let ri = r.try_inverse();
                if ri.is_none() {
                    return glam::Mat3::IDENTITY;
                }
                let ri = ri.unwrap();
                r = (r + ri.transpose()) * 0.5;
            }
            r
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

        fn outer_product(a: glam::Vec3, b: glam::Vec3) -> glam::Mat3 {
            glam::Mat3::from_cols(a * b.x, a * b.y, a * b.z)
        }

        let deforms = &mut self.data.deformed;
        let pose = &self.data.pose;
        let controllers = &self.data.controllers;
        let node_binds = &self.data.binds;

        for ((deform, &pose), (controllers, controller_binds)) in deforms
            .iter_mut()
            .zip(pose)
            .zip(controllers.iter().zip(node_binds))
            .skip(1)
        {
            let w_sum = controllers
                .iter()
                .filter(|c| c.id.as_int() != 0)
                .fold(0f32, |sum, &ControlPoint { weight, .. }| sum + weight);
            if w_sum < 0.0001 {
                continue;
            }

            let b_bar = controllers
                .iter()
                .zip(controller_binds)
                .filter(|(c, _)| c.id.as_int() != 0)
                .fold(
                    glam::Vec3::ZERO,
                    |acc, (&ControlPoint { weight, .. }, bind_pos)| acc + bind_pos * weight,
                )
                / w_sum;

            let p_bar = controllers
                .iter()
                .filter(|c| c.id.as_int() != 0)
                .fold(glam::Vec3::ZERO, |acc, &ControlPoint { id, weight }| {
                    acc + lattice.current_pos(id) * weight
                })
                / w_sum;

            let covariance = controllers
                .iter()
                .zip(controller_binds)
                .filter(|(c, _)| c.id.as_int() != 0)
                .fold(
                    glam::Mat3::ZERO,
                    |acc, (&ControlPoint { id, weight }, &bind_pos)| {
                        let real_pos = lattice.current_pos(id);
                        acc + outer_product(real_pos - p_bar, bind_pos - b_bar) * weight
                    },
                )
                + glam::Mat3::IDENTITY * 0.00001;

            let rotation = decompose_rotation_svd(covariance);
            *deform = rotation * (pose - b_bar) + p_bar;
        }
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

        let vox = fragments.voxels();
        let (mx, my, mz) = fragments.dimensions();
        let total = mx * my * mz + mx + my + mz;

        let mut points = Vec::<DeformPoint>::with_capacity(total as usize);
        let mut near_buf = Vec::<Cell>::with_capacity(CONTROL_POINTS_COUNT);

        for voxel in vox.cells() {
            let point = glam::vec3(
                (voxel.x as f32 / fragments.options().cell_size) as f32,
                (voxel.y as f32 / fragments.options().cell_size) as f32,
                (voxel.z as f32 / fragments.options().cell_size) as f32,
            ) + origin;

            points.push(self.create_deform(point, node_hash, lattice, &mut near_buf));
        }

        let r0 = self.data.len();
        points.drain(..).for_each(|deform| {
            let handle =
                self.data
                    .insert((deform.point, deform.point, deform.controllers, deform.binds));

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
                let ds = point.distance_squared(position);

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
