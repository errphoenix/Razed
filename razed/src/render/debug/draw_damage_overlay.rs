use ethel::shader::ShaderKind;
use rendrs::{DrawPass, OutputObject, RenderTargetAccessor};

pub type DebugDrawDamageOverlayPass = DrawPass<DebugDrawDamageOverlayCtxWrapper, 0, 2>;

pub const fn draw_damage_overlay_pass(
    shader: &ShaderDebugDrawDamageOverlay,
    ldr_output: RenderTargetAccessor,
    depth_output: RenderTargetAccessor,
) -> DebugDrawDamageOverlayPass {
    let handle_view = shader.handle().view();
    DebugDrawDamageOverlayPass::new(
        handle_view,
        [],
        [
            OutputObject::Color(ldr_output),
            OutputObject::Depth(depth_output),
        ],
        |_section, ctx| {
            let count = ctx.index_count as i32;
            unsafe {
                janus::gl::DrawArrays(janus::gl::TRIANGLES, 0, count);
            }
        },
    )
}

#[derive(Debug)]
pub struct DebugDrawDamageOverlayCtx {
    pub index_count: u32,
}
rendrs::context_wrapper!(DebugDrawDamageOverlayCtx);

ethel::shader_glsl! {
    struct DebugDrawDamageOverlay > [460] {
        common {};

        unit ShaderKind::Vertex => [
            uniform {
                on 0 => projection : mat4  from glam::Mat4;
                on 1 => view       : mat4  from glam::Mat4;
                on 2 => position   : vec3  from glam::Vec3;
                on 3 => radius     : float from f32;
                on 4 => mesh_id    : uint  from ethel::MeshId;
            };
            type {
                super::ETH_TYPE_MESH_METADATA
                super::ETH_TYPE_MESH_TRIANGLE
                super::ETH_TYPE_MESH_VERTEX
            };
            ssbo {
                super::ETH_MESH_SSBO_STATIC
                super::ETH_MESH_SSBO_TRIS
            };

            src() {
                "
                const MeshMetadata metadata = eth_meshmeta[mesh_id];
                const uint index_base = metadata.tris_offset * 3;
                const uint element = gl_VertexID + index_base;

                const uint tri_id = element / 3;

                const MeshTriangle tri = eth_tris_buffer[tri_id];
                const uint tri_index_internal = element % 3;
                uint index;
                if (tri_index_internal == 0) {
                    index = tri.v0;
                } else if (tri_index_internal == 1) {
                    index = tri.v1;
                } else {
                    index = tri.v2;
                }

                const MeshVertex ethVertex = eth_vertex_buffer[index];
                vec3 vertex = vec3(ethVertex.pos_x, ethVertex.pos_y, ethVertex.pos_z);

                vertex *= radius;
                vertex += position;

                gl_Position = projection * view * vec4(vertex, 1.0);
                ";
            }
        ];

        unit ShaderKind::Pixel => [
            attribs {
                ethel::shader_glsl_attribs! {
                    output out_Color: vec4;
                }
            };
            uniform {
                on 5 => color   : vec3  from glam::Vec3;
                on 6 => opacity : float from f32;
            };

            src() {
                "
                out_Color = vec4(color, opacity);
                ";
            }
        ];
    }
}
