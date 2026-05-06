use crate::{Face, Facen, QuadFace, TriFace};

#[derive(Clone, Debug, Default)]
pub struct Convex<F: Face> {
    vertices: Vec<glam::Vec3>,
    faces: Vec<Facen<F>>,
}

impl Convex<QuadFace> {
    pub fn cube(extents: f32) -> Self {
        Self::parallelepiped(glam::Vec3::splat(extents))
    }

    pub fn parallelepiped(extents: glam::Vec3) -> Self {
        let vertices = vec![
            glam::vec3(-extents.x, -extents.y, -extents.z),
            glam::vec3(extents.x, -extents.y, -extents.z),
            glam::vec3(extents.x, extents.y, -extents.z),
            glam::vec3(-extents.x, extents.y, -extents.z),
            glam::vec3(-extents.x, -extents.y, extents.z),
            glam::vec3(extents.x, -extents.y, extents.z),
            glam::vec3(extents.x, extents.y, extents.z),
            glam::vec3(-extents.x, extents.y, extents.z),
        ];
        let faces = vec![
            Facen::new([0, 1, 2, 3], glam::vec3(0.0, 0.0, -1.0)),
            Facen::new([4, 5, 6, 7], glam::vec3(0.0, 0.0, 1.0)),
            Facen::new([0, 1, 5, 4], glam::vec3(0.0, -1.0, 0.0)),
            Facen::new([2, 3, 7, 6], glam::vec3(0.0, 1.0, 0.0)),
            Facen::new([0, 3, 7, 4], glam::vec3(-1.0, 0.0, 0.0)),
            Facen::new([1, 2, 6, 5], glam::vec3(1.0, 0.0, 0.0)),
        ];
        Self { vertices, faces }
    }

    pub fn triangulate(&self) -> Convex<TriFace> {
        let mut faces = Vec::with_capacity(self.faces.len() * 2);
        self.faces.iter().for_each(|face| {
            let (a, b) = face.indexed.triangulate();
            faces.push(Facen::new(a, face.normal));
            faces.push(Facen::new(b, face.normal));
        });

        Convex {
            vertices: self.vertices.clone(),
            faces,
        }
    }
}

impl Convex<Vec<u32>> {
    pub fn cube(extents: f32) -> Self {
        Self::parallelepiped(glam::Vec3::splat(extents))
    }

    pub fn parallelepiped(extents: glam::Vec3) -> Self {
        let vertices = vec![
            glam::vec3(-extents.x, -extents.y, -extents.z),
            glam::vec3(extents.x, -extents.y, -extents.z),
            glam::vec3(extents.x, extents.y, -extents.z),
            glam::vec3(-extents.x, extents.y, -extents.z),
            glam::vec3(-extents.x, -extents.y, extents.z),
            glam::vec3(extents.x, -extents.y, extents.z),
            glam::vec3(extents.x, extents.y, extents.z),
            glam::vec3(-extents.x, extents.y, extents.z),
        ];
        let faces = vec![
            Facen::new([0, 1, 2, 3], glam::vec3(0.0, 0.0, -1.0)),
            Facen::new([4, 5, 6, 7], glam::vec3(0.0, 0.0, 1.0)),
            Facen::new([0, 1, 5, 4], glam::vec3(0.0, -1.0, 0.0)),
            Facen::new([2, 3, 7, 6], glam::vec3(0.0, 1.0, 0.0)),
            Facen::new([0, 3, 7, 4], glam::vec3(-1.0, 0.0, 0.0)),
            Facen::new([1, 2, 6, 5], glam::vec3(1.0, 0.0, 0.0)),
        ];
        Self { vertices, faces }
    }

    pub fn triangulate(&self) -> Convex<TriFace> {
        let mut vertices = self.vertices.clone();
        let mut faces = Vec::new();

        for Facen {
            indexed: indices,
            normal,
        } in &self.faces
        {
            // invalid face?
            if indices.len() < 3 {
                continue;
            }

            // leave as-is
            if indices.len() == 3 {
                let indices = indices.as_array().copied().unwrap();
                faces.push(Facen::new(TriFace::from(indices), *normal));
                continue;
            }

            // create triangle fan; this assumes the polygon is coplanar
            let center_point = indices
                .iter()
                .map(|&i| vertices[i as usize])
                .sum::<glam::Vec3>()
                / indices.len() as f32;
            let ci = vertices.len() as u32;
            vertices.push(center_point);

            for i in 0..indices.len() {
                let ni = (i + 1) % indices.len();

                let vi = indices[i];
                let vni = indices[ni];

                let fi = [ci, vi, vni];

                faces.push(Facen::new(TriFace::from(fi), *normal));
            }
        }

        Convex { vertices, faces }
    }
}

impl<F: Face> Convex<F> {
    pub const fn new(vertices: Vec<glam::Vec3>, faces: Vec<Facen<F>>) -> Self {
        Self { vertices, faces }
    }

    pub fn into_alloc(mut self) -> Convex<Vec<u32>> {
        Convex {
            vertices: self.vertices.clone(),
            faces: self
                .faces
                .drain(..)
                .map(|f| f.into_alloc_n())
                .collect::<Vec<_>>(),
        }
    }

    pub fn vertices(&self) -> &[glam::Vec3] {
        &self.vertices
    }

    pub fn faces(&self) -> &[Facen<F>] {
        &self.faces
    }

    pub fn vertices_mut(&mut self) -> &mut Vec<glam::Vec3> {
        &mut self.vertices
    }

    pub fn faces_mut(&mut self) -> &mut Vec<Facen<F>> {
        &mut self.faces
    }

    pub fn split_mut(&mut self) -> (&mut Vec<glam::Vec3>, &mut Vec<Facen<F>>) {
        (&mut self.vertices, &mut self.faces)
    }

    pub fn centroid(&self) -> glam::Vec3 {
        let v_tot = self.vertices.iter().fold(glam::Vec3::ZERO, |t, v| t + *v);
        v_tot / self.vertices.len() as f32
    }

    pub fn make_local(&mut self) {
        let centroid = self.centroid();
        self.vertices.iter_mut().for_each(|v| *v -= centroid);
    }

    pub fn translate(&mut self, translation: glam::Vec3) {
        self.vertices.iter_mut().for_each(|v| *v += translation);
    }

    pub fn scale(&mut self, scaling: glam::Vec3) {
        self.vertices.iter_mut().for_each(|v| *v *= scaling);
    }

    /// Compute the mesh's face normals in a preallocated slice of memory.
    ///
    /// Each normal corresponds to a single face, therefore the length of
    /// `normals` must match the number of faces.
    pub fn compute_normals(&self, normals: &mut [glam::Vec3]) {
        crate::compute_normals(&self.faces, normals, &self.vertices);
    }

    /// Compute the mesh's face normals allocating new memory.
    pub fn compute_normals_alloc(&self) -> Vec<glam::Vec3> {
        let mut normals = vec![glam::Vec3::ZERO; self.faces.len()];
        self.compute_normals(&mut normals);
        normals
    }
}
