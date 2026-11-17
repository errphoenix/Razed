use ethel::TextureId;
use janus::StringHash;

use crate::{
    ButtonCallback, FloatGrabArea, InteractableCallback, LayoutOptions, SliderCallback,
    TextContents, WidgetId, look::LookId,
};

#[derive(Clone, Debug, Default)]
pub struct CommonWidgetOptions {
    pub parent: Option<WidgetId>,
    pub look: Option<LookId>,
    pub layout: LayoutOptions,
    pub layer: u32,
}
impl CommonWidgetOptions {
    pub const DEFAULT_LAYER: u32 = 5;

    pub const fn new() -> Self {
        Self {
            parent: None,
            look: None,
            layer: Self::DEFAULT_LAYER,
            layout: LayoutOptions::new(),
        }
    }

    pub const fn with_parent(self, parent: WidgetId) -> Self {
        Self {
            parent: Some(parent),
            ..self
        }
    }

    pub const fn with_look(self, look: LookId) -> Self {
        Self {
            look: Some(look),
            ..self
        }
    }

    pub const fn with_layout(self, layout: LayoutOptions) -> Self {
        Self { layout, ..self }
    }

    pub const fn with_layer(self, layer: u32) -> Self {
        Self { layer, ..self }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct FloatParams {
    pub base_pos: Option<glam::Vec2>,
    pub grab_area: FloatGrabArea,
}
impl FloatParams {
    pub const DEFAULT_GRAB_SIZE: f32 = 16.0;

    pub const fn new() -> Self {
        Self {
            base_pos: None,
            grab_area: FloatGrabArea::SectionHoriz {
                height: Self::DEFAULT_GRAB_SIZE,
            },
        }
    }

    pub const fn with_base_pos(self, base_pos: glam::Vec2) -> Self {
        Self {
            base_pos: Some(base_pos),
            ..self
        }
    }

    pub const fn with_grab_area(self, grab_area: FloatGrabArea) -> Self {
        Self { grab_area, ..self }
    }

    pub const fn with_grab_area_size(self, grab_area_size: f32) -> Self {
        Self {
            grab_area: match self.grab_area {
                FloatGrabArea::SectionHoriz { .. } => FloatGrabArea::SectionHoriz {
                    height: grab_area_size,
                },
                FloatGrabArea::SectionVert { .. } => FloatGrabArea::SectionVert {
                    width: grab_area_size,
                },
                FloatGrabArea::Corner { .. } => FloatGrabArea::Corner {
                    height: grab_area_size,
                    width: grab_area_size,
                },
            },
            ..self
        }
    }
}

#[derive(Clone, Debug, Default, Copy)]
pub struct PanelOptions {
    pub hidden: bool,
    /// if present, any positional layout (flex, grid) is overriden with
    /// the absolute `base_pos` defined in [`FloatParams`]
    pub floating: Option<FloatParams>,
}
impl PanelOptions {
    pub const fn new() -> Self {
        Self {
            hidden: false,
            floating: None,
        }
    }

    pub const fn hidden() -> Self {
        Self::new().with_hidden(true)
    }

    pub const fn floating(grab: FloatGrabArea, base_offset: Option<glam::Vec2>) -> Self {
        Self::new().with_floating(FloatParams {
            base_pos: base_offset,
            grab_area: grab,
        })
    }

    pub const fn with_hidden(self, hidden: bool) -> Self {
        Self { hidden, ..self }
    }

    pub const fn with_floating(self, floating: FloatParams) -> Self {
        Self {
            floating: Some(floating),
            ..self
        }
    }
}

#[derive(Clone, Debug, Copy, Default)]
pub struct ButtonOptions {
    pub text: TextOptions,
    pub callback: ButtonCallback,
}
impl ButtonOptions {
    pub const fn new() -> Self {
        Self {
            text: TextOptions::new(),
            callback: ButtonCallback::None,
        }
    }

    pub const fn labelled(label: TextOptions) -> Self {
        Self::new().with_label(label)
    }

    pub const fn labelled_literal(label_literal: &'static str) -> Self {
        Self::new().with_label_literal(label_literal)
    }

    pub const fn with_label(self, label: TextOptions) -> Self {
        Self {
            text: label,
            ..self
        }
    }

    pub const fn with_label_literal(self, label_literal: &'static str) -> Self {
        Self {
            text: TextOptions::new().with_contents(TextContents::literal(label_literal)),
            ..self
        }
    }

    pub const fn with_callback(self, callback: ButtonCallback) -> Self {
        Self { callback, ..self }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct TextOptions {
    pub contents: TextContents,
    pub font_name: &'static str,
    pub font_size: f32,
    pub line_height: f32,
    pub never_invalidate: bool,
}
impl TextOptions {
    pub const DEFAULT_TEXT: TextContents = TextContents::literal("Invalid Text");
    pub const DEFAULT_FONT: &'static str = "Arial";

    pub const DEFAULT_FONT_SIZE: f32 = 14.0;
    pub const DEFAULT_LINE_HEIGHT: f32 = 20.0;
}
impl Default for TextOptions {
    fn default() -> Self {
        Self::new()
    }
}
impl TextOptions {
    pub const fn new() -> Self {
        Self {
            contents: Self::DEFAULT_TEXT,
            font_name: Self::DEFAULT_FONT,
            font_size: Self::DEFAULT_FONT_SIZE,
            line_height: Self::DEFAULT_LINE_HEIGHT,
            never_invalidate: true,
        }
    }

    pub const fn from_literal(literal: &'static str) -> Self {
        Self::new().with_contents_literal(literal)
    }

    pub const fn from_contents(contents: TextContents) -> Self {
        Self::new().with_contents(contents)
    }

    pub const fn with_contents_literal(self, contents_literal: &'static str) -> Self {
        Self {
            contents: TextContents::literal(contents_literal),
            ..self
        }
    }

    pub const fn with_contents(self, contents: TextContents) -> Self {
        Self { contents, ..self }
    }

    pub const fn with_font_family(self, font_name: &'static str) -> Self {
        Self { font_name, ..self }
    }

    pub const fn with_font_size(self, font_size: f32) -> Self {
        Self { font_size, ..self }
    }

    pub const fn with_line_height(self, line_height: f32) -> Self {
        Self {
            line_height,
            ..self
        }
    }

    pub const fn with_revalidation(self, revalidation: bool) -> Self {
        Self {
            never_invalidate: !revalidation,
            ..self
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct ImageOptions {
    pub tint: glam::Vec3,
    pub tint_opacity: f32,
    pub texture: TextureId,
}
impl Default for ImageOptions {
    fn default() -> Self {
        Self::new(super::expect_fallback_texture())
    }
}
impl ImageOptions {
    pub const DEFAULT_TINT: glam::Vec3 = glam::Vec3::ZERO;
    pub const DEFAULT_TINT_OPACITY: f32 = 0.0;

    pub const fn new(texture: TextureId) -> Self {
        Self {
            tint: Self::DEFAULT_TINT,
            tint_opacity: Self::DEFAULT_TINT_OPACITY,
            texture,
        }
    }

    pub const fn tinted(texture: TextureId, tint: glam::Vec3) -> Self {
        Self::new(texture).with_tint(tint)
    }

    pub const fn with_texture(self, texture: TextureId) -> Self {
        Self { texture, ..self }
    }

    pub const fn with_tint(self, tint: glam::Vec3) -> Self {
        Self {
            tint,
            tint_opacity: 0.5,
            ..self
        }
    }

    pub const fn with_tint_opacity(self, tint_opacity: f32) -> Self {
        Self {
            tint_opacity,
            ..self
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct SliderOptions {
    pub text: Option<TextOptions>,
    pub scroll_step: f32,
    /// Normalized (0.0, 1.0)
    pub value_init: f32,
    pub value_sync_handle: Option<StringHash>,
    pub callback: SliderCallback,
    pub vertical: bool,
}
impl Default for SliderOptions {
    fn default() -> Self {
        Self::new()
    }
}
impl SliderOptions {
    /// 5%
    pub const DEFAULT_SCROLL_STEP: f32 = 0.05;

    pub const fn new() -> Self {
        Self {
            text: None,
            scroll_step: Self::DEFAULT_SCROLL_STEP,
            value_init: 0f32,
            value_sync_handle: None,
            callback: InteractableCallback::None,
            vertical: false,
        }
    }

    pub const fn labelled(label: TextOptions) -> Self {
        Self::new().with_text(label)
    }

    pub const fn labelled_literal(label_literal: &'static str) -> Self {
        Self::new().with_text_literal(label_literal)
    }

    pub const fn with_text(self, text: TextOptions) -> Self {
        Self {
            text: Some(text),
            ..self
        }
    }

    pub const fn with_text_literal(self, text_literal: &'static str) -> Self {
        Self {
            text: Some(TextOptions::from_literal(text_literal)),
            ..self
        }
    }

    pub const fn with_scroll_step(self, scroll_step: f32) -> Self {
        Self {
            scroll_step,
            ..self
        }
    }

    pub const fn with_starting_value(self, value_init: f32) -> Self {
        Self { value_init, ..self }
    }

    pub const fn with_sync_handle(self, value_sync_handle: StringHash) -> Self {
        Self {
            value_sync_handle: Some(value_sync_handle),
            ..self
        }
    }

    pub const fn with_callback(self, callback: SliderCallback) -> Self {
        Self { callback, ..self }
    }

    pub const fn with_vertical(self, vertical: bool) -> Self {
        Self { vertical, ..self }
    }
}

#[derive(Clone, Debug)]
pub enum ElementOptions {
    Panel(CommonWidgetOptions, PanelOptions),
    Button(CommonWidgetOptions, ButtonOptions),
    Text(CommonWidgetOptions, TextOptions),
    Image(CommonWidgetOptions, ImageOptions),
    Slider(CommonWidgetOptions, SliderOptions),
}
impl ElementOptions {
    pub const fn panel(core: CommonWidgetOptions, panel: PanelOptions) -> Self {
        Self::Panel(core, panel)
    }

    pub const fn button(core: CommonWidgetOptions, button: ButtonOptions) -> Self {
        Self::Button(core, button)
    }

    pub const fn text(core: CommonWidgetOptions, text: TextOptions) -> Self {
        Self::Text(core, text)
    }

    pub const fn image(core: CommonWidgetOptions, image: ImageOptions) -> Self {
        Self::Image(core, image)
    }

    pub const fn slider(core: CommonWidgetOptions, slider: SliderOptions) -> Self {
        Self::Slider(core, slider)
    }

    pub fn common_options(&self) -> &CommonWidgetOptions {
        match self {
            ElementOptions::Panel(core_element_params, _) => core_element_params,
            ElementOptions::Button(core_element_params, _) => core_element_params,
            ElementOptions::Text(core_element_params, _) => core_element_params,
            ElementOptions::Image(core_element_params, _) => core_element_params,
            ElementOptions::Slider(core_element_params, _) => core_element_params,
        }
    }
}
