use rendrs::{
    ComputePass,
    graphics::PixelResolution,
    pipeline::{ImageAccessKind, ImageObject, ImageObjectTarget},
};

use crate::render::ViewData;

pub type ShadeDebugAttribsPass = ComputePass<ShadeDebugAttribsCtxWrapper, 0, 4>;

pub const fn shade_debug_attribs_pass(
    shader: &ComputeShaderShadeDebugAttribs,
    raster_in: ImageObject,
    shade_out: ImageObject,
    attr_frame_in: ImageObject,
    attr_grads_in: ImageObject,
) -> ShadeDebugAttribsPass {
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
    ShadeDebugAttribsPass::new(
        handle_view,
        [],
        [raster_in, shade_out, attr_frame_in, attr_grads_in],
        |_, ctx| {
            let ShadeDebugAttribsCtx {
                shader,
                resolution,
                view_data,
                mode,
            } = ctx;

            shader.uniform_resolution_uvec2v([[resolution.width(), resolution.height()]]);
            shader.uniform_camera_position_vec3v([view_data.view_pos]);
            shader.uniform_camera_forward_vec3v([view_data.view_dir]);
            shader.uniform_attrib_mode_uintv([mode.clone() as u32]);

            let wg_x = resolution.width().div_ceil(WORKGROUP_SIZE_XY);
            let wg_y = resolution.height().div_ceil(WORKGROUP_SIZE_XY);
            [wg_x, wg_y, 1]
        },
    )
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShadeDebugAttribsMode {
    BarycentricWeights = 0,
    PerspectiveNormals = 1,
    UvScreenDerivatives = 2,
    VisibilityBuffer = 3,
}
impl ShadeDebugAttribsMode {
    pub const fn try_from_id(id: u32) -> Option<Self> {
        match id {
            0 => Some(Self::BarycentricWeights),
            1 => Some(Self::PerspectiveNormals),
            2 => Some(Self::UvScreenDerivatives),
            3 => Some(Self::VisibilityBuffer),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct ShadeDebugAttribsCtx<'ctx> {
    pub shader: &'ctx ComputeShaderShadeDebugAttribs,

    pub resolution: PixelResolution,
    pub view_data: ViewData,

    pub mode: ShadeDebugAttribsMode,
}
rendrs::context_wrapper!(for<'ctx> ShadeDebugAttribsCtx);

pub const IMAGE_BIND_RASTER_IN: u32 = 10;
pub const IMAGE_BIND_SHADE_OUT: u32 = 11;
pub const IMAGE_BIND_ATTR_FRAME_IN: u32 = 12;
pub const IMAGE_BIND_ATTR_GRADS_IN: u32 = 13;

pub const WORKGROUP_SIZE_XY: u32 = 8;

ethel::shader_glsl_compute! {
    struct ShadeDebugAttribs > [460] {
        workgroup [8, 8, 1];

        uniform {
            length 1, resolution: uvec2 => [u32; 2];

            length 1, camera_forward: vec3 => glam::Vec3;
            length 1, camera_position: vec3 => glam::Vec3;

            length 1, attrib_mode: uint => u32;
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

            uvec2 S_raster_in  = imageLoad(raster_in, px).rg;
            vec4  S_gradients  = imageLoad(attr_grads_in, px);
            vec4  S_framespace = imageLoad(attr_frame_in, px);

            vec4 v_output = vec4(1.0);

            switch(attrib_mode) {
                case 0:
                    v_output.rgb = rendrs_FrameSpace_GetBWeights(S_framespace);
                    break;
                case 1:
                    v_output.rgb = rendrs_FrameSpace_GetNormal(S_framespace);
                    break;
                case 2:
                    v_output = S_gradients;
                    break;
                case 3:
                    v_output.rg = vec2(
                      float(S_raster_in.x) / 35000.0,
                      float(S_raster_in.y) / 1500.0
                    );
                    v_output.b = 0.25;
                    break;
            }

            imageStore(shade_out, px, v_output);
            ";
        }
    }
}
