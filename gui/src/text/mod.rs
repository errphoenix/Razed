use std::{collections::HashMap, num::NonZeroUsize, vec::Drain};

use cosmic_text::{
    Align, Attrs, Buffer, CacheKey, Family, FontSystem, Metrics, Shaping, SwashCache,
};
use etagere::{AllocId, Allocation, AtlasAllocator};
use lru::LruCache;

use crate::{draw::QuadElement, text::font::Font};

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

const DEFAULT_ATLAS_SIZE: i32 = 1024;
const DEFAULT_LRU_SIZE: usize = 256;

pub struct GlyphAtlas {
    packer: AtlasAllocator,
    swash_cache: SwashCache,
    uv_cache: HashMap<CacheKey, GlyphUv, rustc_hash::FxBuildHasher>,
    atlas_lru: LruCache<CacheKey, AllocId>,
    size: u32,
}
impl Default for GlyphAtlas {
    fn default() -> Self {
        Self {
            packer: AtlasAllocator::new(etagere::size2(DEFAULT_ATLAS_SIZE, DEFAULT_ATLAS_SIZE)),
            swash_cache: SwashCache::new(),
            uv_cache: Default::default(),
            atlas_lru: LruCache::new(NonZeroUsize::new(DEFAULT_LRU_SIZE).unwrap()),
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
            atlas_lru: LruCache::new(NonZeroUsize::new(size as usize / 4).unwrap()),
            size,
        }
    }

    fn evict_till_atlas_free(&mut self, key: &CacheKey, width: i32, height: i32) -> Allocation {
        if let Some(alloc) = self.packer.allocate(etagere::size2(width, height)) {
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
    out_buffer: Vec<QuadElement>,
}
impl<'a> TextComposer<'a> {
    pub fn new(metrics: Metrics, font_system: FontSystem) -> Self {
        Self {
            buffer: Buffer::new_empty(metrics),
            attribs: Attrs::new(),
            alignment: Align::Left,
            out_buffer: Vec::new(),
            font_system,
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

    pub fn compose(&mut self, glyph_atlas: &mut GlyphAtlas) -> Drain<'_, QuadElement> {
        self.buffer.shape_until_scroll(&mut self.font_system, false);
        self.buffer.layout_runs().for_each(|run| {
            run.glyphs.iter().for_each(|glyph| {
                let glyph = glyph.physical((0., 0.), 1.0);
            });
        });
        self.out_buffer.drain(..)
    }
}
