use ethel::shader::{Constant, GlslLib, GlslUniform, ShaderProgram};
use rendrs::pipeline::ComputePass;

use crate::data::CageDataPartitionedTriBuffer;

pub type CageRotateComputePass = ComputePass<CageRotateComputeCtxWrapper, 0, 0>;

#[derive(Debug)]
pub struct CageRotateComputeCtx<'data> {
    pub total_cage_count: u32,
    pub cage_data: &'data CageDataPartitionedTriBuffer,
}

rendrs::context_wrapper!(for<'ctx> CageRotateComputeCtx);

pub const fn pass(shader: &ComputeShaderCageRotate) -> CageRotateComputePass {
    let handle_view = shader.compute_handle().view();
    CageRotateComputePass::new(handle_view, [], [], |section, ctx| {
        let section = section.as_index();

        let data = ctx.cage_data;
        data.bind_ssbo_pod_cages_covariants(section, None);
        data.bind_ssbo_pod_cages_rotations(section, None);

        let rotations_count = ctx.total_cage_count * CONST_CAGE_SIZE.value();
        let dispatch_count = rotations_count.div_ceil(WORKGROUP_SIZE);
        [dispatch_count, 1, 1]
    })
}

macro_rules! ssbo_binding {
    (InCovariants) => {
        5
    };
    (OutRotations) => {
        6
    };
}

pub const CONST_EXTRACT_ROTATION_ITER_COUNT: Constant<u32> = Constant::new("ITERATIONS", 4);
pub const CONST_CAGE_SIZE: Constant<u32> =
    Constant::new("CAGE_SIZE", crate::structure::cage::PER_CAGE_POINTS as u32);
pub const WORKGROUP_SIZE: u32 = 64;
pub const SSBO_INDEX_INPUT_COVARIANTS: u32 = ssbo_binding!(InCovariants);
pub const SSBO_INDEX_OUTPUT_ROTATIONS: u32 = ssbo_binding!(OutRotations);

ethel::shader_glsl_compute! {
    struct CageRotate > [460] {
        workgroup [64, 1, 1];

        uniform {
            length 1, total_cage_count: uint => u32;
        };

        ssbo {
            ethel::shader_glsl_ssbo! {
                buf InCovariants => {
                    [dyn_array mat4: in_covariants => each 8]
                }
            }
            ethel::shader_glsl_ssbo! {
                buf OutRotations => {
                    [dyn_array vec4: out_rotations => each 8]
                }
            }
        };

        const {
            CONST_EXTRACT_ROTATION_ITER_COUNT
            CONST_CAGE_SIZE
        };

        lib {
            crate::render::shader_commons::LIB_MAT3_CONVERT_QUAT;
            LIB_SVD_EXTRACT_ROTATION;
        };

        src() {
            "
            uint id = gl_GlobalInvocationID.x;
            uint cage_id = id / CAGE_SIZE;
            uint point_id = id % CAGE_SIZE;

            if (cage_id >= total_cage_count) {
                return;
            }

            const float EPS = 0.0001;

            mat4 covariant4 = in_covariants[cage_id][point_id];
            mat3 covariant = mat3(
                covariant4[0].xyz,
                covariant4[1].xyz,
                covariant4[2].xyz
            );

            mat3 rotation = svdExtractRotation(covariant);
            vec4 q = matToQuat(rotation);
            q = vec4(0.0, 0.0, 0.0, 1.0);

            out_rotations[cage_id][point_id] = q;
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

        for (int sweep = 0; sweep < 6; ++sweep) {
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
