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
    pub layout_options: LayoutOptions,
    pub layer: u32,
}

#[derive(Clone, Debug, Copy)]
pub struct FloatParams {
    pub base_pos: Option<glam::Vec2>,
    pub grab_area: FloatGrabArea,
}

#[derive(Clone, Debug, Default, Copy)]
pub struct PanelParams {
    pub hidden: bool,
    /// if present, any positional layout (flex, grid) is overriden with
    /// the absolute `base_pos` defined in [`FloatParams`]
    pub float_params: Option<FloatParams>,
}
impl PanelParams {
    pub const fn hidden() -> Self {
        Self {
            hidden: true,
            float_params: None,
        }
    }
}

#[derive(Clone, Debug, Copy, Default)]
pub struct ButtonParams {
    pub text: TextParams,
    pub callback: ButtonCallback,
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
        Self {
            contents: Self::DEFAULT_TEXT,
            font_name: Self::DEFAULT_FONT,
            font_size: Self::DEFAULT_FONT_SIZE,
            line_height: Self::DEFAULT_LINE_HEIGHT,
            never_invalidate: true,
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
