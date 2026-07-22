use std::{collections::HashMap, vec::Drain};

use cosmic_text::{
    Align, Attrs, Buffer, CacheKey, Family, FontSystem, Metrics, Shaping, SwashCache,
};
use etagere::{AllocId, Allocation, AtlasAllocator};
use ethel::assets::TextureId;
use janus::texture::{
    ImageFormat, ImageType, MipLevels, Tex, Texture, TextureFiltering, TextureView,
};
use lru::LruCache;

use crate::draw::{InterfaceAttachment, InterfaceObject};

pub mod font;

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct FontMetrics {
    pub font_size: f32,
    pub line_height: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct TextMeasurement {
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct GlyphInfo {
    pub uv: GlyphUv,
    pub width: u32,
    pub height: u32,
    pub left: i32,
    pub top: i32,
}
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

#[derive(Debug, Default, Clone, Copy)]
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

        let texture = Texture::new_2d(
            size,
            size,
            MipLevels::default(),
            ImageType::Bits8,
            ImageFormat::Rgba,
        );
        texture.set_filtering_minmag(TextureFiltering::Nearest);
        self.view = Some(texture.view());

        texture
    }

    pub const fn invalidate(&mut self) {
        self.view = None;
    }

    /// Copy a rasterized `glyph` to the atlas [`Texture`].
    ///
    /// The atlas texture must have been initialized previously with
    /// [`Self::create_atlas_texture`].
    ///
    /// This function must be called on the OpenGL thread.
    ///
    /// # Panics
    /// If the atlas texture is not initialized, or if no OpenGL context is
    /// currently available on the caller thread.
    pub fn copy_glyph(&self, glyph: GlyphRaster) {
        janus::assert_gl!();

        let texture = self.view.expect("atlas texture uninitialized");
        texture
            .upload_2d(
                0,
                glyph.offset_x as i32,
                glyph.offset_y as i32,
                glyph.size_x as i32,
                glyph.size_y as i32,
                &glyph.data,
            )
            .expect("glyph atlas is always a 2d texture");
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
    uv_cache: HashMap<CacheKey, GlyphInfo, rustc_hash::FxBuildHasher>,
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
    ) -> Option<(GlyphInfo, Option<GlyphRaster>)> {
        if let Some(&uv) = self.uv_cache.get(&key) {
            self.atlas_lru.promote(&key);
            return Some((uv, None));
        }

        let img = self.swash_cache.get_image_uncached(font_sys, key)?;
        let width = img.placement.width;
        let height = img.placement.height;
        if width == 0 || height == 0 {
            return None;
        }
        let left = img.placement.left;
        let top = img.placement.top;

        let allocation = self.evict_till_atlas_free(&key, width as i32, height as i32);
        let rect = allocation.rectangle;

        let min_x = rect.min.x as f32;
        let min_y = rect.min.y as f32;
        let glyph_info = GlyphInfo {
            uv: GlyphUv {
                ux: min_x / self.size as f32,
                uy: min_y / self.size as f32,
                vx: (min_x + width as f32) / self.size as f32,
                vy: (min_y + height as f32) / self.size as f32,
            },
            width,
            height,
            left,
            top,
        };
        self.uv_cache.insert(key, glyph_info);

        let mut data = Vec::new();
        match img.content {
            // unpack single-channel to rgba
            cosmic_text::SwashContent::Mask => {
                data.reserve_exact(img.data.len() * 4);
                for byte in img.data {
                    data.push(255u8);
                    data.push(255u8);
                    data.push(255u8);
                    data.push(byte);
                }
            }
            // no op
            cosmic_text::SwashContent::SubpixelMask | cosmic_text::SwashContent::Color => {
                data = img.data
            }
        }

        let raster = GlyphRaster {
            offset_x: rect.min.x as u32,
            offset_y: rect.min.y as u32,
            size_x: width as u32,
            size_y: height as u32,
            data,
        };
        Some((glyph_info, Some(raster)))
    }
}

#[derive(Debug)]
pub struct TextComposer {
    buffer: cosmic_text::Buffer,
    font_system: Option<FontSystem>,

