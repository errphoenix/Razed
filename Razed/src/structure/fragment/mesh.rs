#![allow(unused, reason = "wip feature")]

use ethel::state::data::hash::{Cell, FxSpatialHash};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct CellularForm {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}

impl CellularForm {
    pub const fn new(width: u32, height: u32, depth: u32) -> Self {
        Self {
            width,
            height,
            depth,
        }
    }

    pub const fn size(&self) -> u32 {
        self.width * self.height * self.depth
    }

    pub const fn depth_slice_size(&self) -> u32 {
        self.width * self.width * self.height
    }

    pub const fn is_null(&self) -> bool {
        self.width == 0 || self.height == 0 || self.depth == 0
    }
}

#[derive(Clone, Debug, Default)]
pub struct CellularMeshSystem {
    groups: rustc_hash::FxHashMap<CellularForm, MeshCellMap>,
    largest: CellularForm,
}

impl CellularMeshSystem {
    pub fn new() -> Self {
        Self {
            groups: rustc_hash::FxHashMap::default(),
            largest: CellularForm::default(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            groups: rustc_hash::FxHashMap::with_capacity_and_hasher(capacity, Default::default()),
            largest: CellularForm::default(),
        }
    }

    pub fn get(&self, form: &CellularForm) -> Option<&MeshCellMap> {
        self.groups.get(form)
    }

    pub fn set(&mut self, form: CellularForm, map: MeshCellMap) {
        self.groups.insert(form, map);
        self.largest = self.largest.max(form);
    }

    pub fn largest(&self) -> CellularForm {
        self.largest
    }

    pub fn fill_cells(&self, cells: &mut FxSpatialHash<ethel::mesh::Id>) {
        let max_x = self.largest.width;
        let max_y = self.largest.height;
        let max_z = self.largest.depth;

        let min_cell = cells.min();
        let Cell {
            x: e_x,
            y: e_y,
            z: e_z,
        } = cells.axis_extents();

        let mut cluster_origin_cell = min_cell;
        for x in 0..e_x {
            for y in 0..e_y {
                for z in 0..e_z {
                    let cell = cluster_origin_cell + Cell { x, y, z };
                }
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct MeshCellMap {
    x: u32,
    y: u32,
    z: u32,

    meshes: Vec<ethel::mesh::Id>,
}

impl MeshCellMap {
    /// Create a new fragment mesh-cell map from a 3D collection of meshes.
    ///
    /// This is meant to be used in combination with
    /// [`procedural voronoi fracture meshes`](CubeVoronoiGenerator).
    ///
    /// The order of the `meshes` is very important. They must be populated,
    /// by each `x` coordinate, which for each must populate each `y`
    /// coordinate, which for each must populate each `z` coordinate.
    /// For example, if these were to be populated by a triple-nested for loop:
    /// ```rust,ignore
    /// for x in 0..3 {
    ///     for y in 0..3 {
    ///         for z in 0..3 {
    ///             // populate mesh corresponding to cell (x,y,z)
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// # Panics
    /// The function will panic if the length of `meshes` does not match the
    /// total count of meshes `x * y * z`.
    pub fn new(x: u32, y: u32, z: u32, meshes: &[ethel::mesh::Id]) -> Self {
        assert_eq!(meshes.len(), (x * y * z) as usize);

        let meshes = {
            let mut v_meshes = Vec::with_capacity(meshes.len());
            v_meshes.extend_from_slice(meshes);
            v_meshes
        };

        Self { x, y, z, meshes }
    }

    pub fn new_empty(x: u32, y: u32, z: u32) -> Self {
        Self {
            x,
            y,
            z,
            meshes: vec![ethel::mesh::Id::default(); (x * y * z) as usize],
        }
    }

    pub const fn mesh_index(&self, cell: Cell) -> usize {
        let cell = cell.abs();
        let x = cell.x as u32 % self.x;
        let y = cell.y as u32 % self.y;
        let z = cell.z as u32 % self.z;
        (x * x * y + z) as usize
    }

    pub fn set_mesh(&mut self, cell: Cell, mesh: ethel::mesh::Id) {
        let index = self.mesh_index(cell);
        self.meshes[index] = mesh;
    }

    pub fn mesh_at(&self, cell: Cell) -> Option<ethel::mesh::Id> {
        let index = self.mesh_index(cell);
        self.meshes.get(index).copied()
    }

    pub fn meshes(&self) -> &[ethel::mesh::Id] {
        &self.meshes
    }
}
