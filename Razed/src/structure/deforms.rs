use ethel::state::data::{
    Column, DirectIndex, IndirectIndex,
    hash::{Cell, FxSpatialHash},
    table::TableView,
};
use physics::xpbd::NodesRowTableView;

use crate::voxel::VoxelGrid;

pub const CONTROL_POINTS_COUNT: usize = 8;
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
pub const RIGIDITY: f32 = 1.0;

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

            let di = self.data.solve_indirect(ii).unwrap();
            self.data.pose[di.as_index()] = glam::Vec3::ZERO;
            self.deleted_points.push(ii);
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
