pub mod font;

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
        self.buffer.set_size(width_opt, height_opt);
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
