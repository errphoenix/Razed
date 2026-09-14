//! Mesh Clipping algorithms for [`MappedMesh`].
//!
//! Derived from [`Clipping a Mesh Against a Plane by David Eberly, Geometric Tools`](https://www.geometrictools.com/Documentation/ClipMesh.pdf)
//!

use std::collections::HashSet;

use crate::{
    Plane,
    mapped::{MappedEdge, MappedFace, MappedMesh, MappedVertex},
};

impl MappedMesh {
    /// Get the mesh's faces ordered by their normals.
    ///
    /// Each element contains the original face index of the face and a
    /// collection of the face's ordered vertex indices.
    pub fn ordered_faces(&self) -> Vec<(u32, Vec<u32>)> {
        let mut sort_vertices_buffer = Vec::new();

        let mut faces = Vec::new();
        for (i, f) in self.faces.iter().enumerate() {
            if f.visible && !f.edges.is_empty() {
                sort_vertices_buffer.clear();
                sort_vertices_buffer.resize(f.edges.len() + 1, 0u32);

                self.ordered_face_vertices(f, &mut sort_vertices_buffer);
                let olen = sort_vertices_buffer.len() - 1;
                let mut face = Vec::with_capacity(olen);

                let nf = f.normal;
                let no = super::mapped::compute_normal(&sort_vertices_buffer, &self.vertices);

                if nf.dot(no) > 0.0 {
                    // clockwise
                    for j in 0..olen {
                        face.push(sort_vertices_buffer[j]);
                    }
                } else {
                    // counter-clockwise
                    for j in (0..olen).rev() {
                        face.push(sort_vertices_buffer[j]);
                    }
                }

                faces.push((i as u32, face));
            }
        }

        faces
    }

    /// Get the ordered, contiguous vertices indices of the given `face` in
    /// newly allocated memory.
    ///
    /// See also [`MappedMesh::ordered_face_vertices`].
    pub fn ordered_face_vertices_alloc(&self, face: &MappedFace) -> Vec<u32> {
        let mut out_vertices = vec![0u32; face.edges.len() + 1];
        self.ordered_face_vertices(face, &mut out_vertices);
        out_vertices
    }

    /// Get the ordered, contiguous vertices indices of the given `face`.
    ///
    /// Note: the passed `out_vertices` mutable slice must be of minimum
    /// length of `num of face edges + 1`.
    ///
    /// See also [`MappedMesh::ordered_face_vertices_alloc`].
    pub fn ordered_face_vertices(&self, face: &MappedFace, out_vertices: &mut [u32]) {
        debug_assert!(out_vertices.len() >= face.edges.len() + 1);

        if face.edges.is_empty() {
            return;
        }

        let mut edges = face.edges.iter().copied().collect::<Vec<u32>>();

        // bubble-sort vertices for each edge
        {
            let mut i0 = 0;
            let mut i1 = 1;
            let mut choice = 1;

            while i1 < edges.len() - 1 {
                let current = self.edges[edges[i0] as usize].vertices[choice];

                for j in i1..edges.len() {
                    let e_t = &self.edges[edges[j] as usize];
                    if e_t.vertices[0] == current {
                        edges.swap(i1, j);
                        choice = 1;
                        break;
                    }
                    if e_t.vertices[1] == current {
                        edges.swap(i1, j);
                        choice = 0;
                        break;
                    }
                }

                i0 = i1;
                i1 += 1;
            }
        }

        out_vertices[0] = self.edges[edges[0] as usize].vertices[0];
        out_vertices[1] = self.edges[edges[0] as usize].vertices[1];

        for (i, &sorted_ei) in edges.iter().enumerate().skip(1) {
            let m_edge = &self.edges[sorted_ei as usize];
            if m_edge.vertices[0] == out_vertices[i] {
                out_vertices[i + 1] = m_edge.vertices[1];
            } else {
                out_vertices[i + 1] = m_edge.vertices[0];
            }
        }
    }

