use ethel::{
    assets::{AssetRegistry, Handle, RawTexture, TextureMetadata},
    render::buffer::StorageSection,
};
use janus::{
    StringHash,
    texture::{ImageFormat, ImageType, MipLevels, Tex, Texture, TextureKind, TextureView},
};
use rendrs::{
    graphics::{
        ImageTargetFormat, ShCoeffsBuffer,
        brdf_bake_specular::ComputeShaderBrdfBakingSpecular,
        irradiance_harmonics::{ComputeShaderIrradianceHarmonics, IrradianceHarmonicsCtx},
        reflection_filtering::{
            BSplineDownscaleCtx, ComputeShaderBSplineDownscale, ComputeShaderPrefilterCubemap,
            FILTERING_MIP_COUNT, PrefilterCubemapCtx,
        },
    },
    pipeline::{ImageAccessKind, ImageObject, ImageObjectTarget, Pass, RenderPool, SamplerObject},
};

use crate::render::pass::{
    ComputeShaderEquirectDecode, EquirectDecodeCtx, equirect_decode_compute,
};

pub const ENVMAP_MIPS: i32 = FILTERING_MIP_COUNT as i32;
pub const ENVMAP_RESOLUTION: i32 = 128;
pub const TRUENV_CUBEMAP_RES: i32 = 1024;

type TextureRegistry = AssetRegistry<RawTexture, TextureMetadata>;

/// Returns the full-resolution cubemap environment view, and a new texture
/// of a downscaled environment cubemap that matches the necessary probe
/// resolution for a reflection cubemap.
pub fn load_environment_map(texture_assets: &mut TextureRegistry) -> (TextureView, Texture) {
    const DEV_ENV_NAME: &str = crate::assets::ENVMAP_NAME_759_HDRI_SKIES_COM;
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

    // find mip of source env. map that matches (debug) probe resolution
    let mips_to_match = {
        let mut res = TRUENV_CUBEMAP_RES;
        let mut i = 1;
        while res > ENVMAP_RESOLUTION {
            res >>= 1;
            i += 1;
        }
        i
    };

    let cubemap_tex = Texture::new_cubemap(
        TRUENV_CUBEMAP_RES,
        TRUENV_CUBEMAP_RES,
        MipLevels::try_new(mips_to_match).unwrap_or_default(),
        ImageType::Float16,
        ImageFormat::Rgba,
    );

    {
        // equirectangular to cubemap conversion
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
    }
    let fullres = texture_assets.get_gpu_view(DEV_ENV_ID).unwrap();

    // downscale for reflection cubemap
    let downscaled = Texture::new_cubemap(
        ENVMAP_RESOLUTION,
        ENVMAP_RESOLUTION,
        MipLevels::try_new(ENVMAP_MIPS).unwrap(),
        ImageType::Float16,
        ImageFormat::Rgba,
    );
    for i in 0..6 {
        rendrs::graphics::image_blit(
            ImageTargetFormat::Rgba16f,
            ImageObject::DirectTexture(fullres),
            ImageObject::DirectTexture(downscaled.view()),
            Some(mips_to_match - 1),
            None,
            Some(i),
            Some(i),
        );
    }
    janus::gl::barrier_shader_image();

    (fullres, downscaled)
}

pub fn bake_brdf_specular() -> Texture {
    let shader = ComputeShaderBrdfBakingSpecular::new_compiled();

    const RESOLUTION: i32 = 256;
    let tex = Texture::new_2d(
        RESOLUTION,
        RESOLUTION,
        MipLevels::default(),
        ImageType::Float16,
        ImageFormat::DualChannel,
    );

    let elapsed = janus::gl::synchronous(|| {
        rendrs::graphics::brdf_bake_specular(&shader, tex.view());
    });

    println!("env. spec. brdf bake time: {elapsed} nanos");
    tex
}

/// Sources radiance from `texture` to build an irradiance map with
/// spherical harmonics on the given ssbo.
///
/// `texture` must contain the environment map and it (or any of its mips) must
/// be of 16x16 resolution. The texture on which mips were created by
/// [`debug_probe_reflection`] through [`BSplineDownscalePass`] is perfect
/// for this.
///
/// [`BSplineDownscalePass`]: rendrs::graphics::passes::reflection_filtering::BSplineDownscalePass
pub fn debug_irradiance(texture: TextureView, output: &ShCoeffsBuffer) {
    debug_assert_eq!(texture.target_kind(), TextureKind::CubeMap);

    let shader = ComputeShaderIrradianceHarmonics::new_compiled();

    let elapsed = janus::gl::synchronous(|| {
        rendrs::graphics::irradiance_harmonics(&shader, SamplerObject::new(texture)).execute(
            StorageSection::Back,
            &RenderPool::dummy(),
            &IrradianceHarmonicsCtx {
                output_coefficients: output,
            },
        );
    });

    println!("irradiance sh time: {elapsed} nanos");
}

/// Writes mips to `texture` (so the caller must allocate for atleast
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

    let elapsed_0 = janus::gl::synchronous(|| {
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
    });
    println!("downscale time: {elapsed_0} nanos");

    let prefiltered = Texture::new_cubemap(
        ENVMAP_RESOLUTION,
        ENVMAP_RESOLUTION,
        MipLevels::try_new(ENVMAP_MIPS).unwrap(),
        ImageType::Float16,
        ImageFormat::Rgba,
    );
    let shader = &ComputeShaderPrefilterCubemap::new_compiled();
    let pass = rendrs::graphics::rf_prefilter_cubemap(&shader, texture, prefiltered.view());

    let elapsed_1 = janus::gl::synchronous(|| {
        pass.execute(
            StorageSection::Back,
            &RenderPool::dummy(),
            &PrefilterCubemapCtx::new(ENVMAP_RESOLUTION as u32),
        );
    });

    println!("prefilter time: {elapsed_1} nanos");
    println!("total time: {} nanos", elapsed_0 + elapsed_1);

    prefiltered
}
