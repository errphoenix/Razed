use std::collections::{HashMap, HashSet};

use crate::{Face, Facen, convex::Convex};

#[derive(Clone, Debug, Default)]
pub struct MappedMesh {
    pub vertices: Vec<MappedVertex>,
    pub edges: Vec<MappedEdge>,
    pub faces: Vec<MappedFace>,
}
impl MappedMesh {
    pub fn new<F: Face>(convex: Convex<F>) -> Self {
        let vertices = convex
            .vertices()
            .iter()
            .map(|&p| MappedVertex::new(p))
            .collect::<Vec<_>>();

        let mut edges = Vec::with_capacity(vertices.len() / 2);
        let mut faces = Vec::with_capacity(convex.faces().len());

        let mut existing_edges = HashMap::with_capacity(edges.capacity());

        // face index to edge index mapping
        let mut ef_map = HashMap::with_capacity(faces.capacity());

        for (i, face) in convex.faces().iter().enumerate() {
            for j in 0..face.len() {
                let v0 = face[j];
                let v1 = face[(j + 1) % face.len()];

                let entry = ef_map.entry(i).or_insert_with(|| Vec::new());

                if !existing_edges.contains_key(&(v0, v1))
                    && !existing_edges.contains_key(&(v1, v0))
                {
                    let ei = edges.len();
                    existing_edges.insert((v0, v1), ei as u32);
                    edges.push(MappedEdge {
                        vertices: [v0, v1],
                        ..Default::default()
                    });
                    entry.push(ei as u32);
                } else {
                    if let Some(ei) = existing_edges
                        .get(&(v0, v1))
                        .or_else(|| existing_edges.get(&(v1, v0)))
                    {
                        entry.push(*ei);
                    }
                }
            }

            faces.push(MappedFace {
                normal: face.normal,
                ..Default::default()
            });
        }

        ef_map.drain().for_each(|(f_i, f_edges)| {
            let face = &mut faces[f_i];
            for ei in f_edges {
                face.edges.insert(ei);
                edges[ei as usize].faces.insert(f_i as u32);
            }
        });

        Self {
            vertices,
            edges,
            faces,
        }
    }

    pub fn unmap(self) -> Convex<Vec<u32>> {
        let mut faces = self.ordered_faces();
        if faces.is_empty() {
            return Convex::new(Vec::new(), Vec::new());
        }

        // prune invisible vertices and remap indices
        let mut points = Vec::with_capacity(self.vertices.len());
        let mut vmap = vec![-1i32; self.vertices.len()];
        for (i, cv) in self.vertices.iter().enumerate() {
            if cv.visible {
                vmap[i] = points.len() as i32;
                points.push(cv.point);
            }
        }
        for f in &mut faces {
            for i in &mut f.1 {
                *i = vmap[*i as usize] as u32;
            }
        }

        let mesh_faces = faces
            .drain(..)
            .map(|(id, indices)| {
                let normal = self.faces[id as usize].normal;
                Facen::<Vec<_>>::new(indices, normal)
            })
            .collect::<Vec<_>>();

        Convex::new(points, mesh_faces)
    }

    pub fn preserve_hard_edges(&mut self, cosine_threshold: f32) {
        // original edge, face with new edge
        let mut dupe = Vec::<(u32, u32)>::new();
        for (i, edge) in self.edges.iter().enumerate() {
            if !edge.visible {
                continue;
            }
            // any edges more complex are ignored
            if edge.faces.len() == 2 {
                let (f0i, f1i) = {
                    let mut fi = edge.faces.iter();
                    let f0 = *fi.next().unwrap();
                    let f1 = *fi.next().unwrap();
                    (f0, f1)
                };
                let f0 = &self.faces[f0i as usize];
                let f1 = &self.faces[f1i as usize];
                if f0.visible && f1.visible {
                    let n0 = f0.normal;
                    let n1 = f1.normal;
                    if n0.dot(n1) < cosine_threshold {
                        // face 1 gets moved to new edge
                        dupe.push((i as u32, f1i));
                    }
                }
            }
        }

        // edge id, valid v0,v1
        let mut u_edges = Vec::<(u32, [u32; 2])>::new();

        for (edge, face) in dupe {
            let src_e = &mut self.edges[edge as usize];
            src_e.faces.remove(&face);

            let o_v0 = src_e.vertices[0];
            let o_v1 = src_e.vertices[1];
            let p0 = self.vertices[o_v0 as usize].point;
            let p1 = self.vertices[o_v1 as usize].point;
            let v0 = self.vertices.len() as u32;
            self.vertices.push(MappedVertex::new(p0));
            self.vertices.push(MappedVertex::new(p1));

            let mut e_faces = HashSet::new();
            e_faces.insert(face);

            let new_edge = self.edges.len() as u32;
            self.edges.push(MappedEdge {
                vertices: [v0, v0 + 1],
                faces: e_faces,
                visible: true,
            });

            let face = &mut self.faces[face as usize];
            face.edges.remove(&edge);
            face.edges.insert(new_edge);

            for &u_ei in &face.edges {
                let u_e = &self.edges[u_ei as usize];
                let mut u_v = u_e.vertices;
                for u_v in &mut u_v {
                    if *u_v == o_v0 {
                        *u_v = v0;
                    } else if *u_v == o_v1 {
                        *u_v = v0 + 1;
                    }
                }
                u_edges.push((u_ei, u_v));
            }
            for &(e, v) in &u_edges {
                self.edges[e as usize].vertices = v;
            }

            u_edges.clear();
        }
    }

