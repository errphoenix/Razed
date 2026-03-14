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
        for (deforms, pose, controllers, binds) in deforms.join(pose).join(controllers).join(binds)
        {
            *deforms = glam::Vec3::ZERO;
            controllers.iter().zip(binds.iter()).for_each(
                |(&ControlPoint { id, weight }, &bind)| {
                    if id == 0 {
                        return;
                    }

                    // SAFETY:
                    // we assume the indirect index is always valid
                    let position = unsafe { lattice.position_unchecked(id) };
                    let delta = position - bind;

                    *deforms += delta * weight;
                },
            );

            *deforms += *pose;
        }
    }

    pub fn generate_points(
        &mut self,
        fragments: &VoxelGrid,
        node_hash: &FxSpatialHash<u32>,
        lattice: &LatticeView,
    ) -> std::ops::Range<usize> {
        let i = fragments.options().density;
        // half-unit for center-to-corner offset
        let hu = 0.5 / i as f32;

        let vox = fragments.voxels();
        let (mx, my, mz) = fragments.cell_bounds();

        let total = mx * my * mz + mx * my + mz;
        let mut points = Vec::<DeformPoint>::with_capacity(total as usize);
        let mut near_buf = Vec::<Cell>::with_capacity(CONTROL_POINTS_COUNT);

        for &voxel in vox.elements() {
            let center = fragments.point_from_id(voxel);
            let corner = center - hu;

            let mut other = center;
            let cell = fragments.cell_from_id(voxel);
            if cell.x >= mx / 2 {
                other += hu * glam::Vec3::X;
            }
            if cell.y >= mx / 2 {
                other += hu * glam::Vec3::Y;
            }
            if cell.z >= mx / 2 {
                other += hu * glam::Vec3::Z;
            }

            points.push(DeformPoint::new(corner, node_hash, lattice, &mut near_buf));
            if other != center {
                points.push(DeformPoint::new(other, node_hash, lattice, &mut near_buf));
            }
        }

        let r0 = self.data.len();
        points.drain(..).for_each(|deform| {
            self.data
                .put((deform.point, deform.point, deform.controllers, deform.binds));
        });
        let r1 = self.data.len();

        r0..r1
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ControlPoint {
    pub id: u32,
    pub weight: f32,
}

#[derive(Debug, Clone, Copy)]
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
                let dist_sq = point.distance_squared(position) + f32::EPSILON;

                binds[c] = position;
                controllers[c] = ControlPoint {
                    id: node,
                    weight: 1.0 / dist_sq.powf(Self::RIGIDITY),
                };

                c = i;
            });

        Self {
            point,
            controllers,
            binds,
        }
    }
}
