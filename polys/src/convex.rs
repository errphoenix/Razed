use crate::{Face, Facen, Plane, QuadFace, TriFace};

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

    pub fn clip_plane(&self, plane: Plane) -> Option<Convex<Vec<u32>>> {
        let mut new_vertices = Vec::new();
        let mut new_faces = Vec::new();
        let mut cap_indices = Vec::new();

        let signs: Vec<f32> = self
            .vertices
            .iter()
            .map(|&v| plane.normal.dot(v) - plane.d)
            .collect();

        if signs.iter().all(|&s| s < -1e-6) {
            return None;
        }

        let mut find_or_insert = |v: glam::Vec3| -> u32 {
            for (i, &existing) in new_vertices.iter().enumerate() {
                let dev: glam::Vec3 = existing - v;
                if dev.length_squared() < 1e-8 {
                    return i as u32;
                }
            }
            new_vertices.push(v);
            (new_vertices.len() - 1) as u32
        };

        for face in &self.faces {
            let mut out_indices = Vec::new();
            let n = face.indexed.len();

            for i in 0..n {
                let ia = face.indexed[i] as usize;
                let ib = face.indexed[(i + 1) % n] as usize;

                let va = self.vertices[ia];
                let vb = self.vertices[ib];
                let da = signs[ia];
                let db = signs[ib];

                if da >= -1e-6 {
                    out_indices.push(find_or_insert(va));
                }

                if (da > 1e-6 && db < -1e-6) || (da < -1e-6 && db > 1e-6) {
                    let t = da / (da - db);
                    let p = va + t * (vb - va);
                    let idx = find_or_insert(p);
                    out_indices.push(idx);
                    if !cap_indices.contains(&idx) {
                        cap_indices.push(idx);
                    }
                }
            }

            if out_indices.len() >= 3 {
                new_faces.push(Facen::new(out_indices, face.normal));
            }
        }

        if cap_indices.len() >= 3 {
            let centroid = cap_indices
                .iter()
                .map(|&i| new_vertices[i as usize])
                .sum::<glam::Vec3>()
                / cap_indices.len() as f32;

            let up = if plane.normal.abs().dot(glam::Vec3::Y) < 0.9 {
                glam::Vec3::Y
            } else {
                glam::Vec3::X
            };
            let tangent = plane.normal.cross(up).normalize();
            let bitangent = plane.normal.cross(tangent).normalize();

            cap_indices.sort_by(|&a, &b| {
                let va = new_vertices[a as usize] - centroid;
                let vb = new_vertices[b as usize] - centroid;
                let ang_a = va.dot(tangent).atan2(va.dot(bitangent));
                let ang_b = vb.dot(tangent).atan2(vb.dot(bitangent));
                ang_a.partial_cmp(&ang_b).unwrap()
            });

            new_faces.push(Facen::new(cap_indices, -plane.normal));
        }

        if new_vertices.is_empty() {
            None
        } else {
            Some(Convex {
                vertices: new_vertices,
                faces: new_faces,
            })
        }
    }
}
