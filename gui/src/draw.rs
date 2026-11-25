use std::ops::{Index, IndexMut};

use ethel::{
    assets::{AssetMetadataRegistry, TextureId, TextureMetadata},
    state::data::{IndirectIndex, table::TableView},
};
use janus::{
    GpuResource,
    texture::{TextureKind, TextureView},
};
use rendrs::batch::{Batch, BatchGroupIndex, BatchManager, BatchUnitIndex};

use crate::{
    FloatState, InterfaceButtonRowTableView, InterfaceCheckboxRowTableView,
    InterfaceCommonRowTableView, InterfaceDropdownRowTableView, InterfaceFloatDataRowTableView,
    InterfaceImageRowTableView, InterfacePanelRowTableView, InterfaceSliderRowTableView,
    InterfaceTextRowTableView,
    env::UiEnv,
    look::{LooksManager, Rgb},
    text::{GlyphAtlas, TextComposer},
};

#[derive(Debug)]
pub struct InterfaceAggregator<'t> {
    pub environment: &'t UiEnv,
    pub commons: InterfaceCommonRowTableView<'t>,
    pub panels: InterfacePanelRowTableView<'t>,
    pub floating: InterfaceFloatDataRowTableView<'t>,
    pub texts: InterfaceTextRowTableView<'t>,
    pub images: InterfaceImageRowTableView<'t>,
    pub buttons: InterfaceButtonRowTableView<'t>,
    pub sliders: InterfaceSliderRowTableView<'t>,
    pub checkboxes: InterfaceCheckboxRowTableView<'t>,
    pub dropdowns: InterfaceDropdownRowTableView<'t>,
    pub text_resolve_buf: &'t mut String,
    pub looks: &'t LooksManager,
}
impl InterfaceAggregator<'_> {
    pub fn gather_quad_elements(
        &mut self,
        text_composer: &mut TextComposer,
        glyph_atlas: &mut GlyphAtlas,
        buffer: &mut Vec<InterfaceObject>,
    ) {
        let count = self.commons.len();
        let reserve = count.saturating_sub(buffer.len());
        buffer.reserve(reserve);

        for index in 1..count {
            let archetype = self.commons.archetype[index];
            match archetype {
                crate::ComponentKind::Null => {}
                crate::ComponentKind::Panel(indirect_index) => {
                    self.gather_panel(index, indirect_index, buffer)
                }
                crate::ComponentKind::Image(indirect_index) => {
                    self.gather_image(index, indirect_index, buffer)
                }
                crate::ComponentKind::Button { handle, .. } => {
                    self.gather_button(index, handle, buffer)
                }
                crate::ComponentKind::Text(indirect_index) => {
                    self.gather_text(index, indirect_index, text_composer, glyph_atlas, buffer);
                }
                crate::ComponentKind::Slider(handle) => {
                    self.gather_slider(index, handle, buffer);
                }
                crate::ComponentKind::Checkbox(handle) => {
                    self.gather_checkbox(index, handle, buffer);
                }
                crate::ComponentKind::Dropdown { handle, .. } => {
                    self.gather_dropdown(index, handle, buffer)
                }
            };
        }
    }

    fn gather_panel(
        &self,
        common_handle: usize,
        panel_index: IndirectIndex,
        out: &mut Vec<InterfaceObject>,
    ) {
        let (_root, hidden, float_id) = self.panels.coalesced(panel_index);
        if *hidden {
            return;
        }

        let hovered = self.commons.interaction_data[common_handle].hovered;
        let look_id = self.commons.look[common_handle];

        let look = self.looks.get(look_id);
        let background = look.panel_background;
        let background_hover = look.panel_background_hover;
        let opacity = look.panel_opacity;
        let opacity_hover = look.panel_opacity_hover;

        // todo border

        let (rgb, opacity) = if hovered {
            (
                <Rgb as Into<glam::Vec3>>::into(background_hover),
                opacity_hover,
            )
        } else {
            (<Rgb as Into<glam::Vec3>>::into(background), opacity)
        };

        let color = glam::vec4(rgb.x, rgb.y, rgb.z, opacity);

        let bounds = self.commons.feedback_bounds[common_handle];
        let layer = self.commons.layer[common_handle];
        let panel_size = bounds.size();

        out.push(InterfaceObject {
            position: bounds.min,
            size: panel_size,
            color,
            attachment: None,
            layer,
        });

        if let Some(float_id) = float_id {
            let (area, offset, &state) = self.floating.coalesced(*float_id);
            let active = matches!(state, FloatState::Active);

            // draw panel grab-area always
            {
                use crate::FloatGrabArea::*;
                let (position, size) = match area {
                    SectionHoriz { height } => (bounds.min, glam::vec2(panel_size.x, *height)),
                    SectionVert { width } => (bounds.min, glam::vec2(*width, panel_size.y)),
                    Corner { width, height } => (bounds.min, glam::vec2(*width, *height)),
                };

                //debug color
                const GRAB_COLOR: glam::Vec4 = glam::vec4(0.36, 0.36, 0.36, 1.0);
                const GRAB_COLOR_HOVER: glam::Vec4 = glam::vec4(0.26, 0.26, 0.26, 1.0);
                out.push(InterfaceObject {
                    position,
                    size,
                    color: if hovered | active {
                        GRAB_COLOR_HOVER
                    } else {
                        GRAB_COLOR
                    },
                    attachment: None,
                    layer: layer + 1,
                });
            }

            // draw floating new-bounds preview
            if active {
                const THICKNESS: f32 = 2.0;
                // slight offset to blue
                // negative values are clamped to 0 automatically
                let color = glam::vec4(0.79 - color.x, 0.89 - color.y, 1.0 - color.z, 0.8);
                let base_pos = bounds.min() + *offset;

                out.push(InterfaceObject {
                    position: base_pos,
                    size: glam::vec2(panel_size.x, THICKNESS),
                    color,
                    attachment: None,
                    layer: layer + 1,
                });
                out.push(InterfaceObject {
                    position: base_pos,
                    size: glam::vec2(THICKNESS, panel_size.y),
                    color,
                    attachment: None,
                    layer: layer + 1,
                });
                out.push(InterfaceObject {
                    position: base_pos + glam::vec2(0f32, panel_size.y - THICKNESS),
                    size: glam::vec2(panel_size.x, THICKNESS),
                    color,
                    attachment: None,
                    layer: layer + 1,
                });
                out.push(InterfaceObject {
                    position: base_pos + glam::vec2(panel_size.x - THICKNESS, 0f32),
                    size: glam::vec2(THICKNESS, panel_size.y),
                    color,
                    attachment: None,
                    layer: layer + 1,
                });
            }
        }
    }

    fn gather_text(
        &mut self,
        common_handle: usize,
        text_index: IndirectIndex,
        text_composer: &mut TextComposer,
        glyph_atlas: &mut GlyphAtlas,
        out: &mut Vec<InterfaceObject>,
    ) {
        let tdid = self.texts.solve(text_index).as_index();
        let contents = self.texts.contents[tdid];
        let metrics = self.texts.metrics[tdid];
        let font = self.texts.font_name[tdid];

        let text_buf = &mut self.text_resolve_buf;
        text_buf.clear();
        contents.resolve(self.environment, text_buf);

        let core_bounds = self.commons.feedback_bounds[common_handle];
        let size = core_bounds.size();

        text_composer.set_buffer_size(Some(size.x), Some(size.y));
        text_composer.set_font_metrics(metrics);
        text_composer.set_font(font);
        text_composer.set_text(text_buf);

        let anchor = self.commons.feedback_anchor[common_handle];
        let layer = self.commons.layer[common_handle];
        text_composer.compose(anchor.x, anchor.y, layer, glyph_atlas);

        out.extend(text_composer.elements());
    }

    fn gather_image(
        &self,
        common_handle: usize,
        image_index: IndirectIndex,
        out: &mut Vec<InterfaceObject>,
    ) {
        let (tint, &tint_opacity, &texture) = self.images.coalesced(image_index);
        let opacity = tint_opacity.min(1.0);

        let color = glam::vec4(tint.x, tint.y, tint.z, opacity);
        let attachment = InterfaceAttachment::Texture(texture);

        let bounds = self.commons.feedback_bounds[common_handle];
        let layer = self.commons.layer[common_handle];

        out.push(InterfaceObject {
            position: bounds.min,
            size: bounds.size(),
            color,
            attachment: Some(attachment),
            layer,
        });
    }

    fn gather_button(
        &self,
        common_handle: usize,
        _button_index: IndirectIndex,
        out: &mut Vec<InterfaceObject>,
    ) {
        let interact = self.commons.interaction_data[common_handle];
        let look_id = self.commons.look[common_handle];

        let hovered = interact.hovered;
        let pressed = interact.pressed;

        let look = self.looks.get(look_id);
        let background = look.button_background;
        let background_hover = look.button_background_hover;
        let background_press = look.button_background_press;
        let opacity = look.button_opacity;
        let opacity_hover = look.button_opacity_hover;
        let opacity_press = look.button_opacity_press;

        // todo border

        let (rgb, opacity) = if pressed {
            (
                <Rgb as Into<glam::Vec3>>::into(background_press),
                opacity_press,
            )
        } else if hovered {
            (
                <Rgb as Into<glam::Vec3>>::into(background_hover),
                opacity_hover,
            )
        } else {
            (<Rgb as Into<glam::Vec3>>::into(background), opacity)
        };

        let color = glam::vec4(rgb.x, rgb.y, rgb.z, opacity);

        let bounds = self.commons.feedback_bounds[common_handle];
        let layer = self.commons.layer[common_handle];

        out.push(InterfaceObject {
            position: bounds.min,
            size: bounds.size(),
            color,
            attachment: None,
            layer,
        });
    }

    fn gather_slider(
        &self,
        common_handle: usize,
        slider_index: IndirectIndex,
        out: &mut Vec<InterfaceObject>,
    ) {
        let interact = self.commons.interaction_data[common_handle];
        let look_id = self.commons.look[common_handle];

        let hovered = interact.hovered;
        let pressed = interact.pressed;

        let look = self.looks.get(look_id);
        let knob_base = look.slider_knob;
        let knob_hover = look.slider_knob_hover;
        let knob_press = look.slider_knob_press;
        let track = look.slider_track;

        // todo border

        let knob_rgb = if pressed {
            <Rgb as Into<glam::Vec3>>::into(knob_press)
        } else if hovered {
            <Rgb as Into<glam::Vec3>>::into(knob_hover)
        } else {
            <Rgb as Into<glam::Vec3>>::into(knob_base)
        };
        let track_rgb = <Rgb as Into<glam::Vec3>>::into(track);

        let knob_color = glam::vec4(knob_rgb.x, knob_rgb.y, knob_rgb.z, 1.0);
        let track_color = glam::vec4(track_rgb.x, track_rgb.y, track_rgb.z, 1.0);

        let slider_direct = self.sliders.solve(slider_index).as_index();
        let value = self.sliders.value_cache[slider_direct];
        let is_vertical = self.sliders.vertical[slider_direct];

        let bounds = self.commons.feedback_bounds[common_handle];
        let layer = self.commons.layer[common_handle];

        let (knob_axis, cross_axis) = if is_vertical {
            (glam::Vec2::Y, glam::Vec2::X)
        } else {
            (glam::Vec2::X, glam::Vec2::Y)
        };

        const TRACK_THICKNESS: f32 = 0.1; //10% of cross axis

        let bounds_size = bounds.size();
        let knob_size = (bounds_size * cross_axis).max_element() * 0.35;
        let track_size = (bounds_size * knob_axis) + (cross_axis * knob_size * TRACK_THICKNESS);
        let oriented_value = if is_vertical { 1.0 - value } else { value };
        let knob_posx = ((bounds_size * knob_axis).max_element() - knob_size) * oriented_value;

        out.push(InterfaceObject {
            position: bounds.min + (knob_size * 0.5 * cross_axis),
            size: track_size,
            color: track_color,
            attachment: None,
            layer,
        });
        out.push(InterfaceObject {
            position: bounds.min + (knob_posx * knob_axis),
            size: glam::Vec2::splat(knob_size),
            color: knob_color,
            attachment: None,
            layer,
        });
    }

    fn gather_checkbox(
        &self,
        common_handle: usize,
        checkbox_index: IndirectIndex,
        out: &mut Vec<InterfaceObject>,
    ) {
        let look_id = self.commons.look[common_handle];

        let look = self.looks.get(look_id);
        let cb_bg_base = look.checkbox_background;
        let cb_bg_hover = look.checkbox_background_hover;
        let cb_border = look.checkbox_border;
        let cb_border_width = look.checkbox_border_width;
        let cb_check = look.checkbox_check;

        let interact = self.commons.interaction_data[common_handle];
        let checked = self.checkboxes.value(checkbox_index);

        let background = if interact.hovered {
            cb_bg_hover
        } else {
            cb_bg_base
        };

        let bounds = self.commons.feedback_bounds[common_handle];
        let layer = self.commons.layer[common_handle];

        // border
        out.push(InterfaceObject {
            position: bounds.min,
            size: bounds.size(),
            color: cb_border.into(),
            attachment: None,
            layer,
        });
        // overlay background
        let border_margin = glam::Vec2::splat(cb_border_width);
        let inner_pos = bounds.min + border_margin;
        let inner_size = bounds.size() - border_margin * 2.0;
        out.push(InterfaceObject {
            position: inner_pos,
            size: inner_size,
            color: background.into(),
            attachment: None,
            layer,
        });
        // inner check
        if *checked {
            const CHECK_WIDTH_BG_RELATIVE: f32 = 0.6;
            let inner_margin = inner_size * (1.0 - CHECK_WIDTH_BG_RELATIVE);
            let check_pos = inner_pos + inner_margin * 0.5;
            let check_size = inner_size * CHECK_WIDTH_BG_RELATIVE;
            out.push(InterfaceObject {
                position: check_pos,
                size: check_size,
                color: cb_check.into(),
                attachment: None,
                layer,
            });
        }
    }

    fn gather_dropdown(
        &self,
        common_handle: usize,
        dropdown_index: IndirectIndex,
        out: &mut Vec<InterfaceObject>,
    ) {
        let look_id = self.commons.look[common_handle];

        let look = self.looks.get(look_id);
        let base_color = look.menu_button;
        let hover_color = look.menu_button_hover;
        let press_color = look.menu_button_press;
        let open_color = look.menu_button_open;
        let _symbol_color = look.menu_button_symbol;

        let interact = self.commons.interaction_data[common_handle];
        let is_open = self.dropdowns.open(dropdown_index);

        let background_color = if interact.pressed {
            press_color.into()
        } else if interact.hovered {
            hover_color.into()
        } else if *is_open {
            open_color
        } else {
            base_color
        }
        .into();

        let bounds = self.commons.feedback_bounds[common_handle];
        let layer = self.commons.layer[common_handle];

        out.push(InterfaceObject {
            position: bounds.min,
            size: bounds.size(),
            color: background_color,
            attachment: None,
            layer,
        });
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct InterfaceObject {
    /// top-left position of quad
    pub position: glam::Vec2,
    // size in pixels
    pub size: glam::Vec2,
    pub color: glam::Vec4,
    pub attachment: Option<InterfaceAttachment>,
    pub layer: u32,
}
impl InterfaceObject {
    pub const fn new(
        position: glam::Vec2,
        size: glam::Vec2,
        color: glam::Vec4,
        attachment: Option<InterfaceAttachment>,
        layer: u32,
    ) -> Self {
        Self {
            position,
            size,
            color,
            attachment,
            layer,
        }
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InterfaceAttachment {
    Texture(TextureId),
    TextureSection { texture_id: TextureId, uv: [f32; 4] },
}

#[derive(Debug, Clone)]
pub struct BatchingLayerCompositor {
    layers: Vec<BatchManager<Quad>>,
}
impl Default for BatchingLayerCompositor {
    fn default() -> Self {
        Self { layers: Vec::new() }
    }
}
impl IndexMut<usize> for BatchingLayerCompositor {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.layer_mut(index)
    }
}
impl Index<usize> for BatchingLayerCompositor {
    type Output = BatchManager<Quad>;

    fn index(&self, index: usize) -> &Self::Output {
        self.layer(index)
    }
}
impl BatchingLayerCompositor {
    pub fn new(layers: usize) -> Self {
        Self {
            layers: vec![BatchManager::new(); layers],
        }
    }

    pub const fn layer_span(&self) -> usize {
        self.layers.len()
    }

    pub fn batches(&self) -> impl Iterator<Item = &Batch<Quad>> {
        self.layers.iter().map(|layer| layer.batches()).flatten()
    }

    pub fn clear_layers(&mut self) {
        self.layers.iter_mut().for_each(BatchManager::clear);
    }

    pub fn insert(
        &mut self,
        element: InterfaceObject,
        registry: &AssetMetadataRegistry<TextureMetadata>,
    ) -> (BatchGroupIndex, BatchUnitIndex) {
        let mut quad_uv = [0., 0., 1., 1.];
        let texture = if let Some(attachment) = element.attachment {
            let texture = match attachment {
                InterfaceAttachment::Texture(texture_id) => registry.get(texture_id),
                InterfaceAttachment::TextureSection { texture_id, uv } => {
                    quad_uv = uv;
                    registry.get(texture_id)
                }
            };

            texture
                .and_then(|tex| tex.view)
                .unwrap_or(TextureView::null(TextureKind::Dim2D))
        } else {
            TextureView::null(TextureKind::Dim2D)
        };

        let quad = Quad {
            position: element.position,
            size: element.size,
            color: element.color,
            uv: quad_uv,
            texture_id: texture.resource_id(),
        };

        if self.layers.len() <= element.layer as usize {
            self.layers
                .resize_with(element.layer as usize + 1, BatchManager::new);
        }
        let layer = self.layer_mut(element.layer as usize);
        layer.insert(quad, texture)
    }

    pub fn layer(&self, index: usize) -> &BatchManager<Quad> {
        &self.layers[index]
    }

    pub fn layer_mut(&mut self, index: usize) -> &mut BatchManager<Quad> {
        &mut self.layers[index]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct BatchIndex(usize);
impl BatchIndex {
    pub const fn from_index(index: usize) -> Option<Self> {
        if index < rendrs::BATCH_UNITS {
            Some(Self(index))
        } else {
            None
        }
    }

    pub const fn get(self) -> usize {
        self.0
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Quad {
    pub position: glam::Vec2,
    pub size: glam::Vec2,
    pub color: glam::Vec4,
    pub uv: [f32; 4],
    pub texture_id: u32,
}
