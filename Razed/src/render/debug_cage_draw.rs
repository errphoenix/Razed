use ethel::{
    render::buffer::PartitionedTriBuffer,
    shader::{GlslUniform, ShaderKind, ShaderProgram},
};
use rendrs::pipeline::DrawPass;

use crate::data::{self, LayoutFragmentData};

pub type DebugCageDrawPass = DrawPass<DebugCageDrawCtxWrapper, 0, 0>;

#[derive(Debug)]
pub struct DebugCageDrawCtx<'data> {
    pub fragment_data: &'data PartitionedTriBuffer<{ data::FRAGMENTS_STORAGE_PARTS }>,
    pub point_size: f32,
    pub cage_points_count: i32,
}

rendrs::context_wrapper!(for<'ctx> DebugCageDrawCtx);

pub const fn pass(shader: &ShaderDebugCage) -> DebugCageDrawPass {
    let handle_view = shader.handle().view();
    DebugCageDrawPass::new(handle_view, [], [], |section, ctx| {
        let section = section.as_index();
        ctx.fragment_data.bind_shader_storage_single(
            section,
            LayoutFragmentData::PodDeformsPositions as usize,
            Some(SSBO_INDEX_POD_DEFORM_POINTS),
        );
        let count = ctx.cage_points_count;
        let point_size = ctx.point_size;
        unsafe {
            janus::gl::PointSize(point_size);
            janus::gl::DrawArrays(janus::gl::POINTS, 0, count);
        }
    })
}

macro_rules! ssbo_binding {
    (POD_Deform_Points) => {
        7
    };
}

pub const SSBO_INDEX_POD_DEFORM_POINTS: u32 = ssbo_binding!(POD_Deform_Points);

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
                    buf POD_Deform_Points => {
                        [dyn_array vec4: pod_deforms]
                    }
                }
            };

            src() "
                uint id = gl_VertexID + 1;
                vec3 deform = pod_deforms[id].xyz;
                gl_Position = projection * view * vec4(deform, 1.0);
            "
        ];

        unit ShaderKind::Pixel => [
            attribs {
                ethel::shader_glsl_attribs! {
                    output out_Color: vec4;
                }
            };

            src() "
                out_Color = vec4(1.0, 0.0, 1.0, 1.0);
            "
        ];
    }
}
