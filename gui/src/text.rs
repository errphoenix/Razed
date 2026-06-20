use std::{borrow::Cow, collections::HashMap, ffi::OsStr, sync::Arc};

use cosmic_text::{
    Align, Attrs, Buffer, FontSystem, Metrics, Shaping, Weight,
    fontdb::{Database, FaceInfo, Source},
};
use janus::{StringHash, StringMap};

#[derive(Debug, thiserror::Error)]
pub enum FontError {
    #[error("failed to load font due to IO error: {0}")]
    LoadIoError(std::io::Error),
    #[error("no fonts found at specified source path: {0}")]
    NoFontsFoundAtSource(std::path::PathBuf),
}

pub type FontResult = Result<Font, FontError>;

#[derive(Debug)]
pub struct FontLibrary {
    database: Database,
    map: StringMap<Font>,
}
impl FontLibrary {
    pub fn new() -> Self {
        Self {
            database: Database::new(),
            map: HashMap::with_hasher(janus::StringHasher::new()),
        }
    }

    fn load_fonts_dir_impl(
        db: &mut Database,
        path: &impl AsRef<std::path::Path>,
        recursive: bool,
    ) -> Vec<(FontResult, std::path::PathBuf)> {
        if path.as_ref().is_dir()
            && let Ok(dir) = std::fs::read_dir(path)
        {
            dir.filter_map(Result::ok)
                .fold(Vec::new(), |mut fonts, entry| {
                    if entry.path().is_dir() && recursive {
                        let subdir = Self::load_fonts_dir_impl(db, &entry.path(), recursive);
                        fonts.extend(subdir);
                    } else if entry.path().is_file() {
                        let result = Self::load_font_file_impl(db, &entry.path());
                        fonts.push(result);
                    }
                    fonts
                })
        } else {
            Vec::new()
        }
    }

    fn load_font_file_impl(
        db: &mut Database,
        path: &impl AsRef<std::path::Path>,
    ) -> (FontResult, std::path::PathBuf) {
        let read = std::fs::read(path).map_err(FontError::LoadIoError);
        if let Err(err) = read {
            return (Err(err), path.as_ref().to_path_buf());
        }
        let raw = read.unwrap();

        // A font file may provide multiple fonts of differing weight, but we
        // will only take the most "regular" one out of all these.
        // We do this by sorting the fonts by their associated weight, but
        // offsetting by the negative 'normal' weight, so that the normal
        // weight is always prioritized, followed by the thinner weights, then
        // the thicker ones.
        let fonts = {
            let mut fonts = db.load_font_source(Source::Binary(Arc::new(raw)));
            fonts.sort_by(|&id0, &id1| {
                const REGULAR: u16 = Weight::NORMAL.0;
                let w0 = db.face(id0).unwrap().weight.0 - REGULAR;
                let w1 = db.face(id1).unwrap().weight.0 - REGULAR;
                w0.cmp(&w1)
            });
            fonts
        };

        let path = path.as_ref().to_path_buf();
        if let Some(&font) = fonts.first() {
            let weight = db.face(font).unwrap().weight;
            (Ok(Font { id: font, weight }), path)
        } else {
            (Err(FontError::NoFontsFoundAtSource(path.clone())), path)
        }
    }

    fn resolve_fontfile_name(path: &'_ impl AsRef<std::path::Path>) -> Cow<'_, str> {
        // track amount of unknown font names avoid name collisions
        static UNKNOWN_NAMES_COUNT: std::sync::Mutex<u32> = std::sync::Mutex::new(0);

        path.as_ref()
            .file_prefix()
            .map(OsStr::to_string_lossy)
            .unwrap_or_else(|| {
                let n = {
                    let mut g = UNKNOWN_NAMES_COUNT.lock().unwrap();
                    let n = *g;
                    *g += 1;
                    n
                };
                Cow::Owned(format!("unknown-font-{n}"))
            })
    }

    fn treat_font_result(
        result: FontResult,
        path: std::path::PathBuf,
    ) -> Option<(StringHash, Font)> {
        match result {
            Ok(font) => {
                let name = Self::resolve_fontfile_name(&path).to_string();
                let hash = janus::hash_string(&name);
                tracing::event!(
                    tracing::Level::INFO,
                    "successfully loaded font: {name:^16} [source=:{}]",
                    path.display(),
                );
                Some((hash, font))
            }
            Err(err) => {
                tracing::event!(tracing::Level::ERROR, "failed to load font: {}", err);
                None
            }
        }
    }

    pub fn from_paths(paths: &[impl AsRef<std::path::Path>], recursive: bool) -> Self {
        let mut db = Database::new();

        let mut fonts = paths.iter().filter(|path| path.as_ref().exists()).fold(
            Vec::new(),
            |mut book, path| {
                let ids = Self::load_fonts_dir_impl(&mut db, path, recursive);
                book.extend(ids);
                book
            },
        );

        let fonts_map = fonts
            .drain(..)
            .filter_map(|(result, path)| Self::treat_font_result(result, path))
            .collect::<StringMap<_>>();

        Self {
            database: db,
            map: fonts_map,
        }
    }

    pub fn load_font(&mut self, path: impl AsRef<std::path::Path>) -> Option<(StringHash, Font)> {
        let (result, path) = Self::load_font_file_impl(&mut self.database, &path);
        Self::treat_font_result(result, path)
    }

    pub fn get(&self, hash_id: StringHash) -> Option<Font> {
        self.map.get(&hash_id).copied()
    }

    pub(crate) fn get_font_from_hash(&self, hash_id: StringHash) -> Option<&FaceInfo> {
        self.get(hash_id).map(|f| self.get_font(f)).flatten()
    }

    pub(crate) fn get_font(&self, font: Font) -> Option<&FaceInfo> {
        self.database.face(font.id)
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
