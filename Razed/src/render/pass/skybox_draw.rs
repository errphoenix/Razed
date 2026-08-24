use ethel::shader::{Constant, ShaderKind};
use rendrs::pipeline::{DrawPass, OutputObject, RenderTargetAccessor, SamplerObject};

pub type SkyboxDrawPass = DrawPass<(), 1, 2>;

pub const fn pass(
    shader: &ShaderSkybox,
    skybox_sampler: SamplerObject,
    hdr_output: RenderTargetAccessor,
    depth_output: RenderTargetAccessor,
) -> SkyboxDrawPass {
    let handle_view = shader.handle().view();
    SkyboxDrawPass::new(
        handle_view,
        [skybox_sampler],
        [
            OutputObject::Color(hdr_output),
            OutputObject::Depth(depth_output),
        ],
        |_, _| unsafe {
            janus::gl::DepthFunc(janus::gl::GEQUAL);
            janus::gl::DepthMask(janus::gl::FALSE);
            janus::gl::DrawArrays(janus::gl::TRIANGLE_STRIP, 0, 24);
            janus::gl::DepthMask(janus::gl::TRUE);
            janus::gl::DepthFunc(crate::render::DEFAULT_DEPTH_FUNC);
        },
    )
}

ethel::shader_glsl! {
    struct Skybox > [460] {
        common {};

        unit ShaderKind::Pixel => [
            attribs {
                ethel::shader_glsl_attribs! {
                    input fs_uvw: vec3;
                    output outColor: vec4;
                }
            };

            uniform {
                length 1, environment_map: samplerCube => i32;
            };

            src() {
                "
                outColor = textureLod(environment_map, fs_uvw, 0);
                ";
            }
        ];

        unit ShaderKind::Vertex => [
            attribs {
                ethel::shader_glsl_attribs! {
                    output fs_uvw: vec3;
                }
            };

            uniform {
                length 1, projection: mat4 => glam::Mat4;
                length 1, view: mat4 => glam::Mat4;
            };

            const {
                CUBE_VERTICES_TRISTRIP
            };

            src() {
                "
                uint index = gl_VertexID;
                vec3 vertex = CUBE_VERTICES_TRISTRIP[index];

                mat3 view_rotation_3x3 = mat3(
                    view[0].xyz,
                    view[1].xyz,
                    view[2].xyz
                );
                mat4 view_rotation = mat4(
                    vec4(view_rotation_3x3[0], 0.0),
                    vec4(view_rotation_3x3[1], 0.0),
                    vec4(view_rotation_3x3[2], 0.0),
                    vec4(0.0)
                );

                fs_uvw = vertex;

                vec4 clip = projection * view_rotation * vec4(vertex, 0.0);
                gl_Position = vec4(clip.xy, 0.0, clip.w);
                ";
            }
        ];
    }
}

const CUBE_VERTICES_TRISTRIP: Constant<[glam::Vec3; 24]> = Constant::new(
    "CUBE_VERTICES_TRISTRIP[24]",
    [
        glam::vec3(1.0, -1.0, -1.0),
        glam::vec3(1.0, -1.0, 1.0),
        glam::vec3(1.0, 1.0, -1.0),
        glam::vec3(1.0, 1.0, 1.0),
        glam::vec3(-1.0, -1.0, 1.0),
        glam::vec3(-1.0, -1.0, -1.0),
        glam::vec3(-1.0, 1.0, 1.0),
        glam::vec3(-1.0, 1.0, -1.0),
        glam::vec3(-1.0, 1.0, -1.0),
        glam::vec3(1.0, 1.0, -1.0),
        glam::vec3(-1.0, 1.0, 1.0),
        glam::vec3(1.0, 1.0, 1.0),
        glam::vec3(-1.0, -1.0, 1.0),
        glam::vec3(1.0, -1.0, 1.0),
        glam::vec3(-1.0, -1.0, -1.0),
        glam::vec3(1.0, -1.0, -1.0),
        glam::vec3(1.0, -1.0, 1.0),
        glam::vec3(-1.0, -1.0, 1.0),
        glam::vec3(1.0, 1.0, 1.0),
        glam::vec3(-1.0, 1.0, 1.0),
        glam::vec3(-1.0, -1.0, -1.0),
        glam::vec3(1.0, -1.0, -1.0),
        glam::vec3(-1.0, 1.0, -1.0),
        glam::vec3(1.0, 1.0, -1.0),
    ],
);
