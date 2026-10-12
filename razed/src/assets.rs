use ethel::assets::TextureMetadata;

pub type Texture = ethel::assets::RawTexture;

pub use crate::render::graphics::materials::*;

ethel::asset_registry! {
    struct Texture: TextureMetadata {
        MATEX_DEV_NAME_BRICKS103_DIFFUSE      => "assets/dev/bricks103/diffuse.jpg";
        MATEX_DEV_NAME_BRICKS103_NORMAL       => "assets/dev/bricks103/normal.jpg";
        MATEX_DEV_NAME_BRICKS103_OCCLUSION    => "assets/dev/bricks103/occlusion.jpg";
        MATEX_DEV_NAME_BRICKS103_ROUGHNESS    => "assets/dev/bricks103/roughness.jpg";
        MATEX_DEV_NAME_BRICKS103_DISPLACEMENT => "assets/dev/bricks103/displacement.jpg";

        MATEX_DEV_NAME_CONCRETE012_DIFFUSE      => "assets/dev/concrete012/diffuse.jpg";
        MATEX_DEV_NAME_CONCRETE012_NORMAL       => "assets/dev/concrete012/normal.jpg";
        MATEX_DEV_NAME_CONCRETE012_ROUGHNESS    => "assets/dev/concrete012/roughness.jpg";
        MATEX_DEV_NAME_CONCRETE012_DISPLACEMENT => "assets/dev/concrete012/displacement.jpg";

        MATEX_DEV_NAME_ROAD015C_DIFFUSE      => "assets/dev/road015c/diffuse.jpg";
        MATEX_DEV_NAME_ROAD015C_NORMAL       => "assets/dev/road015c/normal.jpg";
        MATEX_DEV_NAME_ROAD015C_OCCLUSION    => "assets/dev/road015c/occlusion.jpg";
        MATEX_DEV_NAME_ROAD015C_ROUGHNESS    => "assets/dev/road015c/roughness.jpg";
        MATEX_DEV_NAME_ROAD015C_DISPLACEMENT => "assets/dev/road015c/displacement.jpg";

        MATEX_DEV_NAME_METAL048C_DIFFUSE      => "assets/dev/metal048c/diffuse.jpg";
        MATEX_DEV_NAME_METAL048C_NORMAL       => "assets/dev/metal048c/normal.jpg";
        MATEX_DEV_NAME_METAL048C_METALLIC     => "assets/dev/metal048c/metallic.jpg";
        MATEX_DEV_NAME_METAL048C_ROUGHNESS    => "assets/dev/metal048c/roughness.jpg";
        MATEX_DEV_NAME_METAL048C_DISPLACEMENT => "assets/dev/metal048c/displacement.jpg";

        MATEX_DEV_NAME_METAL063_DIFFUSE      => "assets/dev/metal063/diffuse.jpg";
        MATEX_DEV_NAME_METAL063_NORMAL       => "assets/dev/metal063/normal.jpg";
        MATEX_DEV_NAME_METAL063_METALLIC     => "assets/dev/metal063/metallic.jpg";
        MATEX_DEV_NAME_METAL063_ROUGHNESS    => "assets/dev/metal063/roughness.jpg";
        MATEX_DEV_NAME_METAL063_DISPLACEMENT => "assets/dev/metal063/displacement.jpg";

        ENVMAP_NAME_CITRUS_ORCHARD => "assets/dev/env/citrus_orchard_puresky.hdr";
        ENVMAP_NAME_759_HDRI_SKIES_COM => "assets/dev/env/759-hdri-skies-com.hdr";
    }
}

pub const ENVMAP_NAME_CITRUS_ORCHARD: &str = "__dev.env.citrus-orchard";
pub const ENVMAP_NAME_759_HDRI_SKIES_COM: &str = "__dev.env.759-hdri-skies.com";
