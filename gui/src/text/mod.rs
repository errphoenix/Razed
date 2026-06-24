use std::{collections::HashMap, vec::Drain};

use cosmic_text::{
    Align, Attrs, Buffer, CacheKey, Family, FontSystem, Metrics, Shaping, SwashCache,
};
use etagere::{AllocId, Allocation, AtlasAllocator};
use ethel::assets::TextureId;
use janus::texture::{ImageFormat, ImageType, Texture, TextureView};
use lru::LruCache;

use crate::{
    draw::{InterfaceAttachment, InterfaceObject},
    text::font::Font,
};

pub mod font;

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct GlyphUv {
    pub ux: f32,
    pub uy: f32,
    pub vx: f32,
    pub vy: f32,
}

#[derive(Clone, Debug)]
pub struct GlyphRaster {
    pub offset_x: u32,
    pub offset_y: u32,
    pub size_x: u32,
    pub size_y: u32,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct GlyphAtlasTexture {
    view: Option<TextureView>,
}
impl GlyphAtlasTexture {
    ethel::hashet! {
        const TEXTURE_ID_HASHET = "i___GLYPH_ATLAS";
    }

    pub fn resource_id() -> TextureId {
        TextureId(*Self::TEXTURE_ID_HASHET)
    }

    /// Allocate a new texture onto the GPU to be used as the atlas texture.
    ///
    /// This function must be called on the OpenGL thread.
    ///
    /// The [`GlyphAtlasTexture`] only keeps a [`TextureView`] in order to
    /// still be able to access the texture resource on the GPU.
    ///
    /// Do note that [`TextureView`] is unaware of the actual [`Texture`]
    /// resource state, which may cause errors when access to the
    /// [`Texture`] occur after this is no longer available.
    /// To avoid this, ensure [`Self::invalidate`] is called when the
    /// [`Texture`] resource is destroyed.
    ///
    /// The caller is responsible for the lifecycle of the returned
    /// [`Texture`].
    ///
    /// # Panics
    /// If no OpenGL context is currently available on the caller thread.
    pub fn create_atlas_texture(&mut self, size: i32) -> Texture {
        janus::assert_gl!();

        let bytes = vec![0u8; (size * size) as usize];

        let texture = Texture::from_bytes(
            size,
            size,
            &bytes,
            ImageType::Bits8,
            ImageFormat::SingleChannel,
        );
        self.view = Some(texture.view());
        texture
    }

    pub const fn invalidate(&mut self) {
        self.view = None;
    }

    /// Copy a sequence of `glyphs` to the atlas [`Texture`].
    ///
    /// The atlas texture must have been initialized previously with
    /// [`Self::create_atlas_texture`].
    ///
    /// This function must be called on the OpenGL thread.
    ///
    /// # Panics
    /// If the atlas texture is not initialized, or if no OpenGL context is
    /// currently available on the caller thread.
    pub fn copy_glyphs(&self, glyphs: &[GlyphRaster]) {
        janus::assert_gl!();

        let texture = self.view.expect("atlas texture uninitialized");
        for glyph in glyphs {
            texture.upload_region(
                glyph.offset_x as i32,
                glyph.offset_y as i32,
                glyph.size_x as i32,
                glyph.size_y as i32,
                &glyph.data,
            );
        }
    }

    pub const fn texture(&self) -> Option<TextureView> {
        self.view
    }
}

const DEFAULT_ATLAS_SIZE: i32 = 1024;

pub struct GlyphAtlas {
    swash_cache: SwashCache,
    packer: AtlasAllocator,
    size: u32,

    atlas_lru: LruCache<CacheKey, AllocId, rustc_hash::FxBuildHasher>,
    uv_cache: HashMap<CacheKey, GlyphUv, rustc_hash::FxBuildHasher>,
}
impl Default for GlyphAtlas {
    fn default() -> Self {
        Self {
            packer: AtlasAllocator::new(etagere::size2(DEFAULT_ATLAS_SIZE, DEFAULT_ATLAS_SIZE)),
            swash_cache: SwashCache::new(),
            uv_cache: Default::default(),
            atlas_lru: LruCache::unbounded_with_hasher(rustc_hash::FxBuildHasher::default()),
            size: Default::default(),
        }
    }
}
impl std::fmt::Debug for GlyphAtlas {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlyphAtlas")
            .field("packer", &"???")
            .field("swash_cache", &self.swash_cache)
            .field("uv_cache", &self.uv_cache)
            .field("atlas_lru", &self.atlas_lru)
            .field("size", &self.size)
            .finish()
    }
}
impl GlyphAtlas {
    pub fn new(size: u32) -> Self {
        Self {
            swash_cache: SwashCache::new(),
            packer: AtlasAllocator::new(etagere::size2(size as i32, size as i32)),
            uv_cache: HashMap::default(),
            atlas_lru: LruCache::unbounded_with_hasher(rustc_hash::FxBuildHasher::default()),
            size,
        }
    }

