use ethel::{
    render::buffer::PartitionedTriBuffer,
    shader::{Constant, GlslLib, GlslStruct, GlslUniform, ShaderProgram, WriteValue},
};
use rendrs::pipeline::ComputePass;

use crate::{
    data::{CagePartitionedBuffer, LayoutXpbdDebugData},
    structure::cage,
};

pub type CageDeformComputePass = ComputePass<CageDeformComputeCtxWrapper, 0, 0>;

#[derive(Debug)]
pub struct CageDeformComputeCtx<'data> {
    pub total_cage_count: u32,
    pub cage_data: &'data CagePartitionedBuffer,
    pub lattice_data: &'data PartitionedTriBuffer<4>,
}

rendrs::context_wrapper!(for<'ctx> CageDeformComputeCtx);

pub const fn pass(shader: &ComputeShaderCageDeform) -> CageDeformComputePass {
    let handle_view = shader.compute_handle().view();
    CageDeformComputePass::new(handle_view, [], [], |section, ctx| {
        let section = section.as_index();

        ctx.cage_data.bind_ssbo_all();
        ctx.lattice_data.bind_shader_storage_single(
            section,
            LayoutXpbdDebugData::ImapNodes as usize,
            Some(SSBO_INDEX_IMAP_LATTICE),
        );
        ctx.lattice_data.bind_shader_storage_single(
            section,
            LayoutXpbdDebugData::PodNodes as usize,
            Some(SSBO_INDEX_POD_LATTICE_POSITION),
        );

        let rotations_count = ctx.total_cage_count * cage::PER_CAGE_POINTS as u32;
        let dispatch_count = rotations_count.div_ceil(CAGE_DEFORM_WORKGROUP_SIZE);
        [dispatch_count, 1, 1]
    })
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CagePoints(pub [glam::Vec4; cage::PER_CAGE_POINTS]);

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LatticeAttachments(pub [NodeAttachment; cage::PER_POINT_LATTICE_ATTACHMENTS]);

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct NodeAttachment {
    /// Index into cage's attach_lattice array
    pub index: u32,
    pub weight: f32,
}
impl WriteValue for NodeAttachment {
    fn write_value(&self, to: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(to, "NodeAttachment({}, {})", self.index, self.weight)
    }
}

macro_rules! ssbo_binding {
    (Pod_Rotation) => {
        0
    };
    (Pod_BindRef) => {
        1
    };
    (Pod_Points) => {
        2
    };
    (Pod_Points_Bind) => {
        3
    };
    (Pod_Barycenter_Bind) => {
        4
    };
    (Pod_Attachments) => {
        5
    };
    (Pod_Lut_Lattice) => {
        6
    };
    (Pod_Bind_Lattice) => {
        7
    };
    (IMap_Lattice) => {
        8
    };
    (Pod_Lattice_Position) => {
        9
    };
}

pub const SSBO_INDEX_POD_ROTATION: u32 = ssbo_binding!(Pod_Rotation);
pub const SSBO_INDEX_POD_BIND_REF: u32 = ssbo_binding!(Pod_BindRef);
pub const SSBO_INDEX_POD_POINTS: u32 = ssbo_binding!(Pod_Points);
pub const SSBO_INDEX_POD_POINTS_BIND: u32 = ssbo_binding!(Pod_Points_Bind);
pub const SSBO_INDEX_POD_BARYCENTER_BIND: u32 = ssbo_binding!(Pod_Barycenter_Bind);
pub const SSBO_INDEX_POD_ATTACHMENTS: u32 = ssbo_binding!(Pod_Attachments);
pub const SSBO_INDEX_POD_LUT_LATTICE: u32 = ssbo_binding!(Pod_Lut_Lattice);
pub const SSBO_INDEX_POD_BIND_LATTICE: u32 = ssbo_binding!(Pod_Bind_Lattice);
pub const SSBO_INDEX_IMAP_LATTICE: u32 = ssbo_binding!(IMap_Lattice);
pub const SSBO_INDEX_POD_LATTICE_POSITION: u32 = ssbo_binding!(Pod_Lattice_Position);

pub const CAGE_DEFORM_WORKGROUP_SIZE: u32 = 64;
pub const CAGE_DEFORM_PER_GROUP_CAGE_COUNT: u32 = 8;
pub const CAGE_DEFORM_PER_POINT_ATTACH_COUNT: u32 = cage::PER_POINT_LATTICE_ATTACHMENTS as u32;

pub const TYPE_CAGE_POINTS_LIST: GlslStruct = CagePointsGlslStruct::as_definition();
pub const TYPE_CAGE_POINT_ATTACHMENT_NODE: GlslStruct = NodeAttachmentGlslStruct::as_definition();
pub const TYPE_CAGE_POINT_ATTACHMENTS_LIST: GlslStruct =
    LatticeAttachmentsGlslStruct::as_definition();

ethel::shader_glsl_struct! {
    struct CagePoints {
        list[8]: [glam::Vec4; cage::PER_CAGE_POINTS] => vec4;
    }
}
ethel::shader_glsl_struct! {
    struct NodeAttachment {
        index: u32 => uint;
        weight: f32 => float;
    }
}
ethel::shader_glsl_struct! {
    struct LatticeAttachments {
        list[4]: [NodeAttachment; cage::PER_POINT_LATTICE_ATTACHMENTS] => NodeAttachment;
    }
}

ethel::shader_glsl_compute! {
    struct CageDeform > [460] {
        workgroup [64, 1, 1];

        uniform {
            length 1, total_cage_count: uint => u32;
        };

        type {
            super::TYPE_INDEX_INDIRECT
            super::TYPE_INDEX_DIRECT
            TYPE_CAGE_POINT_ATTACHMENT_NODE
            TYPE_CAGE_POINT_ATTACHMENTS_LIST
            TYPE_CAGE_POINTS_LIST
        };

        ssbo {
            ethel::shader_glsl_ssbo! {
                buf Pod_Rotation => {
                    [dyn_array vec4: pod_cage_rotation => each 8] // 8 is per-cage-points
                }
            }
            ethel::shader_glsl_ssbo! {
                buf Pod_BindRef => {
                    [dyn_array vec4: pod_cage_bindref]
                }
            }
            ethel::shader_glsl_ssbo! {
                buf Pod_Points => {
                    [dyn_array vec4: pod_cage_points => each 8] // 8 is per-cage-points
                }
            }
            ethel::shader_glsl_ssbo! {
                buf Pod_Points_Bind => {
                    [dyn_array vec4: pod_cage_points_bind => each 8] // 8 is per-cage-points
                }
            }
            ethel::shader_glsl_ssbo! {
                buf Pod_Barycenter_Bind => {
                    [dyn_array vec4: pod_cage_barycenter_bind => each 8] // 8 is per-cage-points
                }
            }
            ethel::shader_glsl_ssbo! {
                buf Pod_Attachments => {
                    [dyn_array LatticeAttachments: pod_cage_attachments => each 8] // 8 is per-cage-points
                }
            }
            ethel::shader_glsl_ssbo! {
                buf Pod_Lut_Lattice => {
                    [dyn_array IndirectIndex: pod_cage_lut_lattice => each 8] // 8 is per-cage-max-attachments
                }
            }
            ethel::shader_glsl_ssbo! {
                buf Pod_Bind_Lattice => {
                    [dyn_array vec4: pod_cage_lattice_bind => each 8] // 8 is per-cage-max-attachments
                }
            }
            ethel::shader_glsl_ssbo! {
                buf IMap_Lattice => {
                    [dyn_array DirectIndex: imap_lattice]
                }
            }
            ethel::shader_glsl_ssbo! {
                buf Pod_Lattice_Position => {
                    [dyn_array vec4: pod_lattice_position]
                }
            }
        };

        const {
            Constant::new("PER_GROUP_CAGE_COUNT", CAGE_DEFORM_PER_GROUP_CAGE_COUNT)
            Constant::new("PER_POINT_ATTACH_COUNT", CAGE_DEFORM_PER_POINT_ATTACH_COUNT)
        };

        lib {
            super::LIB_QUAT_MUL_QUAT;
            super::LIB_QUAT_ROT_VEC;
            super::LIB_VEC3_OUTER;
            super::LIB_MAT3_CONVERT_QUAT;
            LIB_SVD_EXTRACT_ROTATION;
        };

        share {
            vec3 sm_lattice_pos[PER_GROUP_CAGE_COUNT][8];
        };

        src() {
            "
            uint local       = gl_LocalInvocationID.x;
            uint cage_local_index  = local / PER_GROUP_CAGE_COUNT;
            uint point_local_index = local % PER_GROUP_CAGE_COUNT;
            uint cage_global_index = gl_WorkGroupID.x * PER_GROUP_CAGE_COUNT + cage_local_index;

            // since the number shader invocation per cage is the same as the
            // number of maximum attached lattice nodes, we can cooperatively
            // load 1 real-time lattice position only once and store it in the
            // workgroup's shared memory, saving a double-lookup.
            uint logic_local_index = point_local_index;
            IndirectIndex cage_lut_lattice[8] = pod_cage_lut_lattice[cage_global_index];
            if (cage_global_index < total_cage_count) {
                IndirectIndex id   = cage_lut_lattice[logic_local_index];
                DirectIndex direct = imap_lattice[id.index];
                vec3 node_position = pod_lattice_position[direct.index].xyz;
                sm_lattice_pos[cage_local_index][logic_local_index] = node_position;
            } else {
                // if the thread is outside the working range just fake it
                // till the barrier, zero it out because why not.
                sm_lattice_pos[cage_local_index][logic_local_index] = vec3(0.0);
            }

            barrier();

            if (cage_global_index >= total_cage_count) return;

            vec3 cage_bind_ref = pod_cage_bindref[cage_global_index].xyz;
            vec4 cage_lattice_binds[8] = pod_cage_lattice_bind[cage_global_index]; // shared per-cage lattice bind-pos cache
            vec4 point_barycenter_binds[8] = pod_cage_barycenter_bind[cage_global_index]; // per-point bind-lattice barycenter
            vec3 bind_barycenter = point_barycenter_binds[point_local_index].xyz;

            LatticeAttachments cage_attachments[8] = pod_cage_attachments[cage_global_index];
            NodeAttachment point_attachments[PER_POINT_ATTACH_COUNT] = cage_attachments[point_local_index].list;

            vec3 shared_cage_lattice_pos[8] = sm_lattice_pos[cage_local_index];

            // compute real-time barycenter
            vec3 real_barycenter = vec3(0.0);
            for (uint i = 0; i < PER_POINT_ATTACH_COUNT; ++i) {
                NodeAttachment attachment = point_attachments[i];
                vec3 real_node_pos = shared_cage_lattice_pos[attachment.index];
                real_barycenter += real_node_pos * attachment.weight;
            }
            real_barycenter -= cage_bind_ref;

            mat3 covariance = mat3(0.0);
            for (uint i = 0; i < PER_POINT_ATTACH_COUNT; ++i) {
                NodeAttachment attachment = point_attachments[i];

                vec3 real_node_pos = shared_cage_lattice_pos[attachment.index];
                vec3 bind_node_pos = cage_lattice_binds[attachment.index].xyz;

                covariance += outer(
                    real_node_pos - real_barycenter,
                    bind_node_pos - bind_barycenter
                ) * attachment.weight;
            }
            const mat3 MAT3_IDENTITY = mat3(vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0), vec3(0.0, 0.0, 1.0));
            covariance += MAT3_IDENTITY * 0.0001;

            mat3 rotation_mat = svdExtractRotation(covariance);
            vec4 rotation = matToQuat(rotation_mat);

            vec4 cage_rotations[8] = pod_cage_rotation[cage_global_index];
            cage_rotations[point_local_index] = rotation;

            vec4 cage_points_bind[8] = pod_cage_points_bind[cage_global_index];
            vec3 point_bind = cage_points_bind[point_local_index].xyz;
            vec3 deformed = rotateQuat(point_bind - bind_barycenter, rotation) + real_barycenter;

            pod_cage_points[cage_global_index][point_local_index] = vec4(deformed, 1.0);
            ";
        }
    }
}

/// SVD decomposition utility based on McAdams / Sifakis
///
/// Cretes a `svdExtractRotation` function taking in a single mat3 covariance
/// matrix, returning the mat3 rotation matrix.
pub const LIB_SVD_EXTRACT_ROTATION: GlslLib = ethel::shader_glsl_lib! {
    mat3 svdExtractRotation [
        A : mat3
    ] => "
        mat3 ATA = transpose(A) * A;
        mat3 V = mat3(1.0);

        for (int sweep = 0; sweep < 5; ++sweep) {
            float num = ATA[0][1];
            if (abs(num) > 1e-6) {
                float tau = (ATA[1][1] - ATA[0][0]) / (2.0 * num);
                float sign_tau = tau >= 0.0 ? 1.0 : -1.0;
                float t = (abs(tau) < 1e-7) ? 1.0 : (sign_tau / (abs(tau) + sqrt(1.0 + tau*tau)));
                float c = 1.0 / sqrt(1.0 + t*t);
                float s = t * c;

                vec3 v0 = V[0];
                vec3 v1 = V[1];
                V[0] = c * v0 - s * v1;
                V[1] = s * v0 + c * v1;

                float a00 = ATA[0][0];
                float a11 = ATA[1][1];
                ATA[0][0] = c*c*a00 - 2.0*s*c*num + s*s*a11;
                ATA[1][1] = s*s*a00 + 2.0*s*c*num + c*c*a11;
                ATA[0][1] = 0.0;
                ATA[1][0] = 0.0;

                float a02 = ATA[0][2];
                float a12 = ATA[1][2];
                ATA[0][2] = c*a02 - s*a12;
                ATA[1][2] = s*a02 + c*a12;
                ATA[2][0] = ATA[0][2];
                ATA[2][1] = ATA[1][2];
            }

            num = ATA[0][2];
            if (abs(num) > 1e-6) {
                float tau = (ATA[2][2] - ATA[0][0]) / (2.0 * num);
                float sign_tau = tau >= 0.0 ? 1.0 : -1.0;
                float t = (abs(tau) < 1e-7) ? 1.0 : (sign_tau / (abs(tau) + sqrt(1.0 + tau*tau)));
                float c = 1.0 / sqrt(1.0 + t*t);
                float s = t * c;

                vec3 v0 = V[0];
                vec3 v2 = V[2];
                V[0] = c * v0 - s * v2;
                V[2] = s * v0 + c * v2;

                float a00 = ATA[0][0];
                float a22 = ATA[2][2];
                ATA[0][0] = c*c*a00 - 2.0*s*c*num + s*s*a22;
                ATA[2][2] = s*s*a00 + 2.0*s*c*num + c*c*a22;
                ATA[0][2] = 0.0;
                ATA[2][0] = 0.0;

                float a01 = ATA[0][1];
                float a21 = ATA[2][1];
                ATA[0][1] = c*a01 - s*a21;
                ATA[2][1] = s*a01 + c*a21;
                ATA[1][0] = ATA[0][1];
                ATA[1][2] = ATA[2][1];
            }

            num = ATA[1][2];
            if (abs(num) > 1e-6) {
                float tau = (ATA[2][2] - ATA[1][1]) / (2.0 * num);
                float sign_tau = tau >= 0.0 ? 1.0 : -1.0;
                float t = (abs(tau) < 1e-7) ? 1.0 : (sign_tau / (abs(tau) + sqrt(1.0 + tau*tau)));
                float c = 1.0 / sqrt(1.0 + t*t);
                float s = t * c;

                vec3 v1 = V[1];
                vec3 v2 = V[2];
                V[1] = c * v1 - s * v2;
                V[2] = s * v1 + c * v2;

                float a11 = ATA[1][1];
                float a22 = ATA[2][2];
                ATA[1][1] = c*c*a11 - 2.0*s*c*num + s*s*a22;
                ATA[2][2] = s*s*a11 + 2.0*s*c*num + c*c*a22;
                ATA[1][2] = 0.0;
                ATA[2][1] = 0.0;

                float a10 = ATA[1][0];
                float a20 = ATA[2][0];
                ATA[1][0] = c*a10 - s*a20;
                ATA[2][0] = s*a10 + c*a20;
                ATA[0][1] = ATA[1][0];
                ATA[0][2] = ATA[2][0];
            }
        }

        mat3 USig = A * V;
        vec3 u0 = USig[0];
        vec3 u1 = USig[1];
        vec3 u2 = USig[2];
        float sig0 = length(u0);
        float sig1 = length(u1);
        float sig2 = length(u2);

        u0 = (sig0 > 1e-5) ? (u0 / sig0) : vec3(0.0);
        u1 = (sig1 > 1e-5) ? (u1 / sig1) : vec3(0.0);
        u2 = (sig2 > 1e-5) ? (u2 / sig2) : vec3(0.0);

        if (sig0 <= 1e-5) u0 = cross(u1, u2);
        if (sig1 <= 1e-5) u1 = cross(u2, u0);
        if (sig2 <= 1e-5) u2 = cross(u0, u1);

        if (length(u0) <= 1e-5) {
            u0 = vec3(1.0, 0.0, 0.0);
            u1 = vec3(0.0, 1.0, 0.0);
            u2 = vec3(0.0, 0.0, 1.0);
        } else if (length(u1) <= 1e-5) {
            vec3 temp = (abs(u0.x) > 0.9) ? vec3(0.0, 1.0, 0.0) : vec3(1.0, 0.0, 0.0);
            u1 = normalize(cross(u0, temp));
            u2 = cross(u0, u1);
        }

        mat3 U = mat3(u0, u1, u2);

        if (determinant(A) < 0.0) {
            if (sig0 <= sig1 && sig0 <= sig2) {
                U[0] = -U[0];
            } else if (sig1 <= sig0 && sig1 <= sig2) {
                U[1] = -U[1];
            } else {
                U[2] = -U[2];
            }
        }

        return U * transpose(V);
    "
};
