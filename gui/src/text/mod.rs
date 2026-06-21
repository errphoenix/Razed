use std::{collections::HashMap, num::NonZeroUsize};

use cosmic_text::{
    Align, Attrs, Buffer, CacheKey, Family, FontSystem, Metrics, Shaping, SwashCache,
};
use etagere::{AllocId, Allocation, AtlasAllocator};
use lru::LruCache;

use crate::text::font::Font;

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
                panic!("glyph LRU is empty: impossible to find empty atlas section");
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

pub struct TextContext<'a> {
    buffer: cosmic_text::Buffer,
    font_system: FontSystem,
    attribs: Attrs<'a>,
    alignment: Align,
}
impl<'a> TextContext<'a> {
    pub fn new(metrics: Metrics, font_system: FontSystem) -> Self {
        Self {
            buffer: Buffer::new_empty(metrics),
            attribs: Attrs::new(),
            alignment: Align::Left,
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

    pub fn layout(&mut self) -> Option<TextLayoutData> {
        self.buffer.shape_until_scroll(&mut self.font_system, false);

        let lines = self.buffer.layout_runs();

        let data = lines.fold(TextLayoutData::default(), |mut data, line| {
            data.total_width = data.total_width.max(line.line_w);
            data.total_height += line.line_height;
            data.lines_count += 1;

            data.glyphs = line
                .glyphs
                .iter()
                .map(|glyph| GlyphData {
                    x0: glyph.x,
                    y0: line.line_top,
                    x1: glyph.x + glyph.w,
                    y1: glyph.y,
                    code: glyph.glyph_id,
                })
                .collect();

            data
        });

        // TODO
        // Some(TextLayoutData {
        //     top: layout.line_top,
        //     baseline: layout.line_y,
        //     total_width: layout.line_w,
        //     total_height: (),
        //     line_height: (),
        //     line_count: layout.line,
        // })

        None
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct TextLayoutData {
    y_origin: f32,
    total_width: f32,
    total_height: f32,
    lines_count: u32,
    glyphs: Vec<GlyphData>,
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Default)]
pub struct GlyphData {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    code: u16,
}

#[cfg(test)]
mod tests {
    use cosmic_text::Family;

    use super::*;

    #[test]
    fn layout_text() {
        const SIZE: f32 = 8f32;
        const LINE_HEIGHT: f32 = 10f32;

        // TODO

        let metrics = cosmic_text::Metrics::new(SIZE, LINE_HEIGHT);
        let font_system = cosmic_text::FontSystem::new();

        let mut ctx = TextContext::new(metrics, font_system);

        const TEXT: &str = "Test";
        const STYLE: Attrs = Attrs::new().family(Family::Serif);

        //ctx.set_text(TEXT, &STYLE, None);
    }
}
