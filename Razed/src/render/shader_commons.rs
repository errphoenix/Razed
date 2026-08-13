#![allow(unused)]

use ethel::shader::{Constant, GlslAttribute, GlslLib, GlslStruct};

/// Minimal pixel shader attributes.
///
/// Includes the following input attributes:
/// * `fs_world: vec3` the world position of the fragment
/// * `fs_normal: vec3` the surface normal of the fragment
/// * `fs_uv: vec2` the uv map coordinate of the fragment
///
/// And the `outColor: vec4` output for framebuffer color output.
pub const ATTRIBS_PIXEL_MINIMAL: GlslAttribute = ethel::shader_glsl_attribs! {
    input fs_world: vec3;
    input fs_normal: vec3;
    input fs_uv: vec2;
    output outColor: vec4;
};

pub const CONST_AMBIENT_LIGHT: Constant<f32> = Constant::new("LIGHT_AMBIENT", 0.25);

ethel::shader_glsl_struct! {
    struct IndirectIndex {
        index: u32 => uint;
        generation: u32 => uint;
    }
}

ethel::shader_glsl_struct! {
    struct DirectIndex {
        index: u32 => uint;
        generation: u32 => uint;
    }
}

pub const TYPE_MESH_METADATA: GlslStruct = ethel::mesh::MetadataGlslStruct::as_definition();
pub const TYPE_MESH_VERTEX: GlslStruct = ethel::mesh::VertexGlslStruct::as_definition();

pub const TYPE_INDEX_INDIRECT: GlslStruct = IndirectIndexGlslStruct::as_definition();
pub const TYPE_INDEX_DIRECT: GlslStruct = DirectIndexGlslStruct::as_definition();

pub const LIB_QUAT_CONVERT_MAT: GlslLib = ethel::shader_glsl_lib! {
    mat3 quatToMat [ q: vec4 ] => "
        mat3 m = mat3(0.0);

        float sqx = q.x * q.x;
        float sqy = q.y * q.y;
        float sqz = q.z * q.z;
        float sqw = q.w * q.w;

        float invs = 1.0 / (sqx + sqy + sqz + sqw);
        m[0][0] = (sqx - sqy - sqz + sqw) * invs;
        m[1][1] = (-sqx + sqy - sqz + sqw) * invs;
        m[2][2] = (-sqx - sqy + sqz + sqw) * invs;

        float tmp1 = q.x * q.y;
        float tmp2 = q.z * q.w;
        m[1][0] = 2.0 * (tmp1 + tmp2) * invs;
        m[0][1] = 2.0 * (tmp1 - tmp2) * invs;

        tmp1 = q.x * q.z;
        tmp2 = q.y * q.w;
        m[2][0] = 2.0 * (tmp1 - tmp2) * invs;
        m[0][2] = 2.0 * (tmp1 + tmp2) * invs;

        tmp1 = q.y * q.z;
        tmp2 = q.x * q.w;
        m[2][1] = 2.0 * (tmp1 + tmp2) * invs;
        m[1][2] = 2.0 * (tmp1 - tmp2) * invs;

        return m;
    "
};

pub const LIB_QUAT_MUL_QUAT: GlslLib = ethel::shader_glsl_lib! {
    vec4 mulQuat [ q0: vec4, q1: vec4 ] => "
        vec4 r;
        r.x = (q0.w * q1.x) + (q0.x + q1.w) + (q0.y * q1.z) - (q0.z * q1.y);
        r.y = (q0.w * q1.y) - (q0.x * q1.z) + (q0.y * q1.w) + (q0.z * q1.x);
        r.z = (q0.w * q1.z) + (q0.x * q1.y) - (q0.y * q1.x) + (q0.z * q1.w);
        r.w = (q0.w * q1.w) - (q0.x * q1.x) - (q0.y * q1.y) - (q0.z * q1.z);
        return r;
    "
};

