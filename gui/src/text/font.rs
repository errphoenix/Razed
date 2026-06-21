use std::{borrow::Cow, collections::HashMap, ffi::OsStr, sync::Arc};

use cosmic_text::{
    Weight,
    fontdb::{Database, Source},
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
    map: StringMap<Font>,
}
impl FontLibrary {
    pub fn new() -> Self {
        Self {
            map: HashMap::with_hasher(janus::StringHasher::new()),
        }
    }

    pub fn from_paths(
        db: &mut Database,
        paths: &[impl AsRef<std::path::Path>],
        recursive: bool,
    ) -> Self {
        let mut fonts = paths.iter().filter(|path| path.as_ref().exists()).fold(
            Vec::new(),
            |mut book, path| {
                let ids = Self::load_fonts_dir_impl(db, path, recursive);
                book.extend(ids);
                book
            },
        );

        let fonts_map = fonts
            .drain(..)
            .filter_map(|(result, path)| Self::treat_font_result(result, path))
            .collect::<StringMap<_>>();

        Self { map: fonts_map }
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

        static FALLBACK_FAMILY: &str = "Arial";

        let path = path.as_ref().to_path_buf();
        if let Some(&font) = fonts.first() {
            let f = db.face(font).unwrap();
            let weight = f.weight;

            let family = f
                .families
                .first()
                .map(|(f, _)| f.clone())
                .unwrap_or_else(|| {
                    tracing::event!(
                        tracing::Level::WARN,
                        "failed to determine font family for font: fallback to '{}'",
                        FALLBACK_FAMILY
                    );
                    FALLBACK_FAMILY.to_string()
                });

            (
                Ok(Font {
                    id: font,
                    weight,
                    family,
                }),
                path,
            )
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

    pub fn load_font(
        &mut self,
        database: &mut Database,
        path: impl AsRef<std::path::Path>,
    ) -> Option<(StringHash, Font)> {
        let (result, path) = Self::load_font_file_impl(database, &path);
        Self::treat_font_result(result, path)
    }

    pub fn get(&self, hash_id: StringHash) -> Option<&Font> {
        self.map.get(&hash_id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Font {
    pub(crate) id: cosmic_text::fontdb::ID,
    pub(crate) weight: cosmic_text::Weight,
    pub(crate) family: String,
}
