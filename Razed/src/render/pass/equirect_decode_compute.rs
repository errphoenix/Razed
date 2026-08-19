use janus::texture::Tex;
use rendrs::pipeline::{ComputePass, ImageObjectTarget};

/// Assumes input equirectangular (aka lat-long map) and output cubemap to be
/// in rgba16f format.
pub type EquirectDecodePass = ComputePass<EquirectDecodeCtxWrapper, 0, 0>;

/// Images are passed in ctx to allow for the pass to be reused for different
/// conversions.
#[derive(Debug)]
pub struct EquirectDecodeCtx<'ctx> {
    pub shader: &'ctx ComputeShaderEquirectDecode,
    pub src_equirect: ImageObjectTarget,
    pub dst_cubemap: ImageObjectTarget,
}

rendrs::context_wrapper!(for<'ctx> EquirectDecodeCtx);

pub const fn pass(shader: &ComputeShaderEquirectDecode) -> EquirectDecodePass {
    let handle_view = shader.compute_handle().view();
    EquirectDecodePass::new(handle_view, [], [], |_, ctx| {
        let EquirectDecodeCtx {
            shader,
            src_equirect,
            dst_cubemap,
        } = ctx;

        let (res_src, res_dst) = {
            let m_src = src_equirect.texture().metadata();
            let m_dst = dst_cubemap.texture().metadata();
            (
                [m_src.width() as u32, m_src.height() as u32],
                [m_dst.width() as u32, m_dst.height() as u32],
            )
        };

        shader.uniform_resolution_src_uvec2v([res_src]);
        shader.uniform_resolution_face_uvec2v([res_dst]);
        src_equirect.bind();
        dst_cubemap.bind();

        let wg_x = res_dst[0].div_ceil(WORKGROUP_SIZE_XY);
        let wg_y = res_dst[1].div_ceil(WORKGROUP_SIZE_XY);

        [wg_x, wg_y, 6]
    })
}

pub const WORKGROUP_SIZE_XY: u32 = 8;
pub const IMAGE_BINDING_SRC_EQUIRECT: u32 = 0;
pub const IMAGE_BINDING_DST_CUBEMAP: u32 = 1;

ethel::shader_glsl_compute! {
    struct EquirectDecode > [460] {
        workgroup [8, 8, 1];

        uniform {
            length 1, resolution_src : uvec2 => [u32; 2];
            length 1, resolution_face: uvec2 => [u32; 2];
        };
        image {
            on IMAGE_BINDING_SRC_EQUIRECT => src_equirect : image2D   as rgba32f readonly;
            on IMAGE_BINDING_DST_CUBEMAP  => dst_cubemap  : imageCube as rgba16f writeonly;
        };

        src() {
            "
            uint x = gl_GlobalInvocationID.x;
            uint y = gl_GlobalInvocationID.y;
            uint face = gl_GlobalInvocationID.z;

            vec2 uv_n = (vec2(x, y) + 0.5) / vec2(resolution_face);
            float u = uv_n.x * 2.0 - 1.0;
            float v = uv_n.y * 2.0 - 1.0;

            vec3 dir;
            switch(face) {
                case 0:
                    dir = vec3( 1.0, -v, -u);
                    break;
                case 1:
                    dir = vec3(-1.0, -v,  u);
                    break;
                case 2:
                    dir = vec3( u,  1.0,  v);
                    break;
                case 3:
                    dir = vec3( u, -1.0, -v);
                    break;
                case 4:
                    dir = vec3( u, -v,  1.0);
                    break;
                case 5:
                    dir = vec3(-u, -v, -1.0);
                    break;
            }
            dir = normalize(dir);

            const float PI = 3.14159;
            float phi = atan(dir.z, dir.x);
            float rho = acos(clamp(dir.y, -1.0, 1.0));
            vec2 uv = vec2(phi / (2.0*PI) + 0.5, rho / PI);
            ivec2 px = ivec2(uv * vec2(resolution_src));

            vec4 C = imageLoad(src_equirect, px);
            imageStore(dst_cubemap, ivec3(x, y, face), C);
            ";
        }
    }
}
