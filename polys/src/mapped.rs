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
            .filter(|(_, v)| !v.is_empty())
            .map(|(id, indices)| {
                let normal = self.faces[id as usize].normal;
                Facen::<Vec<_>>::new(indices, normal)
            })
            .collect::<Vec<_>>();

        Convex::new(points, mesh_faces)
    }

    pub fn preserve_hard_edges(&mut self, cosine_threshold: f32) {
        let mut ord_faces = self.ordered_faces();
        if ord_faces.is_empty() {
            return;
        }

        let prev_vert_count = self.vertices.len();

        let mut v2f = vec![Vec::new(); prev_vert_count];
        for (f_i, verts) in &ord_faces {
            for &v_i in verts {
                v2f[v_i as usize].push(*f_i);
            }
        }

        let mut f2v = HashMap::new();
        for v_i in 0..prev_vert_count {
            let touch_faces = &v2f[v_i];
            if touch_faces.is_empty() {
                continue;
            }

            let mut groups = Vec::<Vec<_>>::new();
            for &f_i in touch_faces {
                let f = &self.faces[f_i as usize];
                if !f.visible || f.edges.is_empty() {
                    continue;
                }
                let n1 = f.normal;
                let mut place = false;
                for group in &mut groups {
                    let n2 = self.faces[group[0] as usize].normal;
                    if n1.dot(n2) >= cosine_threshold {
                        group.push(f_i);
                        place = true;
                        break;
                    }
                }
                if !place {
                    groups.push(vec![f_i]);
                }
            }

            for (g_i, grp) in groups.iter().enumerate() {
                let assigned = if g_i == 0 {
                    v_i as u32
                } else {
                    let new_i = self.vertices.len() as u32;
                    self.vertices.push(self.vertices[v_i]);
                    new_i
                };
                for &f_i in grp {
                    f2v.insert((v_i as u32, f_i), assigned);
                }
            }
        }

        for (f_i, verts) in &mut ord_faces {
            for v in verts {
                if let Some(&new_v) = f2v.get(&(*v, *f_i)) {
                    *v = new_v;
                }
            }
        }

        self.edges.clear();
        self.faces.iter_mut().for_each(|f| f.edges.clear());

        let mut existing = HashMap::new();
        for (f_i, verts) in ord_faces {
            for j in 0..verts.len() {
                let v0 = verts[j];
                let v1 = verts[(j + 1) % verts.len()];
                let key = if v1 < v0 { (v0, v1) } else { (v1, v0) };
                let ei = *existing.entry(key).or_insert_with(|| {
                    let new_ei = self.edges.len() as u32;
                    self.edges.push(MappedEdge {
                        vertices: [v0, v1],
                        faces: HashSet::new(),
                        visible: true,
                    });
                    new_ei
                });
                self.edges[ei as usize].faces.insert(f_i);
                self.faces[f_i as usize].edges.insert(ei);
            }
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
    assert!(
        ordered_vertices.len() >= 4,
        "ordered vertices length is less than 4: the function requires a valid, sorted closed loop of vertices"
    );
    let len = ordered_vertices.len() - 1;
    let mut normal = glam::Vec3::ZERO;
    for i in 0..len {
        let a = g_vertices[ordered_vertices[i] as usize].point;
        let b = g_vertices[ordered_vertices[(i + 1) % len] as usize].point;
        normal.x += (a.y - b.y) * (a.z + b.z);
        normal.y += (a.z - b.z) * (a.x + b.x);
        normal.z += (a.x - b.x) * (a.y + b.y);
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
