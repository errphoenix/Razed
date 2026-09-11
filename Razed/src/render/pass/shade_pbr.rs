use rendrs::{
    ComputePass,
    graphics::PixelResolution,
    pipeline::{ImageAccessKind, ImageObject, ImageObjectTarget},
};

use crate::render::ViewData;

pub type ShadePbrPass = ComputePass<ShadePbrCtxWrapper, 0, 4>;

pub const fn shade_pbr_pass(
    shader: &ComputeShaderShadePbr,
    raster_in: ImageObject,
    shade_out: ImageObject,
    attr_frame_in: ImageObject,
    attr_grads_in: ImageObject,
) -> ShadePbrPass {
    let handle_view = shader.compute_handle().view();
    let raster_in = ImageObjectTarget::new(
        raster_in,
        ImageAccessKind::ReadOnly,
        IMAGE_BIND_RASTER_IN,
        None,
    );
    let shade_out = ImageObjectTarget::new(
        shade_out,
        ImageAccessKind::WriteOnly,
        IMAGE_BIND_SHADE_OUT,
        None,
    );
    let attr_frame_in = ImageObjectTarget::new(
        attr_frame_in,
        ImageAccessKind::ReadOnly,
        IMAGE_BIND_ATTR_FRAME_IN,
        None,
    );
    let attr_grads_in = ImageObjectTarget::new(
        attr_grads_in,
        ImageAccessKind::ReadOnly,
        IMAGE_BIND_ATTR_GRADS_IN,
        None,
    );
    ShadePbrPass::new(
        handle_view,
        [],
        [raster_in, shade_out, attr_frame_in, attr_grads_in],
        |_, ctx| {
            let ShadePbrCtx {
                shader,
                resolution,
                view_data,
                //dev_mat_page,
                ..
            } = ctx;

            shader.uniform_resolution_uvec2v([[resolution.width(), resolution.height()]]);
            shader.uniform_camera_position_vec3v([view_data.view_pos]);
            shader.uniform_camera_forward_vec3v([view_data.view_dir]);
            //shader.uniform_dev_material_pages_uintv(*dev_mat_page);

            let wg_x = resolution.width().div_ceil(WORKGROUP_SIZE_XY);
            let wg_y = resolution.height().div_ceil(WORKGROUP_SIZE_XY);
            [wg_x, wg_y, 1]
        },
    )
}

#[derive(Debug)]
pub struct ShadePbrCtx<'ctx> {
    pub shader: &'ctx ComputeShaderShadePbr,

    pub resolution: PixelResolution,
    pub view_data: ViewData,

    // 0 = diffuse + alpha
    // 1 = normal + emissive
    // 2 = ormd
    pub dev_mat_page: [u32; 3],
}
rendrs::context_wrapper!(for<'ctx> ShadePbrCtx);

pub const IMAGE_BIND_RASTER_IN: u32 = 10;
pub const IMAGE_BIND_SHADE_OUT: u32 = 11;
pub const IMAGE_BIND_ATTR_FRAME_IN: u32 = 12;
pub const IMAGE_BIND_ATTR_GRADS_IN: u32 = 13;

pub const WORKGROUP_SIZE_XY: u32 = 8;

ethel::shader_glsl_compute! {
    struct ShadePbr > [460] {
        workgroup [8, 8, 1];

        uniform {
            length 1, resolution: uvec2 => [u32; 2];

            length 1, camera_forward: vec3 => glam::Vec3;
            length 1, camera_position: vec3 => glam::Vec3;

            // 0 = diffuse + alpha
            // 1 = normal + emissive
            // 2 = ormd
            length 3, dev_material_pages: uint => u32;
        };
        image {
            on IMAGE_BIND_RASTER_IN => raster_in : uimage2D as rg32ui  readonly;
            on IMAGE_BIND_SHADE_OUT => shade_out : image2D as rgba16f writeonly;

            on IMAGE_BIND_ATTR_FRAME_IN => attr_frame_in : image2D as rgba16  readonly;
            on IMAGE_BIND_ATTR_GRADS_IN => attr_grads_in : image2D as rgba16f readonly;
        };
        lib {
            rendrs::pack::PACK_OCTAHEDRON_DECODE;
            rendrs::geometry::rasterize::LIB_UTIL_FRAMESPACE_GET_NORMAL;
            rendrs::geometry::rasterize::LIB_UTIL_FRAMESPACE_GET_BWEIGHTS;
        };

        src() {
            "
            uvec2 id = gl_GlobalInvocationID.xy;
            if (id.x >= resolution.x || id.y >= resolution.y) {
                return;
            }

            ivec2 px = ivec2(id);
            uvec2 px_data = imageLoad(raster_in, px).rg;

            vec4 outColor = vec4(float(px_data.x), float(px_data.y), 0.0, 1.0);
            imageStore(shade_out, px, outColor);

            ";
        }
    }
}