/// `Quaternion x Vector3` rotation utility function.
///
/// Creates the `rotateQuat` function, taking, in order, the `vec3` to rotate
/// and then the quaternion rotation represented by a `vec4`.
///
/// Returns the rotated `vec3`, the given vector is not changed.
///
/// Depends on [`LIB_QUAT_MUL_QUAT`];
pub const LIB_QUAT_ROT_VEC: GlslLib = ethel::shader_glsl_lib! {
    vec3 rotateQuat [ p: vec3, q: vec4 ] => "
        vec4 q_conj = vec4(-q.x, -q.y, -q.z, q.w);
        vec4 p4 = vec4(p, 1.0);

        vec4 r = mulQuat(q, p4);
        r = mulQuat(r, q_conj);
        return r.xyz;
    "
};

pub const LIB_QUAT_SLERP: GlslLib = ethel::shader_glsl_lib! {
    vec4 slerpQuat [ q0: vec4, q1: vec4, t: float ] => "
        float dotp = dot(normalize(q0), normalize(q1));

        // non-orthogonal
        if (abs(dotp) > 0.9999) {
            if (t <= 0.5) {
                return q0;
            }
            return q1;
        }

        float theta = acos(dotp);
        vec4 B = ((q0 * sin((1.0 - t) * theta) + q1 * sin(t * theta)) / sin(theta));
        B.w = 1.0;
        return B;
    "
};

pub const LIB_MAT3_COFACTOR: GlslLib = ethel::shader_glsl_lib! {
    mat3 cofactor3 [ m: mat3 ] => "
        vec3 c0 = m[0];
        vec3 c1 = m[1];
        vec3 c2 = m[2];
        return mat3(
            cross(c1, c2),
            cross(c2, c0),
            cross(c0, c1)
        );
    "
};

pub const LIB_QUAT_FROM_ANGLE: GlslLib = ethel::shader_glsl_lib! {
    vec4 quatFromAxisAngle [
        axis  : vec3,
        angle : float
    ] => "
        float half_angle = angle * 0.5;
        float s = sin(half_angle);
        return vec4(axis * s, cos(half_angle));
    "
};

/// Utility function to convert a 3x3 matrix to a normalized quaternion.
///
/// Creates a single `matToQuat` function, which takes in a `mat3` value and
/// returns a `vec4` quaternion.
pub const LIB_MAT3_CONVERT_QUAT: GlslLib = ethel::shader_glsl_lib! {
    vec4 matToQuat [
        m : mat3
    ] => "
        float tr = m[0][0] + m[1][1] + m[2][2];
        vec4 q;
        if (tr > 0.0) {
            float S = sqrt(tr + 1.0) * 2.0;
            q.w = 0.25 * S;
            q.x = (m[1][2] - m[2][1]) / S;
            q.y = (m[2][0] - m[0][2]) / S;
            q.z = (m[0][1] - m[1][0]) / S;
        } else if ((m[0][0] > m[1][1]) && (m[0][0] > m[2][2])) {
            float S = sqrt(1.0 + m[0][0] - m[1][1] - m[2][2]) * 2.0;
            q.w = (m[1][2] - m[2][1]) / S;
            q.x = 0.25 * S;
            q.y = (m[0][1] + m[1][0]) / S;
            q.z = (m[2][0] + m[0][2]) / S;
        } else if (m[1][1] > m[2][2]) {
            float S = sqrt(1.0 + m[1][1] - m[0][0] - m[2][2]) * 2.0;
            q.w = (m[2][0] - m[0][2]) / S;
            q.x = (m[0][1] + m[1][0]) / S;
            q.y = 0.25 * S;
            q.z = (m[1][2] + m[2][1]) / S;
        } else {
            float S = sqrt(1.0 + m[2][2] - m[0][0] - m[1][1]) * 2.0;
            q.w = (m[0][1] - m[1][0]) / S;
            q.x = (m[2][0] + m[0][2]) / S;
            q.y = (m[1][2] + m[2][1]) / S;
            q.z = 0.25 * S;
        }
        return normalize(q);
    "
};

