//! Mesh Clipping algorithms
//!
//! Derived from [`Clipping a Mesh Against a Plane by David Eberly, Geometric Tools`](https://www.geometrictools.com/Documentation/ClipMesh.pdf)
//!

use std::collections::HashSet;

use crate::Plane;

#[derive(Clone, Debug, Default)]
pub struct ClipMesh {
    pub vertices: Vec<ClipVertex>,
    pub edges: Vec<ClipEdge>,
    pub faces: Vec<ClipFace>,
}

impl ClipMesh {
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

#[derive(Clone, Copy, Debug)]
pub struct ClipVertex {
    pub point: glam::Vec3,
    pub distance: f32,
    pub occurs: u32,
    pub visible: bool,
}

// todo: adjust based on world scale
const EPS: f32 = f32::EPSILON;

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
