use ethel::TextureId;
use janus::StringHash;

use crate::{
    ButtonCallback, FloatGrabArea, InteractableCallback, LayoutOptions, SliderCallback,
    TextContents, WidgetId, look::LookId,
};

#[derive(Clone, Debug, Default)]
pub struct CoreElementParams<'children> {
    pub parent: Option<WidgetId>,
    pub look: Option<LookId>,
    pub children: Option<&'children [WidgetId]>,
    pub layout: LayoutOptions,
    pub layer: u32,
}
impl<'children> CoreElementParams<'children> {
    pub const DEFAULT_LAYER: u32 = 5;

    pub const fn new() -> Self {
        Self {
            parent: None,
            look: None,
            children: None,
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

    pub const fn with_children(self, children: &'children [WidgetId]) -> Self {
        Self {
            children: Some(children),
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
pub struct PanelParams {
    pub hidden: bool,
    /// if present, any positional layout (flex, grid) is overriden with
    /// the absolute `base_pos` defined in [`FloatParams`]
    pub floating: Option<FloatParams>,
}
impl PanelParams {
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
pub struct ButtonParams {
    pub text: TextParams,
    pub callback: ButtonCallback,
}
impl ButtonParams {
    pub const fn new() -> Self {
        Self {
            text: TextParams::new(),
            callback: ButtonCallback::None,
        }
    }

    pub const fn labelled(label: TextParams) -> Self {
        Self::new().with_label(label)
    }

    pub const fn labelled_literal(label_literal: &'static str) -> Self {
        Self::new().with_label_literal(label_literal)
    }

    pub const fn with_label(self, label: TextParams) -> Self {
        Self {
            text: label,
            ..self
        }
    }

    pub const fn with_label_literal(self, label_literal: &'static str) -> Self {
        Self {
            text: TextParams::new().with_contents(TextContents::literal(label_literal)),
            ..self
        }
    }

    pub const fn with_callback(self, callback: ButtonCallback) -> Self {
        Self { callback, ..self }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct TextParams {
    pub contents: TextContents,
    pub font_name: &'static str,
    pub font_size: f32,
    pub line_height: f32,
    pub never_invalidate: bool,
}
impl TextParams {
    pub const DEFAULT_TEXT: TextContents = TextContents::literal("Invalid Text");
    pub const DEFAULT_FONT: &'static str = "Arial";

    pub const DEFAULT_FONT_SIZE: f32 = 14.0;
    pub const DEFAULT_LINE_HEIGHT: f32 = 20.0;
}
impl Default for TextParams {
    fn default() -> Self {
        Self::new()
    }
}
impl TextParams {
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
pub struct ImageParams {
    pub tint: glam::Vec4,
    pub opacity: f32,
    pub texture: TextureId,
}
impl ImageParams {
    pub const DEFAULT_TINT: glam::Vec4 = glam::Vec4::ZERO;
    pub const DEFAULT_OPACITY: f32 = 1.0;
}
impl Default for ImageParams {
    fn default() -> Self {
        Self {
            tint: Self::DEFAULT_TINT,
            opacity: Self::DEFAULT_OPACITY,
            texture: super::FALLBACK_TEXTURE
                .read()
                .unwrap()
                .expect("fallback texture must be st"),
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct SliderParams {
    pub text: Option<TextParams>,
    pub scroll_step: f32,
    /// Normalized (0.0, 1.0)
    pub value_init: f32,
    pub value_sync_handle: Option<StringHash>,
    pub callback: SliderCallback,
    pub vertical: bool,
}
impl SliderParams {
    /// 5%
    pub const DEFAULT_SCROLL_STEP: f32 = 0.05;
}
impl Default for SliderParams {
    fn default() -> Self {
        Self {
            text: None,
            scroll_step: Self::DEFAULT_SCROLL_STEP,
            value_init: 0f32,
            value_sync_handle: None,
            callback: InteractableCallback::None,
            vertical: false,
        }
    }
}

#[derive(Clone, Debug)]
pub enum ElementParams<'children> {
    Panel(CoreElementParams<'children>, PanelParams),
    Button(CoreElementParams<'children>, ButtonParams),
    Text(CoreElementParams<'children>, TextParams),
    Image(CoreElementParams<'children>, ImageParams),
    Slider(CoreElementParams<'children>, SliderParams),
}
impl ElementParams<'_> {
    pub fn core(&'_ self) -> &CoreElementParams<'_> {
        match self {
            ElementParams::Panel(core_element_params, _) => core_element_params,
            ElementParams::Button(core_element_params, _) => core_element_params,
            ElementParams::Text(core_element_params, _) => core_element_params,
            ElementParams::Image(core_element_params, _) => core_element_params,
            ElementParams::Slider(core_element_params, _) => core_element_params,
        }
    }
}
