use std::ops::Deref;

use cosmic_text::{
    Align, Attrs, Buffer, FontSystem, Metrics, Shaping,
    fontdb::{self, Database},
};

#[derive(Debug)]
pub struct FontLibrary {
    database: Database,
}
impl FontLibrary {
    pub fn new() -> Self {
        Self {
            database: Database::new(),
        }
    }

    fn load_fonts_dir_impl(path: impl AsRef<std::path::Path>, recursive: bool) -> Vec<Font> {
        if path.as_ref().is_dir()
            && let Ok(dir) = std::fs::read_dir(path)
        {
            dir.filter_map(Result::ok)
                .fold(Vec::new(), |mut fonts, entry| {
                    if entry.path().is_dir() && recursive {
                        let subdir = Self::load_fonts_dir_impl(entry.path(), recursive);
                        fonts.extend(subdir);
                    } else if entry.path().is_file() {
                    }
                    fonts
                });

            todo!()
        } else {
            Vec::new()
        }
    }

    fn load_font_file_impl(
        db: &mut Database,
        path: impl AsRef<std::path::Path>,
    ) -> Result<Font, ()> {
        // for now ignore error, handle this better later on
        db.load_font_file(path).map_err(|_| ())?;
        todo!()
    }

    pub fn from_paths(paths: &[impl AsRef<std::path::Path>]) -> Self {
        {
            paths.iter().filter(|path| true).for_each(|path| {});
        }
        todo!()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Font {
    id: cosmic_text::fontdb::ID,
    weight: cosmic_text::Weight,
}

pub struct TextContext {
    buffer: cosmic_text::Buffer,
    font_system: FontSystem,
}
impl TextContext {
    pub fn new(metrics: Metrics, font_system: FontSystem) -> Self {
        Self {
            buffer: Buffer::new_empty(metrics),
            font_system,
        }
    }

    pub fn set_size(&mut self, width_opt: Option<f32>, height_opt: Option<f32>) {
        self.buffer.set_size(width_opt, height_opt);
    }

    pub fn set_text(
        &mut self,
        string: &str,
        attribs: &Attrs,
        alignment: Option<crate::ItemAlignment>,
    ) {
        let alignment = match alignment {
            None => None,
            Some(alignment) => Some(match alignment {
                crate::ItemAlignment::Center | crate::ItemAlignment::Auto => Align::Center,
                crate::ItemAlignment::Start => Align::Left,
                crate::ItemAlignment::End => Align::Right,
            }),
        };

        self.buffer
            .set_text(string, attribs, Shaping::Advanced, alignment);
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
