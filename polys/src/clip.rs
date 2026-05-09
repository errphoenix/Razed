//! Mesh Clipping algorithms
//!
//! Derived from [`Clipping a Mesh Against a Plane by David Eberly, Geometric Tools`](https://www.geometrictools.com/Documentation/ClipMesh.pdf)
//!

use std::collections::{HashMap, HashSet};

use crate::{Face, Facen, Plane, convex::Convex, post_process};

#[derive(Clone, Debug, Default)]
pub struct ClipMesh {
    pub vertices: Vec<ClipVertex>,
    pub edges: Vec<ClipEdge>,
    pub faces: Vec<ClipFace>,

    // Face normals cache, parallel to the `faces` vector.
    pub normals_cache: Vec<glam::Vec3>,
}

impl ClipMesh {
    pub fn new<F: Face>(convex: Convex<F>) -> Self {
        let vertices = convex
            .vertices()
            .iter()
            .map(|&p| ClipVertex::new(p))
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
                    edges.push(ClipEdge {
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

            faces.push(ClipFace::default());
        }

        ef_map.drain().for_each(|(f_i, f_edges)| {
            let face = &mut faces[f_i];
            for ei in f_edges {
                face.edges.insert(ei);
                edges[ei as usize].faces.insert(f_i as u32);
            }
        });

        let normals_cache = Vec::with_capacity(faces.len());

        Self {
            vertices,
            edges,
            faces,
            normals_cache,
        }
    }

    /// Finish all clipping operations and produce a general mesh.
    ///
    /// This operation requires that the stored cached normals (generated with
    /// [`ClipMesh::cache_current_normals`] must correspond to the mesh's
    /// normals before any clipping operation began; See
    /// [`ClipMesh::ordered_faces`].
    pub fn finish(self) -> Convex<Vec<u32>> {
        let mut points = Vec::with_capacity(self.vertices.len());
        let mut vmap = vec![-1i32; self.vertices.len()];

        for (i, cv) in self.vertices.iter().enumerate() {
            if cv.visible {
                vmap[i] = points.len() as i32;
                points.push(cv.point);
            }
        }

        let mut faces = self.ordered_faces();
        let mut i = 0;
        while i < faces.len() {
            let n_i = faces[i];
            i += 1;
            for _ in 0..n_i {
                faces[i] = vmap[faces[i] as usize] as u32;
                i += 1;
            }
        }

        // this section is very alloc and clone() heavy.
        // this is perfectly fine for the scope of these algorithms in the
        // context of Razed, as pre-compute processes.

        let mut mesh_faces = Vec::new();
        let mut current_face = Facen::<Vec<u32>>::new(Vec::new(), glam::Vec3::ZERO);
        let mut c_count = faces[0];
        let mut normal_t = vec![glam::Vec3::ZERO];

        for &i in faces.iter().skip(1) {
            if c_count == 0 {
                let mut face = current_face.clone();
                current_face.indexed.clear();

                post_process::compute_normals(&[face.clone()], &mut normal_t, &points);
                face.normal = normal_t[0];
                mesh_faces.push(face);

                c_count = i;
                continue;
            }

            current_face.indexed.push(i);
            c_count -= 1;
        }

        // flush last face
        if !current_face.indexed.is_empty() {
            post_process::compute_normals(&[current_face.clone()], &mut normal_t, &points);
            current_face.normal = normal_t[0];
            mesh_faces.push(current_face);
        }

        Convex::new(points, mesh_faces)
    }

    /// Get the mesh's faces ordered by their normals.
    ///
    /// This operation requires that the stored cached normals (generated with
    /// [`ClipMesh::cache_current_normals`] must correspond to the mesh's
    /// normals before any clipping operation began.
    ///
    /// This is necessary in order to determine whether a face is clockwise
    /// or counter-clockwise.
    pub fn ordered_faces(&self) -> Vec<u32> {
        let mut sort_vertices_buffer = Vec::new();

        let mut faces = Vec::new();
        for (i, f) in self.faces.iter().enumerate() {
            if f.visible {
                sort_vertices_buffer.clear();
                sort_vertices_buffer.resize(f.edges.len() + 1, 0u32);

                self.ordered_face_vertices(f, &mut sort_vertices_buffer);
                let olen = sort_vertices_buffer.len() - 1;
                faces.push(olen as u32);

                let nf = self.normals_cache[i];
                let no = compute_normal(&sort_vertices_buffer, &self.vertices);

                if nf.dot(no) > 0.0 {
                    // clockwise
                    for j in (0..olen).rev() {
                        faces.push(sort_vertices_buffer[j]);
                    }
                } else {
                    // counter-clockwise
                    for j in 0..olen {
                        faces.push(sort_vertices_buffer[j]);
                    }
                }
            }
        }

        faces
    }

    /// Get the ordered, contiguous vertices indices of the given `face` in
    /// newly allocated memory.
    ///
    /// See also [`ClipMesh::ordered_face_vertices`].
    pub fn ordered_face_vertices_alloc(&self, face: &ClipFace) -> Vec<u32> {
        let mut out_vertices = vec![0u32; face.edges.len() + 1];
        self.ordered_face_vertices(face, &mut out_vertices);
        out_vertices
    }

    /// Get the ordered, contiguous vertices indices of the given `face`.
    ///
    /// Note: the passed `out_vertices` mutable slice must be of minimum
    /// length of `num of face edges + 1`.
    ///
    /// See also [`ClipMesh::ordered_face_vertices_alloc`].
    pub fn ordered_face_vertices(&self, face: &ClipFace, out_vertices: &mut [u32]) {
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

    pub fn cache_current_normals(&mut self) {
        self.normals_cache.clear();
        self.normals_cache
            .resize(self.faces.len(), glam::Vec3::ZERO);

        let mut ordered_vertex_buffer = Vec::new();
        for (i, f) in self.faces.iter().enumerate() {
            ordered_vertex_buffer.clear();
            ordered_vertex_buffer.resize(f.edges.len() + 1, 0u32);
            let normal = self.compute_face_normal(f, &mut ordered_vertex_buffer);
            self.normals_cache[i] = normal;
        }
    }

    pub fn process_vertices(&mut self, clip_plane: &Plane) -> ClipResult {
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

    pub fn process_edges(&mut self) {
        for (i, e) in self.edges.iter_mut().enumerate() {
            if e.visible {
                let p0 = self.vertices[e.vertices[0] as usize];
                let p1 = self.vertices[e.vertices[1] as usize];
                let d0 = p0.distance;
                let d1 = p1.distance;

                // edge is culled, remove from all faces
                if d0 <= 0.0 && d1 <= 0.0 {
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

                if d0 >= 0.0 && d1 >= 0.0 {
                    // edge is retained; early out
                    continue;
                }

                // the plane is splitting the edge; compute intersection point
                let t = d0 / (d0 - d1);
                let intersect = (1.0 - t) * p0.point + t * p1.point;

                let idx = self.vertices.len();
                self.vertices.push(ClipVertex {
                    point: intersect,
                    distance: 0.0,
                    occurs: 0,
                    visible: true,
                });

                if d0 > 0.0 {
                    e.vertices[1] = idx as u32;
                } else {
                    e.vertices[0] = idx as u32;
                }
            }
        }
    }

    pub fn process_faces(&mut self) {
        for (i, face) in self.faces.iter_mut().enumerate() {
            if face.visible {
                for &e_i in &face.edges {
                    let edge = &self.edges[e_i as usize];
                    self.vertices[edge.vertices[0] as usize].occurs = 0;
                    self.vertices[edge.vertices[1] as usize].occurs = 0;
                }

                if let Some(polyline) = get_open_polyline(&mut self.vertices, &self.edges, face) {
                    // close the open polyline

                    let idx = self.edges.len();
                    self.edges.push(ClipEdge {
                        vertices: [polyline.start, polyline.end],
                        faces: HashSet::from([i as u32]),
                        visible: true,
                    });
                    face.edges.insert(idx as u32);
                }
            }
        }
    }

    /// Compute the normals of a `face` (from ordered vertices).
    ///
    /// This function does not expect pre-ordered vertices, only a buffer to do
    /// so it self through [`ClipMesh::ordered_face_vertices`] to avoid
    /// allocating new memory.
    pub fn compute_face_normal(
        &self,
        face: &ClipFace,
        ordered_vertices_buffer: &mut [u32],
    ) -> glam::Vec3 {
        self.ordered_face_vertices(face, ordered_vertices_buffer);
        compute_normal(ordered_vertices_buffer, &self.vertices)
    }

    /// Compute the normals of a `face` (from ordered vertices) in newly
    /// allocated memory.
    pub fn compute_face_normal_alloc(&self, face: &ClipFace) -> glam::Vec3 {
        let ordered_vertices = self.ordered_face_vertices_alloc(face);
        compute_normal(&ordered_vertices, &self.vertices)
    }
}

fn compute_normal(ordered_vertices: &[u32], g_vertices: &[ClipVertex]) -> glam::Vec3 {
    let mut normal = glam::Vec3::ZERO;
    let len = ordered_vertices.len();

    for i in 0..(len - 1) {
        let vi0 = ordered_vertices[i];
        let vi1 = ordered_vertices[i + 1];

        let v0 = g_vertices[vi0 as usize].point;
        let v1 = g_vertices[vi1 as usize].point;

        normal += v0.cross(v1);
    }

    normal.normalize()
}

#[derive(Clone, Debug)]
pub struct ClipFace {
    pub edges: HashSet<u32>,
    pub visible: bool,
}

#[derive(Clone, Debug)]
pub struct ClipEdge {
    pub vertices: [u32; 2],
    pub faces: HashSet<u32>,
    pub visible: bool,
}

impl Eq for ClipEdge {}

impl Ord for ClipEdge {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Less)
    }
}

impl PartialEq for ClipEdge {
    fn eq(&self, other: &Self) -> bool {
        self.vertices == other.vertices
    }
}

impl PartialOrd for ClipEdge {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.vertices.partial_cmp(&other.vertices)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ClipVertex {
    pub point: glam::Vec3,
    pub distance: f32,
    pub occurs: u32,
    pub visible: bool,
}

impl Default for ClipFace {
    fn default() -> Self {
        Self {
            edges: Default::default(),
            visible: true,
        }
    }
}

impl Default for ClipEdge {
    fn default() -> Self {
        Self {
            vertices: Default::default(),
            faces: Default::default(),
            visible: true,
        }
    }
}

impl Default for ClipVertex {
    fn default() -> Self {
        Self {
            point: Default::default(),
            distance: Default::default(),
            occurs: Default::default(),
            visible: true,
        }
    }
}

impl ClipVertex {
    pub fn new(point: glam::Vec3) -> Self {
        Self {
            point,
            ..Default::default()
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
    vertices: &mut [ClipVertex],
    edges: &[ClipEdge],
    face: &mut ClipFace,
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
