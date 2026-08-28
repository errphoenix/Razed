use rendrs::{
    graphics::PixelResolution,
    pipeline::{
        ComputePass, ImageAccessKind, ImageObject, ImageObjectTarget, RenderTargetAccessor,
    },
};

use crate::render::graphics::RenderParams;

pub type TonemapVfxPass = ComputePass<TonemapVfxCtxWrapper, 0, 2>;

#[derive(Debug)]
pub struct TonemapVfxCtx<'ctx> {
    pub shader: &'ctx ComputeShaderTonemap,
    pub resolution: PixelResolution,
    pub render_params: &'ctx RenderParams,
}

rendrs::context_wrapper!(for<'ctx> TonemapVfxCtx);

pub const fn pass(
    shader: &ComputeShaderTonemap,
    src_hdr: RenderTargetAccessor,
    dst_ldr: RenderTargetAccessor,
) -> TonemapVfxPass {
    let handle_view = shader.compute_handle().view();
    TonemapVfxPass::new(
        handle_view,
        [],
        [
            ImageObjectTarget::new(
                ImageObject::PoolTarget(src_hdr),
                ImageAccessKind::ReadOnly,
                IMAGE_BINDING_SRC,
                None,
            ),
            ImageObjectTarget::new(
                ImageObject::PoolTarget(dst_ldr),
                ImageAccessKind::WriteOnly,
                IMAGE_BINDING_DST,
                None,
            ),
        ],
        |_, ctx| {
            let resolution = ctx.resolution;

            let gamma = ctx.render_params.gamma.as_f32();
            //let exposure = ctx.render_params.exposure...;

            let res = [resolution.width(), resolution.height()];
            ctx.shader.uniform_resolution_uvec2v([res]);
            ctx.shader.uniform_gamma_floatv([gamma]);

            let wg_x = resolution.width().div_ceil(WORKGROUP_SIZE_XY);
            let wg_y = resolution.height().div_ceil(WORKGROUP_SIZE_XY);
            [wg_x, wg_y, 1]
        },
    )
}

pub const IMAGE_BINDING_SRC: u32 = 0;
pub const IMAGE_BINDING_DST: u32 = 1;
pub const WORKGROUP_SIZE_XY: u32 = 8;

ethel::shader_glsl_compute! {
    struct Tonemap > [460] {
        workgroup [8, 8, 1];

        uniform {
            length 1, resolution : uvec2 => [u32; 2];
            length 1, gamma      : float => f32;
        };

        image {
            on IMAGE_BINDING_SRC => src : image2D as rgba16f readonly;
            on IMAGE_BINDING_DST => dst : image2D as rgba8   writeonly;
        };

        lib {
            crate::render::shader_commons::LIB_TONEMAP_ACES_2015;
        };

        src() {
            "
            ivec2 pixel = ivec2(gl_GlobalInvocationID.xy);

            if (pixel.x >= resolution.x || pixel.y >= resolution.y) {
                return;
            }

            float gamma_inv = 1.0 / gamma;

            vec4 C_src = imageLoad(src, pixel);

            // tonemap (ACES approx. Narkowicz 2015)
            vec3 C_map = tonemap_ACES_2015(C_src.rgb);
            // trivial gamma correction
            C_map = pow(C_map, vec3(gamma_inv));

            vec4 C_dst = vec4(C_map, C_src.a);
            imageStore(dst, pixel, C_dst);
            ";
        }
    }
}
