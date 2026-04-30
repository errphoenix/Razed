use ethel::shader::{GlslLib, GlslStruct};

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

pub(super) const TYPE_MESH_METADATA: GlslStruct = ethel::mesh::MetadataGlslStruct::as_definition();
pub(super) const TYPE_MESH_VERTEX: GlslStruct = ethel::mesh::VertexGlslStruct::as_definition();

pub(super) const TYPE_INDEX_INDIRECT: GlslStruct = IndirectIndexGlslStruct::as_definition();
pub(super) const TYPE_INDEX_DIRECT: GlslStruct = DirectIndexGlslStruct::as_definition();

pub(super) const LIB_QUAT_CONVERT_MAT: GlslLib = ethel::shader_glsl_lib! {
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

pub(super) const LIB_QUAT_MUL_QUAT: GlslLib = ethel::shader_glsl_lib! {
    vec4 mulQuat [ q0: vec4, q1: vec4 ] => "
        vec4 r;
        r.x = (q0.w * q1.x) + (q0.x + q1.w) + (q0.y * q1.z) - (q0.z * q1.y);
        r.y = (q0.w * q1.y) - (q0.x * q1.z) + (q0.y * q1.w) + (q0.z * q1.x);
        r.z = (q0.w * q1.z) + (q0.x * q1.y) - (q0.y * q1.x) + (q0.z * q1.w);
        r.w = (q0.w * q1.w) - (q0.x * q1.x) - (q0.y * q1.y) - (q0.z * q1.z);
        return r;
    "
};

/// Depends on [`LIB_QUAT_MUL_QUAT`];
pub(super) const LIB_QUAT_ROT_VEC: GlslLib = ethel::shader_glsl_lib! {
    vec3 rotateQuat [ p: vec3, q: vec4 ] => "
        vec4 q_conj = vec4(-q.x, -q.y, -q.z, q.w);
        vec4 p4 = vec4(p, 1.0);

        vec4 r = mulQuat(q, p4);
        r = mulQuat(r, q_conj);
        return r.xyz;
    "
};
