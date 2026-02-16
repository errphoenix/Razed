#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FragmentState {
    /// The fragment is attached to the lattice structure.
    ///
    /// The behaviour of the fragment is entirely driven by the lattice nodes
    /// it is attached to.
    #[default]
    Attached,

    /// The fragment is an independent physical body.
    ///
    /// It is not attached to any structure and it is most likely in movement
    /// heading towards the ground.
    Debris,

    /// Static/inactive
    ///
    /// The then fragment, and now debris has been on the ground for a
    /// prolonged period of time.
    ///
    /// It is likely scheduled for removal.
    InactiveDebris,
}

ethel::table_spec! {
    struct Fragments {
        parents: [u32; 4];
        influence: [f32; 4];

        state: FragmentState;
        health: f32; // also acts as mass in Debris state

        position: glam::Vec3;
        velocity: glam::Vec3;
        forces: glam::Vec3;
    }
}

#[derive(Debug, Default)]
pub struct FragmentSystem {
    fragments: FragmentsRowTable,
}

impl FragmentSystem {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VoxelCell {
    x: i32,
    y: i32,
    z: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct VoxelGridOptions {
    width: f32,
    height: f32,
    depth: f32,
    density: i32,
}

impl VoxelGridOptions {
    pub fn new(width: f32, height: f32, depth: f32, density: i32) -> Self {
        Self {
            width,
            height,
            depth,
            density,
        }
    }

    pub fn with_width(self, width: f32) -> Self {
        Self {
            width,
            height: self.height,
            depth: self.depth,
            density: self.density,
        }
    }

    pub fn with_height(self, height: f32) -> Self {
        Self {
            height,
            width: self.width,
            depth: self.depth,
            density: self.density,
        }
    }

    pub fn with_depth(self, depth: f32) -> Self {
        Self {
            depth,
            width: self.width,
            height: self.height,
            density: self.density,
        }
    }

    pub fn with_density(self, density: i32) -> Self {
        Self {
            density,
            width: self.width,
            height: self.height,
            depth: self.depth,
        }
    }
}

pub type VoxelGridFn = fn(VoxelCell) -> bool;
pub type VoxelOffsetFn = fn(VoxelCell) -> glam::Vec3;

#[derive(Clone, Debug)]
pub struct VoxelGrid {
    generator: VoxelGridFn,
    offset_fn: VoxelOffsetFn,
    options: VoxelGridOptions,

    voxels: std::collections::HashMap<VoxelCell, glam::Vec3>,
}

impl Default for VoxelGrid {
    fn default() -> Self {
        Self {
            generator: |_| true,
            offset_fn: |_| glam::Vec3::ZERO,
            options: VoxelGridOptions::default(),
            voxels: Default::default(),
        }
    }
}

impl VoxelGrid {
    pub fn new(generator: VoxelGridFn, options: VoxelGridOptions) -> Self {
        Self {
            generator,
            options,
            ..Default::default()
        }
    }

    pub fn with_offsets(
        generator: VoxelGridFn,
        options: VoxelGridOptions,
        offset_fn: VoxelOffsetFn,
    ) -> Self {
        Self {
            generator,
            options,
            offset_fn,
            ..Default::default()
        }
    }

    pub fn build(&mut self, center: glam::Vec3) {
        self.voxels.clear();

        let vw = (self.options.density as f32 * self.options.width) as i32;
        let vh = (self.options.density as f32 * self.options.height) as i32;
        let vd = (self.options.density as f32 * self.options.depth) as i32;

        let hvw = vw / 2;
        let hvh = vh / 2;
        let hvd = vd / 2;

        for x in -hvw..hvw {
            for y in -hvh..hvh {
                for z in -hvd..hvd {
                    let cell = VoxelCell { x, y, z };
                    if (self.generator)(cell) {
                        let position = glam::vec3(
                            (cell.x as f32 / vw as f32) * self.options.width,
                            (cell.y as f32 / vh as f32) * self.options.height,
                            (cell.z as f32 / vd as f32) * self.options.depth,
                        );
                        let offset = (self.offset_fn)(cell);
                        self.voxels.insert(cell, center + position + offset);
                    }
                }
            }
        }
    }

    pub fn get_voxel(&self, cell: VoxelCell) -> Option<glam::Vec3> {
        self.voxels.get(&cell).copied()
    }

    pub fn options(&self) -> &VoxelGridOptions {
        &self.options
    }

    pub fn options_mut(&mut self) -> &mut VoxelGridOptions {
        &mut self.options
    }

    pub fn voxels(&self) -> &std::collections::HashMap<VoxelCell, glam::Vec3> {
        &self.voxels
    }
}