    pub fn clip_process_vertices(&mut self, clip_plane: &Plane) -> ClipResult {
        let mut n = 0;
        let mut p = 0;

        for v in &mut self.vertices {
            if v.visible {
                v.distance = clip_plane.normal.dot(v.point) - clip_plane.d;
                if v.distance >= EPS {
                    p += 1;
                } else if v.distance <= -EPS {
                    n += 1;
                    v.visible = false;
                } else {
                    // point is within floating-point tolerance
                    v.distance = 0.0;
                }
            }
        }

        if n == 0 {
            return ClipResult::None;
        }
        if p == 0 {
            return ClipResult::Whole;
        }

        return ClipResult::Partial;
    }

    pub fn clip_process_edges(&mut self) {
        for (i, e) in self.edges.iter_mut().enumerate() {
            if e.visible {
                let p0 = self.vertices[e.vertices[0] as usize];
                let p1 = self.vertices[e.vertices[1] as usize];
                let d0 = p0.distance;
                let d1 = p1.distance;

                // edge is culled, remove from all faces
                if d0 <= EPS && d1 <= EPS {
                    for &f_i in &e.faces {
                        let face = &mut self.faces[f_i as usize];
                        face.edges.remove(&(i as u32));
                        if face.edges.is_empty() {
                            face.visible = false;
                        }
                    }
                    e.visible = false;
                    continue;
                }

                if d0 >= EPS && d1 >= EPS {
                    // edge is retained; early out
                    continue;
                }

                // the plane is splitting the edge; compute intersection point
                let t = d0 / (d0 - d1);
                let intersect = (1.0 - t) * p0.point + t * p1.point;

                let idx = self.vertices.len();
                self.vertices.push(MappedVertex {
                    point: intersect,
                    distance: 0.0,
                    occurs: 0,
                    visible: true,
                });

                if d0 > EPS {
                    e.vertices[1] = idx as u32;
                } else {
                    e.vertices[0] = idx as u32;
                }
            }
        }
    }

    pub fn clip_process_faces(&mut self, clip_plane: &Plane) {
        let closed_face = MappedFace {
            normal: -clip_plane.normal,
            ..Default::default()
        };
        let cfi = self.faces.len();
        self.faces.push(closed_face);

        for i in 0..=cfi {
            if self.faces[i].visible {
                for &e_i in &self.faces[i].edges {
                    let edge = &self.edges[e_i as usize];
                    self.vertices[edge.vertices[0] as usize].occurs = 0;
                    self.vertices[edge.vertices[1] as usize].occurs = 0;
                }

                if let Some(polyline) =
                    get_open_polyline(&mut self.vertices, &self.edges, &mut self.faces[i])
                {
                    // close the open polyline

                    let idx = self.edges.len();
                    self.edges.push(MappedEdge {
                        vertices: [polyline.start, polyline.end],
                        faces: HashSet::from([i as u32, cfi as u32]),
                        visible: true,
                    });
                    self.faces[i].edges.insert(idx as u32);
                    self.faces[cfi].edges.insert(idx as u32);
                }
            }
        }
    }
}

const EPS: f32 = 0.01;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClipResult {
    None = 1,
    Whole = -1,
    Partial = 0,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Polyline {
    pub start: u32,
    pub end: u32,
}

pub fn get_open_polyline(
    vertices: &mut [MappedVertex],
    edges: &[MappedEdge],
    face: &mut MappedFace,
) -> Option<Polyline> {
    // count number of occurrences for each vertex in the polyline
    // resulting `occurs` values must be 1 or 2
    for &e_i in &face.edges {
        let edge = &edges[e_i as usize];
        vertices[edge.vertices[0] as usize].occurs += 1;
        vertices[edge.vertices[1] as usize].occurs += 1;
    }

    // determine whether the polyline is open
    let mut start = None;
    let mut end = None;
    for &e_i in &face.edges {
        let edge = &edges[e_i as usize];
        let i0 = edge.vertices[0];
        let i1 = edge.vertices[1];

        if vertices[i0 as usize].occurs == 1 {
            if start.is_none() {
                start = Some(i0)
            } else if end.is_none() {
                end = Some(i0);
            }
        }
        if vertices[i1 as usize].occurs == 1 {
            if start.is_none() {
                start = Some(i1)
            } else if end.is_none() {
                end = Some(i1);
            }
        }
    }

    if let Some(start) = start
        && let Some(end) = end
    {
        Some(Polyline { start, end })
    } else {
        None
    }
}
