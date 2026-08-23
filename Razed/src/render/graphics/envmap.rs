use ethel::{
    assets::{AssetRegistry, Handle, RawTexture, TextureMetadata},
    render::buffer::StorageSection,
};
use janus::{
    StringHash,
    texture::{ImageFormat, ImageType, MipLevels, Tex, Texture, TextureKind, TextureView},
};
use rendrs::{
    graphics::reflection_filtering::{
        BSplineDownscaleCtx, ComputeShaderBSplineDownscale, ComputeShaderPrefilterCubemap,
        FILTERING_MIP_COUNT, PrefilterCubemapCtx,
    },
    pipeline::{ImageAccessKind, ImageObject, ImageObjectTarget, Pass, RenderPool},
};

use crate::render::pass::{
    ComputeShaderEquirectDecode, EquirectDecodeCtx, equirect_decode_compute,
};

pub const ENVMAP_MIPS: i32 = FILTERING_MIP_COUNT as i32;
pub const ENVMAP_RESOLUTION: i32 = 256;

type TextureRegistry = AssetRegistry<RawTexture, TextureMetadata>;

pub fn load_environment_map(texture_assets: &mut TextureRegistry) -> TextureView {
    const DEV_ENV_NAME: &str = crate::assets::ENVMAP_EQUIRECT_ENV_NAME_CITRUS_ORCHARD;
    const DEV_ENV_ID: StringHash = janus::hash_string(DEV_ENV_NAME);

    let equirect_tex = {
        let raw = {
            let handle = texture_assets.get_mut(DEV_ENV_ID).unwrap();
            handle.load_to_memory(&()).unwrap();
            let raw = handle.take_from_memory().unwrap();
            let rgba = image::DynamicImage::ImageRgba32F(raw.0.into_rgba32f());
            RawTexture::new(rgba)
        };
        Texture::from_2d_image(raw.image(), MipLevels::default())
    }
    .unwrap();

    let cubemap_tex = Texture::new_cubemap(
        ENVMAP_RESOLUTION,
        ENVMAP_RESOLUTION,
        MipLevels::try_new(ENVMAP_MIPS).unwrap(),
        ImageType::Float16,
        ImageFormat::Rgba,
    );

    let shader = ComputeShaderEquirectDecode::new_compiled();
    let decode_pass = equirect_decode_compute::pass(&shader);
    let decode_ctx = EquirectDecodeCtx {
        shader: &shader,
        src_equirect: ImageObjectTarget::new(
            ImageObject::from_direct_texture(equirect_tex.view()),
            ImageAccessKind::ReadOnly,
            equirect_decode_compute::IMAGE_BINDING_SRC_EQUIRECT,
            None,
        ),
        dst_cubemap: ImageObjectTarget::new(
            ImageObject::from_direct_texture(cubemap_tex.view()),
            ImageAccessKind::WriteOnly,
            equirect_decode_compute::IMAGE_BINDING_DST_CUBEMAP,
            None,
        ),
    };
    decode_pass.execute(StorageSection::Spare, &RenderPool::dummy(), &decode_ctx);
    janus::gl::barrier_shader_image();
    cubemap_tex.generate_mipmaps();

    texture_assets.add_handle(Handle::from_gpu_resource(
        DEV_ENV_ID,
        cubemap_tex,
        texture_assets,
    ));
    texture_assets.get_gpu_view(DEV_ENV_ID).unwrap()
}

pub fn bake_brdf_specular() -> Texture {
    const RESOLUTION: i32 = 256;
    let tex = Texture::new_2d(
        RESOLUTION,
        RESOLUTION,
        MipLevels::default(),
        ImageType::Float16,
        ImageFormat::DualChannel,
    );
    rendrs::graphics::brdf_bake_specular(Default::default(), tex.view());
    janus::gl::barrier_shader_image();
    tex
}

/// Writes mips to `texture` (so it must allocate for atleast
/// [`FILTERING_MIP_COUNT`] mip levels) and writes the filtered reflection
/// map to a newly allocated texture.
///
/// [`FILTERING_MIP_COUNT`]: rendrs::graphics::reflection_filtering::FILTERING_MIP_COUNT
///
/// Returns the prefiltered reflection cubemap.
pub fn debug_probe_reflection(texture: TextureView) -> Texture {
    debug_assert_eq!(texture.target_kind(), TextureKind::CubeMap);

    let shader = ComputeShaderBSplineDownscale::new_compiled();
    let pass = rendrs::graphics::rf_bspline_downsample(&shader);

    for i in 1..ENVMAP_MIPS {
        pass.execute(
            StorageSection::Back,
            &RenderPool::dummy(),
            &BSplineDownscaleCtx {
                target: texture,
                mip_level: MipLevels::try_new(i).unwrap(),
            },
        );
        janus::gl::barrier_shader_image();
    }

    let prefiltered = Texture::new_cubemap(
        ENVMAP_RESOLUTION,
        ENVMAP_RESOLUTION,
        MipLevels::try_new(ENVMAP_MIPS).unwrap(),
        ImageType::Float16,
        ImageFormat::Rgba,
    );
    let shader = &ComputeShaderPrefilterCubemap::new_compiled();
    let pass = rendrs::graphics::rf_prefilter_cubemap(&shader, texture, prefiltered.view());
    pass.execute(
        StorageSection::Back,
        &RenderPool::dummy(),
        &PrefilterCubemapCtx::new(ENVMAP_RESOLUTION as u32),
    );
    janus::gl::barrier_shader_image();

    prefiltered
}