    attribs: Attrs<'static>,
    alignment: Align,

    element_buffer: Vec<InterfaceObject>,
    raster_buffer: Vec<GlyphRaster>,
}
impl TextComposer {
    pub fn new() -> Self {
        const METRICS: Metrics = Metrics::new(1.0, 1.0);
        Self {
            buffer: Buffer::new_empty(METRICS),
            font_system: None,
            attribs: Attrs::new(),
            alignment: Align::Left,
            element_buffer: Vec::new(),
            raster_buffer: Vec::new(),
        }
    }

    pub fn set_font_metrics(&mut self, metrics: FontMetrics) {
        let font_sys = self.font_system.as_mut().expect("font_system is not set");
        self.buffer.set_metrics(
            font_sys,
            Metrics {
                font_size: metrics.font_size,
                line_height: metrics.line_height,
            },
        );
    }

    pub fn set_font_system(&mut self, font_system: FontSystem) {
        self.font_system = Some(font_system);
    }

    /// Set the size of the buffer for text layouting.
    pub fn set_buffer_size(&mut self, width_opt: Option<f32>, height_opt: Option<f32>) {
        let font_sys = self.font_system.as_mut().expect("font_system is not set");
        self.buffer.set_size(font_sys, width_opt, height_opt);
    }

    pub fn attributes(&self) -> &Attrs<'static> {
        &self.attribs
    }

    pub fn attributes_mut(&mut self) -> &mut Attrs<'static> {
        &mut self.attribs
    }

    pub fn set_alignment(&mut self, alignment: crate::ItemAlignment) {
        self.alignment = match alignment {
            crate::ItemAlignment::Start => Align::Left,
            crate::ItemAlignment::End => Align::Right,
            crate::ItemAlignment::Stretch => Align::Justified,
            _ => Align::Center,
        };
    }

    pub fn set_font(&mut self, font_family: &'static str) {
        self.attribs.family = Family::Name(font_family);
    }

    pub fn set_text(&mut self, string: &str) {
        let font_sys = self.font_system.as_mut().expect("font_system is not set");
        self.buffer.set_text(
            font_sys,
            string,
            &self.attribs,
            Shaping::Advanced,
            Some(self.alignment),
        );
    }

    pub fn measure(&mut self) -> TextMeasurement {
        let width = self
            .buffer
            .layout_runs()
            .fold(0.0f32, |v, run| v.max(run.line_w));

        let mut height = 0.0f32;
        if let Some(last) = self.buffer.layout_runs().last() {
            height = last.line_top + last.line_height;
        }

        TextMeasurement { width, height }
    }

    pub fn compose(
        &mut self,
        x_offset: f32,
        y_offset: f32,
        layer: u32,
        glyph_atlas: &mut GlyphAtlas,
    ) {
        let font_sys = self.font_system.as_mut().expect("font_system is not set");
        self.buffer.layout_runs().for_each(|run| {
            let baseline_y = y_offset + run.line_y;
            run.glyphs.iter().for_each(|glyph| {
                let glyph = glyph.physical((x_offset, baseline_y), 1.0);
                if let Some((info, raster)) =
                    glyph_atlas.get_or_rasterize(font_sys, glyph.cache_key)
                {
                    if let Some(raster) = raster {
                        self.raster_buffer.push(raster);
                    }
                    if info.width == 0 || info.height == 0 {
                        return;
                    }

                    let x = glyph.x as f32 + info.left as f32;
                    let y = glyph.y as f32 - info.top as f32;
                    let width = info.width as f32;
                    let height = info.height as f32;
                    let uv = info.uv;

                    self.element_buffer.push(InterfaceObject {
                        position: glam::vec2(x, y),
                        size: glam::vec2(width, height),
                        color: glam::Vec4::ONE,
                        attachment: Some(InterfaceAttachment::TextureSection {
                            texture_id: GlyphAtlasTexture::resource_id(),
                            uv: [uv.ux, uv.uy, uv.vx, uv.vy],
                        }),
                        layer,
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