    pub const fn size(&self) -> u32 {
        self.size
    }

    fn evict_till_atlas_free(&mut self, key: &CacheKey, width: i32, height: i32) -> Allocation {
        if let Some(alloc) = self.packer.allocate(etagere::size2(width, height)) {
            self.atlas_lru.push(*key, alloc.id);
            alloc
        } else {
            if let Some((evict_key, evict_alloc)) = self.atlas_lru.pop_lru() {
                self.uv_cache.remove(&evict_key);
                self.packer.deallocate(evict_alloc);
                self.evict_till_atlas_free(key, width, height)
            } else {
                panic!("glyph LRU is empty but no free section found");
            }
        }
    }

    pub fn get_or_rasterize(
        &mut self,
        font_sys: &mut FontSystem,
        key: CacheKey,
    ) -> Option<(GlyphUv, Option<GlyphRaster>)> {
        if let Some(&uv) = self.uv_cache.get(&key) {
            return Some((uv, None));
        }

        let img = self.swash_cache.get_image_uncached(font_sys, key)?;
        let width = img.placement.width as i32;
        let height = img.placement.height as i32;
        if width == 0 || height == 0 {
            return None;
        }

        let allocation = self.evict_till_atlas_free(&key, width, height);
        let rect = allocation.rectangle;
        let uv_bounds = GlyphUv {
            ux: rect.min.x as f32 / self.size as f32,
            uy: rect.min.y as f32 / self.size as f32,
            vx: rect.max.x as f32 / self.size as f32,
            vy: rect.max.y as f32 / self.size as f32,
        };
        self.uv_cache.insert(key, uv_bounds);

        let raster = GlyphRaster {
            offset_x: rect.min.x as u32,
            offset_y: rect.min.y as u32,
            size_x: rect.width() as u32,
            size_y: rect.height() as u32,
            data: img.data,
        };
        Some((uv_bounds, Some(raster)))
    }
}

#[derive(Debug)]
pub struct TextComposer<'a> {
    buffer: cosmic_text::Buffer,
    font_system: FontSystem,

    attribs: Attrs<'a>,
    alignment: Align,

    element_buffer: Vec<InterfaceObject>,
    raster_buffer: Vec<GlyphRaster>,
}
impl<'a> TextComposer<'a> {
    pub fn new(metrics: Metrics, font_system: FontSystem) -> Self {
        Self {
            buffer: Buffer::new_empty(metrics),
            font_system,
            attribs: Attrs::new(),
            alignment: Align::Left,
            element_buffer: Vec::new(),
            raster_buffer: Vec::new(),
        }
    }

    /// Set the size of the buffer for text layouting.
    pub fn set_buffer_size(&mut self, width_opt: Option<f32>, height_opt: Option<f32>) {
        self.buffer
            .set_size(&mut self.font_system, width_opt, height_opt);
    }

    pub fn attributes(&self) -> &Attrs<'a> {
        &self.attribs
    }

    pub fn attributes_mut(&mut self) -> &mut Attrs<'a> {
        &mut self.attribs
    }

    pub fn set_alignment(&mut self, alignment: crate::ItemAlignment) {
        self.alignment = match alignment {
            crate::ItemAlignment::Center | crate::ItemAlignment::Auto => Align::Center,
            crate::ItemAlignment::Start => Align::Left,
            crate::ItemAlignment::End => Align::Right,
        };
    }

    pub fn set_font(&mut self, font: &'a Font) {
        self.attribs.family = Family::Name(&font.family);
    }

    pub fn set_text(&mut self, string: &str) {
        self.buffer.set_text(
            &mut self.font_system,
            string,
            &self.attribs,
            Shaping::Advanced,
            Some(self.alignment),
        );
    }

    pub fn compose(&mut self, glyph_atlas: &mut GlyphAtlas) {
        self.buffer.lines.clear();
        self.buffer.shape_until_scroll(&mut self.font_system, false);
        self.buffer.layout_runs().for_each(|run| {
            run.glyphs.iter().for_each(|glyph| {
                let glyph = glyph.physical((0., 0.), 1.0);
                if let Some((uv, raster)) =
                    glyph_atlas.get_or_rasterize(&mut self.font_system, glyph.cache_key)
                {
                    if let Some(raster) = raster {
                        self.raster_buffer.push(raster);
                    }

                    self.element_buffer.push(InterfaceObject {
                        position: Default::default(),
                        size: Default::default(),
                        color: glam::Vec4::ZERO,
                        attachment: Some(InterfaceAttachment::TextureSection {
                            texture_id: GlyphAtlasTexture::resource_id(),
                            uv: [uv.ux, uv.uy, uv.vx, uv.vy],
                        }),
                    });
                }
            });
        });
    }

    pub fn elements(&mut self) -> Drain<'_, InterfaceObject> {
        self.element_buffer.drain(..)
    }

    pub fn rasters(&mut self) -> Drain<'_, GlyphRaster> {
        self.raster_buffer.drain(..)
    }
}
