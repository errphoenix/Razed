use ethel::state::data::hash::Cell;

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

    pub fn mesh_index(&self, cell: Cell) -> usize {
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
