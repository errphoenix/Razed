use const_format::concatcp;
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

        ENVMAP_CUBE_ENV_NAME_LARNACACASTLE_NEGX => "assets/dev/env/larnaca_castle/negx.jpg";
        ENVMAP_CUBE_ENV_NAME_LARNACACASTLE_NEGY => "assets/dev/env/larnaca_castle/negy.jpg";
        ENVMAP_CUBE_ENV_NAME_LARNACACASTLE_NEGZ => "assets/dev/env/larnaca_castle/negz.jpg";
        ENVMAP_CUBE_ENV_NAME_LARNACACASTLE_POSX => "assets/dev/env/larnaca_castle/posx.jpg";
        ENVMAP_CUBE_ENV_NAME_LARNACACASTLE_POSY => "assets/dev/env/larnaca_castle/posy.jpg";
        ENVMAP_CUBE_ENV_NAME_LARNACACASTLE_POSZ => "assets/dev/env/larnaca_castle/posz.jpg";
    }
}

pub const ENVMAP_CUBE_ENV_NAME_LARNACACASTLE: &str = "__dev.env.cube.larnaca-castle";
pub const ENVMAP_CUBE_ENV_NAME_LARNACACASTLE_NEGX: &str =
    concatcp!(ENVMAP_CUBE_ENV_NAME_LARNACACASTLE, ".negx");
pub const ENVMAP_CUBE_ENV_NAME_LARNACACASTLE_NEGY: &str =
    concatcp!(ENVMAP_CUBE_ENV_NAME_LARNACACASTLE, ".negy");
pub const ENVMAP_CUBE_ENV_NAME_LARNACACASTLE_NEGZ: &str =
    concatcp!(ENVMAP_CUBE_ENV_NAME_LARNACACASTLE, ".negz");
pub const ENVMAP_CUBE_ENV_NAME_LARNACACASTLE_POSX: &str =
    concatcp!(ENVMAP_CUBE_ENV_NAME_LARNACACASTLE, ".posz");
pub const ENVMAP_CUBE_ENV_NAME_LARNACACASTLE_POSY: &str =
    concatcp!(ENVMAP_CUBE_ENV_NAME_LARNACACASTLE, ".posy");
pub const ENVMAP_CUBE_ENV_NAME_LARNACACASTLE_POSZ: &str =
    concatcp!(ENVMAP_CUBE_ENV_NAME_LARNACACASTLE, ".posz");
