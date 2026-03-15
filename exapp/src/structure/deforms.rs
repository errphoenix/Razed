use std::collections::HashSet;

use ethel::state::data::{
    Column,
    hash::{Cell, FxSpatialHash},
};

use crate::structure::{LatticeView, fragment::VoxelGrid};

pub const CONTROL_POINTS_COUNT: usize = 8;

ethel::table_spec! {
    struct Deforms {
        deformed: glam::Vec3; // current deformed points
        pose: glam::Vec3; // the base points of the bind pose

        controllers: [ControlPoint; CONTROL_POINTS_COUNT];
        binds: [glam::Vec3; CONTROL_POINTS_COUNT];
    }
}

#[derive(Debug, Default)]
pub struct DeformSystem {
    data: DeformsRowTable,
}

impl DeformSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: DeformsRowTable::with_capacity(capacity),
        }
    }

    pub fn data(&self) -> &DeformsRowTable {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut DeformsRowTable {
        &mut self.data
    }

    pub fn deform(&mut self, lattice: &LatticeView) {
        let (deforms, pose, controllers, binds) = self.data.split_mut();
        for (deform, pose, controllers, binds) in deforms.join(pose).join(controllers).join(binds) {
            *deform = glam::Vec3::ZERO;
            controllers.iter().zip(binds.iter()).for_each(
                |(&ControlPoint { id, weight }, &bind)| {
                    if id == 0 {
                        return;
                    }

                    // SAFETY:
                    // we assume the indirect index is always valid
                    let position = unsafe { lattice.position_unchecked(id) };
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
        node_hash: &FxSpatialHash<u32>,
        lattice: &LatticeView,
    ) -> std::ops::Range<usize> {
        let vox = fragments.voxels();
        let (mx, my, mz) = fragments.dimensions();

        let total = mx * my * mz + mx + my + mz;
        let mut unique_cells = HashSet::<Cell>::with_capacity(total as usize);
        for &voxel in vox.cells() {
            for x in 0..1 {
                for y in 0..1 {
                    for z in 0..1 {
                        let cell = Cell {
                            x: voxel.x + x,
                            y: voxel.y + y,
                            z: voxel.z + z,
                        };
                        unique_cells.insert(cell);
                    }
                }
            }
        }

        let mut points = Vec::<DeformPoint>::with_capacity(total as usize);
        let mut near_buf = Vec::<Cell>::with_capacity(CONTROL_POINTS_COUNT);
        let hu = fragments.options().density as f32 * 0.5;

        for cell in unique_cells.iter() {
            let point = glam::vec3(
                (cell.x / fragments.options().density) as f32 + hu,
                (cell.y / fragments.options().density) as f32 + hu,
                (cell.z / fragments.options().density) as f32 + hu,
            ) + origin;

            points.push(DeformPoint::new(point, node_hash, lattice, &mut near_buf));
        }

        let r0 = self.data.len();
        points.drain(..).for_each(|deform| {
            self.data
                .put((deform.point, deform.point, deform.controllers, deform.binds));
        });
        let r1 = self.data.len();
        println!("done; {} deform points", r1 - r0);

        r0..r1
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ControlPoint {
    pub id: u32,
    pub weight: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct DeformPoint {
    point: glam::Vec3,
    controllers: [ControlPoint; CONTROL_POINTS_COUNT],
    binds: [glam::Vec3; CONTROL_POINTS_COUNT],
}

impl DeformPoint {
    pub const CONTROL_POINT_MAX_RANGE: u32 = 16;
    pub const RIGIDITY: f32 = 4.0;

    fn new(
        point: glam::Vec3,
        node_hash: &FxSpatialHash<u32>,
        lattice: &LatticeView,
        near_buf: &mut Vec<Cell>,
    ) -> Self {
        near_buf.clear();
        let max_range = Self::CONTROL_POINT_MAX_RANGE * node_hash.resolution.get();
        let _ = node_hash.nearest_cells(
            node_hash.cell_at(point),
            CONTROL_POINTS_COUNT as u32,
            max_range,
            near_buf,
            false,
        );

        let mut c = 0;
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
                let position = unsafe { lattice.position_unchecked(node) };
                let dist = point.distance(position) + f32::EPSILON;

                binds[c] = position;
                controllers[c] = ControlPoint {
                    id: node,
                    weight: 1.0 / dist.powf(Self::RIGIDITY),
                };

                c = i;
            });

        let w_t = controllers
            .iter()
            .fold(0f32, |w_t, ControlPoint { weight: w, .. }| w_t + *w);
        controllers
            .iter_mut()
            .for_each(|ControlPoint { weight, .. }| {
                *weight /= w_t;
            });

        Self {
            point,
            controllers,
            binds,
        }
    }
}
