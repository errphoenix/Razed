use ethel::shader::{Constant, ShaderKind};
use rendrs::pipeline::{DrawPass, OutputObject, RenderTargetAccessor, Sampler, SamplerObject};

pub type PostDrawSkyboxPass = DrawPass<(), 1, 2>;

pub const fn draw_skybox_pass(
    shader: &ShaderPostSkybox,
    skybox_sampler: SamplerObject,
    hdr_output: RenderTargetAccessor,
    depth_output: RenderTargetAccessor,
) -> PostDrawSkyboxPass {
    let handle_view = shader.handle().view();
    PostDrawSkyboxPass::new(
        handle_view,
        [Sampler::wrap(skybox_sampler, SAMPLER_UNIT_ENVMAP)],
        [
            OutputObject::Color(hdr_output),
            OutputObject::Depth(depth_output),
        ],
        |_, _| unsafe {
            janus::gl::DepthFunc(janus::gl::GEQUAL);
            janus::gl::DepthMask(janus::gl::FALSE);
            janus::gl::DrawArrays(janus::gl::TRIANGLES, 0, 36);
            janus::gl::DepthMask(janus::gl::TRUE);
            janus::gl::DepthFunc(crate::render::DEFAULT_DEPTH_FUNC);
        },
    )
}

pub const SAMPLER_UNIT_ENVMAP: u32 = 8;

ethel::shader_glsl! {
    struct PostSkybox > [460] {
        common {};

        unit ShaderKind::Pixel => [
            attribs {
                ethel::shader_glsl_attribs! {
                    input fs_uvw: vec3;
                    output outColor: vec4;
                }
            };

            sampler {
                on SAMPLER_UNIT_ENVMAP => environment_map : samplerCube;
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
                on 0 => projection : mat4 from glam::Mat4;
                on 1 => view       : mat4 from glam::Mat4;
            };

            const {
                CUBE_VERTICES
            };

            src() {
                "
                uint index = gl_VertexID;
                vec3 vertex = CUBE_VERTICES[index];

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

const CUBE_VERTICES: Constant<[glam::Vec3; 36]> = Constant::new(
    "CUBE_VERTICES[36]",
    [
        // +x
        glam::vec3(1.0, -1.0, -1.0),
        glam::vec3(1.0, -1.0, 1.0),
        glam::vec3(1.0, 1.0, 1.0),
        glam::vec3(1.0, -1.0, -1.0),
        glam::vec3(1.0, 1.0, 1.0),
        glam::vec3(1.0, 1.0, -1.0),
        // -x
        glam::vec3(-1.0, -1.0, 1.0),
        glam::vec3(-1.0, -1.0, -1.0),
        glam::vec3(-1.0, 1.0, -1.0),
        glam::vec3(-1.0, -1.0, 1.0),
        glam::vec3(-1.0, 1.0, -1.0),
        glam::vec3(-1.0, 1.0, 1.0),
        // +y
        glam::vec3(-1.0, 1.0, -1.0),
        glam::vec3(1.0, 1.0, -1.0),
        glam::vec3(1.0, 1.0, 1.0),
        glam::vec3(-1.0, 1.0, -1.0),
        glam::vec3(1.0, 1.0, 1.0),
        glam::vec3(-1.0, 1.0, 1.0),
        // -y
        glam::vec3(-1.0, -1.0, 1.0),
        glam::vec3(1.0, -1.0, 1.0),
        glam::vec3(1.0, -1.0, -1.0),
        glam::vec3(-1.0, -1.0, 1.0),
        glam::vec3(1.0, -1.0, -1.0),
        glam::vec3(-1.0, -1.0, -1.0),
        // +z
        glam::vec3(-1.0, -1.0, 1.0),
        glam::vec3(-1.0, 1.0, 1.0),
        glam::vec3(1.0, 1.0, 1.0),
        glam::vec3(-1.0, -1.0, 1.0),
        glam::vec3(1.0, 1.0, 1.0),
        glam::vec3(1.0, -1.0, 1.0),
        // -z
        glam::vec3(1.0, -1.0, -1.0),
        glam::vec3(1.0, 1.0, -1.0),
        glam::vec3(-1.0, 1.0, -1.0),
        glam::vec3(1.0, -1.0, -1.0),
        glam::vec3(-1.0, 1.0, -1.0),
        glam::vec3(-1.0, -1.0, -1.0),
    ],
);