/// The Smith NDF masking function `G1` used to mask microfacet normals.
///
/// Creates the `ndf_G1_Smith` function, which has the following paramaters:
/// * the microfacet surface normal 3d vector
/// * the 3d vector pointing from the surface to the point
/// * the result of the NDF's `lambda` function for the 3d vector pointing
///   from the surface to the point
///
/// The 'point' is usually the viewpoint or the light's origin.
///
/// The `lambda` function is different depending on the chosen NDF.
///
/// The function returns a floating-point scalar.
pub const LIB_NDF_MASK_G1_SMITH: GlslLib = ethel::shader_glsl_lib! {
    float ndf_G1_Smith[
        micro_normal : vec3,
        to_point     : vec3,
        lambda_point : float
    ] => "
        float MdotV = dot(micro_normal, to_point);
        float n = MdotV > 0.0 ? 1.0 : 0.0;
        float d = 1.0 + lambda_point;
        return n / d;
    "
};

/// The Smith normal distribution joint masking-shadowing function `G2`, used
/// to mask microfacets from 2 visible directions.
///
/// This is the "separable" form defined by Heitz: the simplest, but prone to
/// over-darkening as it incorrectly uncorrelates masking and shadowing.
/// However, some applications are known to still utilize this approach.
///
/// Creates the `ndf_G2_SmithSeparable` function, which has the following
/// parameters:
/// * the microfacet surface normal 3d vector
/// * the 3d vector pointing from the surface to the viewpoint
/// * the 3d vector pointing away from the surface to the light's origin
/// * the result of the NDF's `lambda` function for the 3d vector pointing
///   from the surface to the viewpoint
/// * the result of the NDF's `lambda` function for the 3d vector pointing
///   from the surface to the light's origin
///
/// The `lambda` function is different depending on the chosen NDF.
///
/// The function returns a floating-point scalar.
///
/// Depends on [`LIB_NDF_MASK_G1_SMITH`]
pub const LIB_NDF_MASK_G2_SMITH_SEPARABLE: GlslLib = ethel::shader_glsl_lib! {
    float ndf_G2_SmithSeparable[
        micro_normal : vec3,
        to_view      : vec3,
        to_light     : vec3,
        lambda_view  : float,
        lambda_light : float
    ] => "
        float a = ndf_G1_Smith(micro_normal, to_view, lambda_view);
        float b = ndf_G1_Smith(micro_normal, to_light, lambda_light);
        return a * b;
    "
};

/// The Smith normal distribution joint masking-shadowing function `G2`, used
/// to mask microfacets from 2 visible directions.
///
/// This is the "height-correlated" form defined by Heitz: this form takes
/// advantage of the fact that the light and view directions are correlated
/// by their relative alignment, but more importantly they both relate to the
/// point's height relative to the rest of the surface.
///
/// Creates the `ndf_G2_SmithHeight` function, which has the following
/// parameters:
/// * the microfacet surface normal 3d vector
/// * the 3d vector pointing from the surface to the viewpoint
/// * the 3d vector pointing away from the surface to the light's origin
/// * the result of the NDF's `lambda` function for the 3d vector pointing
///   from the surface to the viewpoint
/// * the result of the NDF's `lambda` function for the 3d vector pointing
///   from the surface to the light's origin
///
/// The `lambda` function is different depending on the chosen NDF.
///
/// The function returns a floating-point scalar.
pub const LIB_NDF_MASK_G2_SMITH_HEIGHT: GlslLib = ethel::shader_glsl_lib! {
    float ndf_G2_SmithHeight[
        micro_normal : vec3,
        to_view      : vec3,
        to_light     : vec3,
        lambda_view  : float,
        lambda_light : float
    ] => "
        float MdotV = dot(micro_normal, to_view ) > 0.0 ? 1.0 : 0.0;
        float MdotL = dot(micro_normal, to_light) > 0.0 ? 1.0 : 0.0;
        float n = MdotV * MdotL;
        float d = 1.0 + lambda_view + lambda_light;
        return n / d;
    "
};

/// Vector3 outer-product utility function.
///
/// Creates the `outer` function, taking in 2 `vec3` parameters, returning
/// the `mat3` outer product.
pub const LIB_VEC3_OUTER: GlslLib = ethel::shader_glsl_lib! {
    mat3 outer[
        a : vec3,
        b : vec3
    ] => "
        return mat3(
            a * b.x,
            a * b.y,
            a * b.z
        );
    "
};
