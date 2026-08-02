use ethel::shader::{Constant, GlslUniform, ShaderProgram};
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

        let dispatch_count = ctx.total_cage_count.div_ceil(WORKGROUP_SIZE);
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

pub const EXTRACT_ROTATION_ITER_COUNT: Constant<u32> = Constant::new("ITERATIONS", 4);
pub const WORKGROUP_SIZE: u32 = 64;
pub const SSBO_INDEX_INPUT_COVARIANTS: u32 = ssbo_binding!(InCovariants);
pub const SSBO_INDEX_OUTPUT_ROTATIONS: u32 = ssbo_binding!(OutRotations);

ethel::shader_glsl_compute! {
    struct CageRotate > [460] {
        workgroup [64, 1, 1];

        uniform {
            length 1, total_element_count: uint => u32;
        };

        ssbo {
            ethel::shader_glsl_ssbo! {
                buf InCovariants => {
                    [dyn_array mat3: in_covariants]
                }
            }
            ethel::shader_glsl_ssbo! {
                buf OutRotations => {
                    [dyn_array vec4: out_rotations]
                }
            }
        };

        const {
            EXTRACT_ROTATION_ITER_COUNT
        };

        lib {
            crate::render::shader_commons::LIB_QUAT_CONVERT_MAT;
            crate::render::shader_commons::LIB_QUAT_FROM_ANGLE;
            crate::render::shader_commons::LIB_QUAT_MUL_QUAT;
        };

        src() {
            "
            uint id = gl_GlobalInvocationID.x + 1;

            if (id >= total_element_count) {
                return;
            }

            const float EPS = 1e-9;

            mat3 covariant = in_covariants[id];

            vec4 rotation = out_rotations[id];

            for (uint i = 0; i < ITERATIONS; ++i) {
                mat3 R = quatToMat(rotation);

                vec3 cov0 = covariant[0];
                vec3 cov1 = covariant[1];
                vec3 cov2 = covariant[2];
                vec3 rot0 = R[0];
                vec3 rot1 = R[1];
                vec3 rot2 = R[2];

                vec3 tau = cross(rot0, cov0) + cross(rot1, cov1) + cross(rot2, cov2);

                float tau_len = length(tau);
                if (tau_len < eps) {
                    break;
                }

                float w = 1.0 / (
                    abs(dot(rot0, cov0))
                    + abs(dot(rot1, cov1))
                    + abs(dot(rot2, cov2))
                    + eps
                );

                vec3 omega = tau * w;
                float angle = length(omega);
                if (angle < eps) break;

                vec3 axis = omega / angle;
                vec4 drot = quatFromAxisAngle(axis, angle);
                rotation = normalize(mulQuat(rotation, drot));
            }

            out_rotations[id] = rotation;
            ";
        }
    }
}
