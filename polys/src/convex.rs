use crate::{Face, Facen, Plane, QuadFace, TriFace};

#[derive(Clone, Debug, Default)]
pub struct Convex<F: Face> {
    vertices: Vec<glam::Vec3>,
    faces: Vec<Facen<F>>,
}

impl<F: Face> Convex<F> {
    pub const fn new(vertices: Vec<glam::Vec3>, faces: Vec<Facen<F>>) -> Self {
        Self { vertices, faces }
    }
}

impl Convex<QuadFace> {
    pub fn unit_cube() -> Self {
        let vertices = vec![
            glam::vec3(0.0, 0.0, 0.0),
            glam::vec3(1.0, 0.0, 0.0),
            glam::vec3(1.0, 1.0, 0.0),
            glam::vec3(0.0, 1.0, 0.0),
            glam::vec3(0.0, 0.0, 1.0),
            glam::vec3(1.0, 0.0, 1.0),
            glam::vec3(1.0, 1.0, 1.0),
            glam::vec3(0.0, 1.0, 1.0),
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
}

impl Convex<Vec<u32>> {
    pub fn clip_plane(&self, plane: Plane) -> Convex<Vec<u32>> {
        let mut vertices = Vec::new();
        let mut clipped = Vec::new();
        let mut faces = Vec::new();

        let mut t_indices = Vec::new();
        for Facen {
            indexed: indices,
            normal,
        } in &self.faces
        {
            const FACE_SIZE: usize = 4;
            for i in 0..FACE_SIZE {
                let a = self.vertices[indices[i] as usize];
                let b = self.vertices[indices[i] as usize + 1 % FACE_SIZE];
                let da = normal.dot(a) - plane.d;
                let db = normal.dot(b) - plane.d;

                if da >= 0.0 {
                    let i = vertices.len();
                    vertices.push(a);
                    t_indices.push(i as u32);
                }
                if (da >= 0.0) != (db >= 0.0) {
                    let t = da / (da - db);
                    let inters = a + t * (b - a);

                    let i = vertices.len();
                    vertices.push(inters);

                    t_indices.push(i as u32);
                    clipped.push(i as u32);
                }
            }

            if t_indices.len() >= 3 {
                let face_indices = t_indices.drain(..).collect::<Vec<u32>>();
                faces.push(Facen::<Vec<u32>>::new(face_indices, *normal));
            }
        }

        if clipped.len() >= 3 {
            let centroid = clipped
                .iter()
                .map(|index| vertices[*index as usize])
                .fold(glam::Vec3::ZERO, |a, b| a + b)
                / clipped.len() as f32;

            let n_0 = (vertices[clipped[0] as usize] - centroid).normalize();
            clipped.sort_by(|i, j| {
                let vi = (vertices[*i as usize] - centroid).normalize();
                let vj = (vertices[*j as usize] - centroid).normalize();
                let ai = n_0.dot(vi).acos() * plane.normal.cross(n_0).dot(vi).signum();
                let bj = n_0.dot(vj).acos() * plane.normal.cross(n_0).dot(vj).signum();
                ai.partial_cmp(&bj).unwrap()
            });

            faces.push(Facen::new(clipped, -plane.normal));
        }

        Convex { vertices, faces }
    }

    pub fn triangulate(&self) -> Convex<TriFace> {
        let mut faces = Vec::new();

        for Facen {
            indexed: indices,
            normal,
        } in &self.faces
        {
            if indices.len() < 3 {
                continue;
            }

            let base = indices[0];
            let mut i = 0;
            for j in 1..(indices.len() - 1) {
                let i1 = indices[i + j];
                let i2 = indices[i + j + 1];
                faces.push(Facen::new([base, i1, i2], *normal));
                i += 1;
            }
        }

        Convex {
            vertices: self.vertices.clone(),
            faces,
        }
    }
}
