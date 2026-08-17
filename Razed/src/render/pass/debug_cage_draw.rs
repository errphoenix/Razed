use ethel::shader::ShaderKind;
use rendrs::pipeline::DrawPass;

use crate::data::CagePartitionedBuffer;

pub type DebugCageDrawPass = DrawPass<DebugCageDrawCtxWrapper, 0, 0>;

#[derive(Debug)]
pub struct DebugCageDrawCtx<'data> {
    pub cage_data: &'data CagePartitionedBuffer,
    pub point_size: f32,
    pub cage_total_count: u32,
}

rendrs::context_wrapper!(for<'ctx> DebugCageDrawCtx);

pub const fn pass(shader: &ShaderDebugCage) -> DebugCageDrawPass {
    let handle_view = shader.handle().view();
    DebugCageDrawPass::new(handle_view, [], [], |_, ctx| {
        ctx.cage_data
            .bind_ssbo_pod_bindref(Some(SSBO_INDEX_POD_CAGE_REFERENCE));
        ctx.cage_data
            .bind_ssbo_pod_points(Some(SSBO_INDEX_POD_CAGE_POINTS));

        let count = ctx.cage_total_count as i32 * crate::structure::cage::PER_CAGE_POINTS as i32;
        let point_size = ctx.point_size;
        unsafe {
            janus::gl::PointSize(point_size);
            janus::gl::DrawArrays(janus::gl::POINTS, 0, count);
        }
    })
}

macro_rules! ssbo_binding {
    (POD_Cage_Reference) => {
        2
    };
    (POD_Cage_Points) => {
        3
    };
}

pub const SSBO_INDEX_POD_CAGE_REFERENCE: u32 = ssbo_binding!(POD_Cage_Reference);
pub const SSBO_INDEX_POD_CAGE_POINTS: u32 = ssbo_binding!(POD_Cage_Points);

ethel::shader_glsl! {
    struct DebugCage > [460] {
        common {};

        unit ShaderKind::Vertex => [
            uniform {
                length 1, projection: mat4 => glam::Mat4;
                length 1, view: mat4 => glam::Mat4;
            };

            ssbo {
                ethel::shader_glsl_ssbo! {
                    buf POD_Cage_Reference => {
                        [dyn_array vec4: pod_cage_reference]
                    }
                }
                ethel::shader_glsl_ssbo! {
                    buf POD_Cage_Points => {
                        [dyn_array vec4: pod_cage_points => each 8]
                    }
                }
            };

            src() {
                "
                uint id = gl_VertexID + 1;

                uint cage_id = id / 8;
                uint point_id = id % 8;

                vec3 reference = pod_cage_reference[cage_id].xyz;
                vec4[8] points = pod_cage_points[cage_id];
                vec3 point = points[point_id].xyz + reference;

                gl_Position = projection * view * vec4(point, 1.0);
                ";
            }
        ];

        unit ShaderKind::Pixel => [
            attribs {
                ethel::shader_glsl_attribs! {
                    output out_Color: vec4;
                }
            };

            src() {
                "
                out_Color = vec4(1.0, 0.0, 1.0, 0.8);
                ";
            }
        ];
    }
}
