use crate::Face;

/// Compute face normals.
///
/// The given `normals` slice must be pre-populated to the same length as
/// `faces`, with null vectors.
pub fn compute_normals<F: Face>(faces: &[F], normals: &mut [glam::Vec3], vertices: &[glam::Vec3]) {
    for (i, face) in faces.iter().enumerate() {
        let normal = &mut normals[i];

        let len = face.len();
        for v_i in 0..=(len - 2) {
            let vi0 = face[v_i];
            let vi1 = face[v_i + 1];
            let v0 = vertices[vi0 as usize];
            let v1 = vertices[vi1 as usize];
            *normal += v0.cross(v1);
        }

        *normal = normal.normalize();
    }
}