    /// Compute the normals of all faces (from ordered vertices).
    ///
    /// This function does not expect pre-ordered vertices, only a buffer to do
    /// so it self through [`MappedMesh::ordered_face_vertices`] to avoid
    /// allocating new memory.
    pub fn compute_face_normals(&mut self, ordered_vertices_buffer: &mut [u32]) {
        for i in 0..self.faces.len() {
            self.ordered_face_vertices(&self.faces[i], ordered_vertices_buffer);
            let n = compute_normal(ordered_vertices_buffer, &self.vertices);
            self.faces[i].normal = n;
        }
    }

    /// Compute the normals of all faces (from ordered vertices) in newly
    /// allocated memory.
    pub fn compute_face_normals_alloc(&mut self) {
        let mut ordered_vertices = Vec::new();
        for i in 0..self.faces.len() {
            ordered_vertices.resize(self.faces[i].edges.len() + 1, 0u32);
            self.ordered_face_vertices(&self.faces[i], &mut ordered_vertices);
            let n = compute_normal(&ordered_vertices, &self.vertices);
            self.faces[i].normal = n;
        }
    }

    /// Compute the normals of a `face` (from ordered vertices).
    ///
    /// This function does not expect pre-ordered vertices, only a buffer to do
    /// so it self through [`MappedMesh::ordered_face_vertices`] to avoid
    /// allocating new memory.
    pub fn compute_face_normal(
        &self,
        face: &MappedFace,
        ordered_vertices_buffer: &mut [u32],
    ) -> glam::Vec3 {
        self.ordered_face_vertices(face, ordered_vertices_buffer);
        compute_normal(ordered_vertices_buffer, &self.vertices)
    }

    /// Compute the normals of a `face` (from ordered vertices) in newly
    /// allocated memory.
    pub fn compute_face_normal_alloc(&self, face: &MappedFace) -> glam::Vec3 {
        let ordered_vertices = self.ordered_face_vertices_alloc(face);
        compute_normal(&ordered_vertices, &self.vertices)
    }
}

pub(crate) fn compute_normal(ordered_vertices: &[u32], g_vertices: &[MappedVertex]) -> glam::Vec3 {
    let mut normal = glam::Vec3::ZERO;
    let len = ordered_vertices.len();

    let face_center = {
        let mut center = ordered_vertices
            .iter()
            .take(len)
            .map(|&i| g_vertices[i as usize].point)
            .sum::<glam::Vec3>();
        center /= len as f32;
        center
    };

    for i in 0..len {
        let vi0 = ordered_vertices[i];
        let vi1 = ordered_vertices[(i + 1) % len];

        let v0 = g_vertices[vi0 as usize].point - face_center;
        let v1 = g_vertices[vi1 as usize].point - face_center;

        normal += v0.cross(v1);
    }

    normal.normalize()
}

#[derive(Clone, Debug)]
pub struct MappedFace {
    pub edges: HashSet<u32>,
    pub normal: glam::Vec3,
    pub visible: bool,
}
impl Default for MappedFace {
    fn default() -> Self {
        Self {
            edges: Default::default(),
            normal: Default::default(),
            visible: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MappedEdge {
    pub vertices: [u32; 2],
    pub faces: HashSet<u32>,
    pub visible: bool,
}
impl Default for MappedEdge {
    fn default() -> Self {
        Self {
            vertices: Default::default(),
            faces: Default::default(),
            visible: true,
        }
    }
}
impl Eq for MappedEdge {}
impl Ord for MappedEdge {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Less)
    }
}
impl PartialEq for MappedEdge {
    fn eq(&self, other: &Self) -> bool {
        self.vertices == other.vertices
    }
}
impl PartialOrd for MappedEdge {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.vertices.partial_cmp(&other.vertices)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MappedVertex {
    pub point: glam::Vec3,
    pub distance: f32,
    pub occurs: u32,
    pub visible: bool,
}
impl Default for MappedVertex {
    fn default() -> Self {
        Self {
            point: Default::default(),
            distance: Default::default(),
            occurs: Default::default(),
            visible: true,
        }
    }
}
impl MappedVertex {
    pub fn new(point: glam::Vec3) -> Self {
        Self {
            point,
            ..Default::default()
        }
    }
}
