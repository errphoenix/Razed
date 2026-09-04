use crate::{Face, Facen, TriFace};

pub fn compute_vertex_normals(
    faces: &[Facen<TriFace>],
    vertices: &[glam::Vec3],
    out_normals: &mut [glam::Vec3],
) {
    let mut w_normals = {
        let mut vec = Vec::with_capacity(vertices.len());
        for _ in 0..vertices.len() {
            vec.push(Vec::new());
        }
        vec
    };

    for face in faces {
        let TriFace { a, b, c } = face.indexed;
        let v0 = vertices[a as usize];
        let v1 = vertices[b as usize];
        let v2 = vertices[c as usize];

        let t_n = face.normal;

        let a0 = (v1 - v0).dot(v2 - v0);
        let a1 = (v2 - v1).dot(v0 - v1);
        let a2 = (v0 - v2).dot(v1 - v2);

        w_normals[a as usize].push(a0 * t_n);
        w_normals[b as usize].push(a1 * t_n);
        w_normals[c as usize].push(a2 * t_n);
    }

    w_normals
        .iter()
        .enumerate()
        .for_each(|(v_i, wn)| out_normals[v_i] = wn.iter().sum::<glam::Vec3>().normalize());
}

/// Compute face normals.
///
/// The given `normals` slice must be pre-populated to the same length as
/// `faces`, with null vectors.
pub fn compute_normals<F: Face>(faces: &[F], normals: &mut [glam::Vec3], vertices: &[glam::Vec3]) {
    for (i, face) in faces.iter().enumerate() {
        let normal = &mut normals[i];

        let len = face.len();
        if len == 0 {
            continue;
        }

        let face_center = {
            let mut center = face
                .iter_indices()
                .take(len)
                .map(|i| vertices[i as usize])
                .sum::<glam::Vec3>();
            center /= len as f32;
            center
        };

        for v_i in 0..(len - 1) {
            let vi0 = face[v_i];
            let vi1 = face[(v_i + 1) % len];
            let v0 = vertices[vi0 as usize] - face_center;
            let v1 = vertices[vi1 as usize] - face_center;
            *normal += v0.cross(v1);
        }

        *normal = normal.normalize();
    }
}

pub fn compute_uv_cubic(vertex: glam::Vec3, normal: glam::Vec3, uv_scaling: f32) -> glam::Vec2 {
    let n = normal.abs();
    if n.x >= n.y && n.x >= n.z {
        glam::Vec2::new(vertex.y, vertex.z) * uv_scaling
    } else if n.y >= n.x && n.y >= n.z {
        glam::Vec2::new(vertex.x, vertex.z) * uv_scaling
    } else {
        glam::Vec2::new(vertex.x, vertex.y) * uv_scaling
    }
}
