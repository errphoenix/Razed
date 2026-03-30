use ethel::state::data::{
    Column, DirectIndex, IndirectIndex,
    hash::{Cell, FxSpatialHash},
    table::TableView,
};
use physics::xpbd::NodesRowTableView;

use crate::voxel::VoxelGrid;

pub const CONTROL_POINTS_COUNT: usize = 8;
pub const CONTROL_POINTS_MIN_THRESHOLD: usize = 4;
pub const CONTROL_POINT_CONSTRAIN_THRESHOLD: f32 = 0.2;

ethel::table_spec! {
    struct Deforms {
        deformed: glam::Vec3; // current deformed points
        pose: glam::Vec3; // the base points of the bind pose

        controllers: [ControlPoint; CONTROL_POINTS_COUNT];
        binds: [glam::Vec3; CONTROL_POINTS_COUNT];
    }
}

pub const CONTROL_POINT_MAX_RANGE: u32 = 16;
pub const RIGIDITY: f32 = 4.0;

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

    /// Slice of indirect indices to all points deleted during the last
    /// constrain pass.
    ///
    /// Note: by this point, all the deform points returned by this function
    /// have already been freed from their tables and are no longer accessible.
    ///
    /// This data must only be used as a back-reference to other systems that
    /// rely on indirect indices to track deforms and need to track their
    /// lifetime.
    pub fn deleted_points_frame(&self) -> &[IndirectIndex] {
        &self.deleted_points
    }

    pub fn clear_damage_buffers(&mut self) {
        self.damaged_buffer.clear();
        self.deleted_points.clear();
    }

    pub fn process_damage(&mut self, lattice: &NodesRowTableView) {
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
                        let ds = deformed.distance_squared(*bind) + f32::EPSILON;
                        *weight = 1.0 / ds.powf(RIGIDITY);
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

        // temporarily solve indices
        self.damaged_buffer.iter_mut().for_each(|i| {
            let indirect = self.data.handles()[i.as_index()];
            *i = DirectIndex::from_index(indirect.as_index());
        });

        self.damaged_buffer.drain(..).for_each(|indirect| {
            let ii = IndirectIndex::from_index(indirect.as_index());
            //self.data.free(ii);
            self.deleted_points.push(ii);
        });
    }

    /// Unbind all `broken_nodes` attached to any deforms and flag them for
    /// damage.
    pub fn sync_lattice_damage(&mut self, broken_nodes: &[IndirectIndex]) {
        broken_nodes.iter().for_each(|node| {
            if let Some(deforms) = self.node_map.get_mut(node.as_index()) {
                deforms.iter_mut().for_each(|deform| {
                    if deform.as_int() != 0 {
                        let direct = self.data.solve_indirect(*deform).unwrap();
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
                });
            }
        });
    }

    pub fn constrain_v3(&mut self, lattice: &NodesRowTableView) {
        let mut weights_buf = [0f32; CONTROL_POINTS_COUNT];

        let mut i = 0;
        let (deforms, _, controllers, _) = self.data.split_mut();
        for (deform, controllers) in deforms.join(controllers) {
            for j in 0..CONTROL_POINTS_COUNT {
                let controller_id = controllers[j].id;
                if controller_id.as_int() == 0 {
                    continue;
                }

                let position = lattice.current_pos(controller_id);
                let dist_sq = deform.distance_squared(*position) + f32::EPSILON;
                weights_buf[j] = 1.0 / dist_sq.powf(RIGIDITY);
            }

            let w_t = weights_buf.iter().fold(0f32, |t, v| t + v);
            weights_buf.iter_mut().for_each(|w| *w /= w_t);

            let mut flagged = false;
            controllers.iter_mut().zip(weights_buf).for_each(
                |(ControlPoint { id, weight }, cur_weight)| {
                    let constraint = (cur_weight - *weight).abs();
                    if constraint > CONTROL_POINT_CONSTRAIN_THRESHOLD {
                        // sync reverse-map
                        self.node_map[id.as_index()].retain(|&deform| deform != *id);

                        *id = IndirectIndex::default();
                        if !flagged {
                            flagged = true;
                            self.damaged_buffer.push(DirectIndex::from_index(i));
                        }
                    }
                },
            );

            weights_buf.fill(0.0);

            i += 1;
        }
    }

    pub fn constrain_v2(&mut self, lattice: &NodesRowTableView) {
        self.deleted_points.clear();

        let mut weights_buf = [0f32; CONTROL_POINTS_COUNT];
        let mut flagged = Vec::<(u32, f32)>::new(); // todo: do not alloc

        // invalidate control points constraint weights
        // stores direct indices
        {
            let mut i = 0;
            let (deforms, _, controllers, _) = self.data.split_mut();
            for (deform, controllers) in deforms.join(controllers) {
                for j in 0..CONTROL_POINTS_COUNT {
                    //let controller_id = *&unsafe { controllers.get_unchecked(j) }.id;
                    let controller_id = controllers[j].id;
                    if controller_id.as_int() == 0 {
                        continue;
                    }

                    let position = unsafe { lattice.current_pos_unchecked(controller_id) };
                    let dist_sq = deform.distance_squared(*position) + f32::EPSILON;
                    weights_buf[j] = 1.0 / dist_sq.powf(RIGIDITY);
                }

                let w_t = weights_buf.iter().fold(0f32, |t, v| t + v);
                weights_buf.iter_mut().for_each(|w| *w /= w_t);

                let mut b = false;
                let mut dead_weight = 0.0;

                controllers.iter_mut().zip(weights_buf).for_each(
                    |(ControlPoint { id, weight }, current_weight)| {
                        let constraint = (current_weight - *weight).abs();
                        if constraint * constraint > CONTROL_POINT_CONSTRAIN_THRESHOLD {
                            *id = IndirectIndex::default();
                            dead_weight += *weight;
                            *weight = 0f32;
                            if !b {
                                flagged.push((i, dead_weight));
                            } else {
                                flagged.last_mut().as_mut().unwrap().1 += dead_weight;
                            }
                            b = true;
                        }
                    },
                );

                weights_buf.fill(0f32);
                i += 1;
            }
        }

        {
            let (deformed, pose, controllers, binds) = self.data_mut().split_mut();
            flagged.retain(|&(id, _)| {
                let idx = id as usize;

                // update bind pose to current position
                let deform = deformed.alpha[idx];
                pose.alpha[idx] = deform;

                let controllers = &mut controllers.alpha[idx];
                let binds = &mut binds.alpha[idx];
                controllers.iter_mut().zip(binds.iter_mut()).for_each(
                    |(ControlPoint { id, weight }, bind)| {
                        if id.as_int() != 0 {
                            *bind = *lattice.current_pos(*id);
                            let ds = deform.distance_squared(*bind) + f32::EPSILON;
                            *weight = 1.0 / ds.powf(RIGIDITY);
                        }
                    },
                );

                let w_t = controllers.iter_mut().fold(0f32, |s, ctl| s + ctl.weight);
                controllers
                    .iter_mut()
                    .for_each(|ControlPoint { weight, .. }| *weight /= w_t);

                // if all 0, we keep deforms flagged to remove them in the next pass
                controllers.iter().all(|ctl| ctl.id.as_int() == 0)
            });
        }

        // resolve direct indices to stable indirect indices
        flagged.iter_mut().for_each(|(indirect, _)| {
            let direct = DirectIndex::from_index(*indirect as usize);
            *indirect = self.data.handles()[direct.as_index()].as_int()
        });

        // delete dead deforms
        flagged.drain(..).for_each(|(indirect, _)| {
            if indirect != 0 {
                let ii = IndirectIndex::from_index(indirect as usize);
                self.deleted_points.push(ii);
                self.data.free(ii);
            }
        });
    }

    pub fn constrain(&mut self, lattice: &NodesRowTableView) {
        self.deleted_points.clear();
        let mut weights_buf = [0f32; CONTROL_POINTS_COUNT];
        let mut flagged = Vec::<(u32, f32)>::new(); // todo: do not alloc

        // invalidate control points constraint weights
        // stores direct indices
        {
            let mut i = 0;
            let (deforms, _, controllers, _) = self.data.split_mut();
            for (deform, controllers) in deforms.join(controllers) {
                for j in 0..CONTROL_POINTS_COUNT {
                    //let controller_id = *&unsafe { controllers.get_unchecked(j) }.id;
                    let controller_id = controllers[j].id;
                    if controller_id.as_int() == 0 {
                        continue;
                    }

                    let position = unsafe { lattice.current_pos_unchecked(controller_id) };
                    let dist_sq = deform.distance_squared(*position) + f32::EPSILON;
                    weights_buf[j] = 1.0 / dist_sq.powf(RIGIDITY);
                }

                let w_t = weights_buf.iter().fold(0f32, |t, v| t + v);
                weights_buf.iter_mut().for_each(|w| *w /= w_t);

                let mut b = false;
                let mut dead_weight = 0.0;

                controllers.iter_mut().zip(weights_buf).for_each(
                    |(ControlPoint { id, weight }, current_weight)| {
                        let constraint = (current_weight - *weight).abs();
                        if constraint * constraint > CONTROL_POINT_CONSTRAIN_THRESHOLD {
                            *id = IndirectIndex::default();
                            dead_weight += *weight;
                            *weight = 0f32;
                            if !b {
                                flagged.push((i, dead_weight));
                            } else {
                                flagged.last_mut().as_mut().unwrap().1 += dead_weight;
                            }
                            b = true;
                        }
                    },
                );

                weights_buf.fill(0f32);
                i += 1;
            }
        }

        // resolve to stable indirect indices of flagged deforms
        // flagged
        //     .iter_mut()
        //     .for_each(|direct| *direct = self.data.handles()[*direct as usize]);

        // recompute invalidated control points
        // this retains all deforms for which total weight equals to 0 to be
        // deleted in the next pass.
        {
            let (_, _, controllers, _) = self.data.split_mut();
            flagged.retain(|&(direct, dead_weight)| {
                let direct = DirectIndex::from_index(direct as usize);
                let controllers = &mut controllers.alpha[direct.as_index()];
                controllers.iter_mut().for_each(|controller| {
                    controller.weight += dead_weight * controller.weight;
                });

                // controllers
                //     .iter_mut()
                //     .zip(bind)
                //     .for_each(|(ControlPoint { weight, .. }, bind)| {
                //         let dist_sq = pose.distance_squared(bind) + f32::EPSILON;
                //         *weight = 1.0 / dist_sq.powf(DeformPoint::RIGIDITY);
                //     });

                // let w_t = controllers.iter().fold(0f32, |t, v| t + v.weight);
                // controllers
                //     .iter_mut()
                //     .for_each(|ControlPoint { weight, .. }| *weight /= w_t);

                // if all 0, we keep deforms flagged to remove them in the next pass
                controllers.iter().all(|ctl| ctl.id.as_int() == 0)
            });
        }

        // resolve direct indices to stable indirect indices
        flagged.iter_mut().for_each(|(indirect, _)| {
            let direct = DirectIndex::from_index(*indirect as usize);
            *indirect = self.data.handles()[direct.as_index()].as_int()
        });

        // delete dead deforms
        flagged.drain(..).for_each(|(indirect, _)| {
            if indirect != 0 {
                let ii = IndirectIndex::from_index(indirect as usize);
                self.deleted_points.push(ii);
                self.data.free(ii);
            }
        });
    }

    pub fn deform(&mut self, lattice: &NodesRowTableView) {
        let (deforms, pose, controllers, binds) = self.data.split_mut();
        for (deform, pose, controllers, binds) in deforms.join(pose).join(controllers).join(binds) {
            *deform = glam::Vec3::ZERO;
            controllers.iter().zip(binds.iter()).for_each(
                |(&ControlPoint { id, weight }, &bind)| {
                    if id.as_int() == 0 {
                        return;
                    }

                    // SAFETY:
                    // we assume the indirect index is always valid
                    let position = unsafe { lattice.current_pos_unchecked(id) };
                    let delta = position - bind;

                    *deform += delta * weight;
                },
            );

            *deform += *pose;
        }
    }

    pub fn generate_points(
        &mut self,
        origin: glam::Vec3,
        fragments: &VoxelGrid,
        node_hash: &FxSpatialHash<IndirectIndex>,
        lattice: &NodesRowTableView,
    ) -> std::ops::Range<usize> {
        let lattice_size = lattice.len() + lattice.view_offset();
        self.node_map.resize_with(lattice_size, || Vec::new());

        let vox = fragments.voxels();
        let (mx, my, mz) = fragments.dimensions();
        let total = mx * my * mz + mx + my + mz;

        let mut points = Vec::<DeformPoint>::with_capacity(total as usize);
        let mut near_buf = Vec::<Cell>::with_capacity(CONTROL_POINTS_COUNT);

        for voxel in vox.cells() {
            let point = glam::vec3(
                (voxel.x / fragments.options().density) as f32,
                (voxel.y / fragments.options().density) as f32,
                (voxel.z / fragments.options().density) as f32,
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
        let max_range = CONTROL_POINT_MAX_RANGE * node_hash.resolution.get();
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
                let node = *node_hash.get(&cell).expect("query is of populated node");

                // SAFETY:
                // we assume node_hash has been loaded with the nodes of
                // lattice, thus all handles are valid.
                let position = *unsafe { lattice.current_pos_unchecked(node) };
                let dist = point.distance_squared(position) + f32::EPSILON;

                binds[i] = position;
                controllers[i] = ControlPoint {
                    id: node,
                    weight: 1.0 / dist.powf(RIGIDITY),
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
