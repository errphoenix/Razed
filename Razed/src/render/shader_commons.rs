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
///
/// The function assumes the microfacet normal cannot form an angle
/// greater than 90 degrees, therefore they are not clamped. It assumes the
/// use of the 'half-vector' `h` as the microfacet surface normal (param. 1),
/// which is to be perfectly aligned with the microfacet normal vector and
/// points exactly halfway the vectors pointing towards the viewpoint and the
/// light, from which it is derived.
///
/// Mutually exclusive with [`LIB_NDF_MASK_G1_SMITH_GGX_KARIS_APPROX`].
pub const LIB_NDF_MASK_G1_SMITH: GlslLib = ethel::shader_glsl_lib! {
    float ndf_G1_Smith[
        micro_normal : vec3,
        to_point     : vec3,
        lambda_point : float
    ] => "
        float MdotV = dot(micro_normal, to_point);
        float d = 1.0 + lambda_point;
        return MdotV / d;
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
///
/// The function assumes the microfacet normal cannot form an angle
/// greater than 90 degrees, therefore they are not clamped. It assumes the
/// use of the 'half-vector' `h` as the microfacet surface normal (param. 1),
/// which is to be perfectly aligned with the microfacet normal vector and
/// points exactly halfway the vectors pointing towards the viewpoint and the
/// light, from which it is derived.
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
///
/// The function assumes the microfacet normal cannot form an angle
/// greater than 90 degrees, therefore they are not clamped. It assumes the
/// use of the 'half-vector' `h` as the microfacet surface normal (param. 1),
/// which is to be perfectly aligned with the microfacet normal vector and
/// points exactly halfway the vectors pointing towards the viewpoint and the
/// light, from which it is derived.
///
/// Mutually exclusive with [`LIB_NDF_MASK_G2_SMITH_HEIGHT_GGX_HAMMON_APPROX`].
pub const LIB_NDF_MASK_G2_SMITH_HEIGHT: GlslLib = ethel::shader_glsl_lib! {
    float ndf_G2_SmithHeight[
        micro_normal : vec3,
        to_view      : vec3,
        to_light     : vec3,
        lambda_view  : float,
        lambda_light : float
    ] => "
        float MdotV = dot(micro_normal, to_view );
        float MdotL = dot(micro_normal, to_light);
        float n = MdotV * MdotL;
        float d = 1.0 + lambda_view + lambda_light;
        return n / d;
    "
};

/// The Beckmann normal distribution function.
///
/// Creates the `ndf_Beckmann` function, with the following parameters:
/// * the 3d vector of the surface normal
/// * the 3d vector of the microfacet normal
/// * the scalar roughness value
///
/// The lambda function of the Beckmann NDF corresponds to
/// [`LIB_NDF_LAMBDA_A`].
pub const LIB_NDF_BECKMANN: GlslLib = ethel::shader_glsl_lib! {
    float ndf_Beckmann[
        normal       : vec3,
        micro_normal : vec3,
        roughness    : float
    ] => "
        float NdotM = dot(normal, micro_normal);
        float NdotM2 = NdotM*NdotM;
        float NdotM4 = NdotM*NdotM*NdotM*NdotM;
        float a2 = roughness*roughness;

        float id2  = NdotM2 - 1.0;
        float a2d2 = a2 * NdotM2;
        float g = exp(id2 / a2d1);

        float cd = max(NdotM, 0.0);
        float a2pi = 3.14159 * a2;
        float f = cd / a2pi;

        return f * g;
    "
};

/// Derive intermediate `a` variable for an NDF `lambda` function.
///
/// Creates the `ndf_lambda_A` function, with the following
/// parameters:
/// * the 3d vector of the surface normal
/// * the 3d vector pointing from the surface to another point, usually to the
///   viewpoint or the light's origin
/// * the scalar roughness value
///
/// This function is mutually exclusive to [`LIB_NDF_BECKMANN_LAMBDA_A_NOSQRT`].
pub const LIB_NDF_LAMBDA_A: GlslLib = ethel::shader_glsl_lib! {
    float ndf_lambda_A[
        normal    : vec3,
        point     : vec3,
        roughness : float
    ] => "
        float NdotP = dot(normal, point);
        float NdotP2 = NdotP*NdotP;
        float dr = roughness * sqrt(1.0 - NdotP2);
        return NdotP / dr;
    "
};

/// Derive intermediate `a` variable for an NDF `lambda` function.
///
/// Creates the `ndf_lambda_A` function, with the following
/// parameters:
/// * the 3d vector of the surface normal
/// * the 3d vector pointing from the surface to another point, usually to the
///   viewpoint or the light's origin
/// * the scalar roughness value
///
/// The single difference with the square-root variant is that this function
/// lacks a square-root, which makes it a little cheaper.
///
/// This is meant to be used in a case where a `lambda` function requires only
/// the square of the `a` variable, which makes the square-root unnecessary.
/// An example is the lambda function for the GGX NDF.
///
/// This function is mutually exclusive to [`LIB_NDF_LAMBDA_A`].
pub const LIB_NDF_LAMBDA_A_NOSQRT: GlslLib = ethel::shader_glsl_lib! {
    float ndf_lambda_A[
        normal    : vec3,
        point     : vec3,
        roughness : float
    ] => "
        float NdotP = dot(normal, point);
        float NdotP2 = NdotP*NdotP;
        float dr = roughness * (1.0 - NdotP2);
        return NdotP / dr;
    "
};

/// The Beckmann lambda function, required for the Beckmann NDF.
///
/// Creates the `ndf_Beckmann_lambda` function, which takes in a single scalar
/// value as its argument. This value must be the `a` variable as returned
/// by [`LIB_NDF_LAMBDA_A`].
pub const LIB_NDF_BECKMANN_LAMBDA: GlslLib = ethel::shader_glsl_lib! {
    float ndf_Beckmann_lambda[
        a : float
    ] => "
        if (a < 1.6) {
            float aa = a*a;
            float a0 = 1.259 * a;
            float a1 = 0.396 * aa;
            float a2 = 3.535 * a;
            float a3 = 2.181 * aa;
            float n = 1.0 - a0 + a1;
            float d = a2 + a3;
            return n / d;
        } else {
            return 0.0;
        }
    "
};

/// The GGX normal distribution function.
///
/// Creates the `ndf_GGX` function, with the following parameters:
/// * the 3d vector of the surface normal
/// * the 3d vector of the microfacet normal
/// * the scalar roughness value
///
/// The lambda function of the GGX NDF corresponds to [`LIB_NDF_GGX_LAMBDA`].
pub const LIB_NDF_GGX: GlslLib = ethel::shader_glsl_lib! {
    float ndf_GGX[
        normal       : vec3,
        micro_normal : vec3,
        roughness    : float
    ] => "
        float NdotM = dot(normal, micro_normal);
        float a2 = roughness*roughness;
        float n = max(0.0, NdotM) * a2;
        float am = a2 - 1.0;
        float NdotM2 = NdotM*NdotM;
        float d = NdotM2 * am + 1.0;
        float d2 = d*d;
        float pid2 = 3.14159 * d2;
        return n / pid2;
    "
};

/// The GGX lambda function, required for the GGX NDF.
///
/// Creates the `ndf_GGX_lambda` function, which takes in a single scalar
/// value as its argument. This value must be the squared `a` variable as
/// returned by [`LIB_NDF_LAMBDA_A_NOSQRT`].
///
/// [`LIB_NDF_LAMBDA_A`] can also be used for `a`, but the returned value
/// must be squared first.
///
/// Note that [`LIB_NDF_LAMBDA_A_NOSQRT`] does not require its return value to
/// be squared.
///
/// The function returns a floating-point scalar.
pub const LIB_NDF_GGX_LAMBDA: GlslLib = ethel::shader_glsl_lib! {
    float ndf_GGX_lambda[
        a2 : float
    ] => "
        float ia2 = 1.0 / a2;
        float s = sqrt(1.0 + ia2);
        float n = -1.0 + s;
        return n / 2.0;
    "
};

/// A GGX-compatible approximation for the Smith NDF masking function `G1`
/// used to mask microfacet normals.
///
/// This approximation drops the requirement for the lambda function, but
/// requires the roughness.
/// It is a specific optimization that is only compatible with the GGX model,
/// proposed by Karis in his 2013 "Real Shading in Unreal Engine 4".
///
/// Creates the `ndf_G1_Smith` function, which has the following paramaters:
/// * the microfacet surface normal 3d vector
/// * the 3d vector pointing from the surface to the point
/// * the scalar roughness value of the surface
///
/// The 'point' is usually the viewpoint or the light's origin.
///
/// The function returns a floating-point scalar.
///
/// The function assumes the microfacet normal cannot form an angle
/// greater than 90 degrees, therefore they are not clamped. It assumes the
/// use of the 'half-vector' `h` as the microfacet surface normal (param. 1),
/// which is to be perfectly aligned with the microfacet normal vector and
/// points exactly halfway the vectors pointing towards the viewpoint and the
/// light, from which it is derived.
///
/// Mutually exclusive with [`LIB_NDF_MASK_G1_SMITH`].
pub const LIB_NDF_MASK_G1_SMITH_GGX_KARIS_APPROX: GlslLib = ethel::shader_glsl_lib! {
    float ndf_G1_Smith[
        normal    : vec3,
        to_point  : vec3,
        roughness : float
    ] => "
        float NdotV = dot(normal, to_point);
        float n = 2.0 * NdotV;
        float 2ma = 2.0 - roughness;
        float d = NdotV * 2ma + roughness;
        return n / d;
    "
};

/// A GGX-compatible approximation of the Smith normal distribution joint
/// masking-shadowing function `G2`, used to mask microfacets from 2 visible
/// directions.
///
/// This approximation is described by Hammon in his 2017 GDC talk "PBR Diffuse
/// Lighting for GGX+Smith Microsurfaces".
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
/// * the scalar roughness value of the surface
///
/// The function returns a floating-point scalar.
///
/// The function assumes the microfacet normal cannot form an angle
/// greater than 90 degrees, therefore they are not clamped. It assumes the
/// use of the 'half-vector' `h` as the microfacet surface normal (param. 1),
/// which is to be perfectly aligned with the microfacet normal vector and
/// points exactly halfway the vectors pointing towards the viewpoint and the
/// light, from which it is derived.
///
/// Mutually exclusive with [`LIB_NDF_MASK_G2_SMITH_HEIGHT`].
///
/// **NOTE**: this optimization includes the term of the specular BRDF
/// denominator `4 * |dot(n,l)| * |dot(n,v)|`, which means the G2 term must
/// be multiplied outside the fraction when resolving that BRDF.
///
/// Practically, the BRDF equation should go from:
/// ```
/// (FRESNEL * G2 * NDF) / denom
/// ```
/// to:
/// ```
/// ((FRESNEL * NDF) / denom) * G2
/// ```
///
/// where `denom` is specular BRDF denominator as `4 * |dot(n,l)| * |dot(n,v)|`
pub const LIB_NDF_MASK_G2_SMITH_HEIGHT_GGX_HAMMON_APPROX: GlslLib = ethel::shader_glsl_lib! {
    float ndf_G2_SmithHeight[
        normal       : vec3,
        to_view      : vec3,
        to_light     : vec3,
        roughness    : float
    ] => "
        const float N = 0.5;
        float NdotL = abs(dot(normal, to_light));
        float NdotV = abs(dot(normal, to_view ));
        float a = 2.0 * NdotL * NdotV;
        float b = NdotL + NdotV;
        float d = mix(a, b, roughness);
        return N / d;
    "
};

ethel::shader_glsl_struct! {
    struct FresnelParams {
        albedo  : glam::Vec3 => vec3;
        fresnel : glam::Vec3 => vec3;
    }
}

/// Evaluate Fresnel-Schlick parameters.
///
/// Creates the `fresnel_Params` function, which has the following arguments:
/// * a scalar "metalness" factor from 0 to 1
/// * the RGB surface color (albedo)
/// * the RGB "dielectric fallback" color, basically the default specular
///   color of the surface if it is not a metallic (`metalness = 0`, thus
///   dielectric) surface. A good standard value is 0.04 for all 3 channels.
///
/// Returns a `FresnelParams` struct, which first field is the new `albedo`
/// surface color to be used as diffuse color, and then the second field
/// `fresnel` is the `F0` value to be used in the Fresnel function such as
/// [`Fresnel-Schlick`](LIB_FRESNEL_SCHLICK) approximation.
pub const LIB_FRESNEL_PARAMS: GlslLib = ethel::shader_glsl_lib! {
    FresnelParams fresnel_Params[
        metalness           : float,
        surface_color       : vec3,
        dielectric_fallback : vec3
    ] => "
        vec3 f0 = mix(
            dielectric_fallback,
            surface_color,
            metalness
        );
        vec3 ss = mix(
            surface_color,
            vec3(0.0),
            metalness
        );
        return FresnelParams(ss, f0);
    "
};

/// The Fresnel-Schlick approximation function.
///
/// Creates `fresnel_Schlick` function, which has the following arguments:
/// * a 3d vector normal of the surface
/// * a 3d vector that points from the surface to the light
/// * the fresnel F0 value as evaluated by [`LIB_FRESNEL_PARAMS`]
///
/// Returns a 3d vector, intended as an RGB color as the Fresnel term of the
/// specular BRDF.
pub const LIB_FRESNEL_SCHLICK: GlslLib = ethel::shader_glsl_lib! {
    vec3 fresnel_Schlick[
        normal   : vec3,
        to_light : vec3,
        fresnel  : vec3
    ] => "
        float NdotL = max(0.0, dot(normal, to_light));
        float iNdotL = 1.0 - NdotL;
        float iNdotL5 = iNdotL*iNdotL*iNdotL*iNdotL*iNdotL;
        float f = (1.0 - fresnel) * iNdotL5;
        return fresnel + f;
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
