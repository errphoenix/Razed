use ethel::assets::TextureMetadata;

pub type Texture = ethel::assets::RawTexture;

ethel::asset_registry! {
    struct Texture: TextureMetadata {

    }
}
