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
    InterfaceButtonRowTableView, InterfaceCommonRowTableView, InterfaceImageRowTableView,
    InterfacePanelRowTableView, InterfaceSliderRowTableView, InterfaceTextRowTableView,
    env::UiEnv,
    text::{GlyphAtlas, TextComposer},
};

#[derive(Debug)]
pub struct InterfaceAggregator<'t> {
    pub environment: &'t UiEnv,
    pub commons: InterfaceCommonRowTableView<'t>,
    pub panels: InterfacePanelRowTableView<'t>,
    pub texts: InterfaceTextRowTableView<'t>,
    pub images: InterfaceImageRowTableView<'t>,
    pub buttons: InterfaceButtonRowTableView<'t>,
    pub sliders: InterfaceSliderRowTableView<'t>,
    pub text_resolve_buf: &'t mut String,
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
                crate::ComponentKind::Slider { handle, .. } => {
                    self.gather_slider(index, handle, buffer);
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
        let hovered = self.commons.hovered[common_handle];
        let (bg_c, hover_t, &opacity) = self.panels.coalesced(panel_index);

        let bg_c = glam::vec4(bg_c.x, bg_c.y, bg_c.y, opacity);
        let hover_c = hover_t + glam::vec4(0f32, 0f32, 0f32, opacity);

        let hovered_f = hovered as u32 as f32;
        let color = bg_c * (1.0 - hovered_f) + hovered_f * hover_c;

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
        let (tint, &opacity, &texture) = self.images.coalesced(image_index);
        let opacity = (tint.w + opacity).min(1.0);

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
        button_index: IndirectIndex,
        out: &mut Vec<InterfaceObject>,
    ) {
        let hovered = self.commons.hovered[common_handle];
        let pressed = self.commons.pressed[common_handle];

        let button_direct = self.buttons.solve(button_index).as_index();
        let base_color = self.buttons.base_color[button_direct];
        let hover_tint = self.buttons.hover_tint[button_direct];
        let press_tint = self.buttons.press_tint[button_direct];

        let hover_f = (hovered & !pressed) as u32 as f32;
        let press_f = pressed as u32 as f32;

        let mut color = glam::vec4(base_color.x, base_color.y, base_color.z, 1.0);
        color = color * (1.0 - hover_f) + hover_f * hover_tint;
        color = color * (1.0 - press_f) + press_f * press_tint;

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
        let hovered = self.commons.hovered[common_handle];
        let pressed = self.commons.pressed[common_handle];

        let slider_direct = self.sliders.solve(slider_index).as_index();
        let knob_color = self.sliders.knob_color[slider_direct];
        let knob_hover_tint = self.sliders.knob_hover_tint[slider_direct];
        let knob_press_tint = self.sliders.knob_press_tint[slider_direct];
        let track_color = self.sliders.track_color[slider_direct];
        let value = self.sliders.value_cache[slider_direct];

        let hover_f = (hovered & !pressed) as u32 as f32;
        let press_f = pressed as u32 as f32;

        let mut knob_color = glam::vec4(knob_color.x, knob_color.y, knob_color.z, 1.0);
        knob_color = knob_color * (1.0 - hover_f) + hover_f * knob_hover_tint;
        knob_color = knob_color * (1.0 - press_f) + press_f * knob_press_tint;
        knob_color.w = 1.0; // knob is always opaque
        let track_color = glam::vec4(track_color.x, track_color.y, track_color.z, 1.0);

        let bounds = self.commons.feedback_bounds[common_handle];
        let layer = self.commons.layer[common_handle];

        let bounds_size = bounds.size();
        let knob_size = bounds_size.y;
        const TRACK_THICKNESS: f32 = 0.1; //10% of height
        let track_size = glam::vec2(bounds_size.x, knob_size * TRACK_THICKNESS);
        let knob_posx = (bounds_size.x - knob_size) * value;

        out.push(InterfaceObject {
            position: bounds.min + glam::vec2(0f32, knob_size * 0.5),
            size: track_size,
            color: track_color,
            attachment: None,
            layer,
        });
        out.push(InterfaceObject {
            position: bounds.min + glam::vec2(knob_posx, 0f32),
            size: glam::Vec2::splat(knob_size),
            color: knob_color,
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
pub struct BatchingLayerCompositor<const LAYERS: usize> {
    layers: [BatchManager<Quad>; LAYERS],
}
impl<const LAYERS: usize> Default for BatchingLayerCompositor<LAYERS> {
    fn default() -> Self {
        Self {
            layers: std::array::from_fn(|_| BatchManager::default()),
        }
    }
}
impl<const LAYERS: usize> IndexMut<usize> for BatchingLayerCompositor<LAYERS> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.layer_mut(index)
    }
}
impl<const LAYERS: usize> Index<usize> for BatchingLayerCompositor<LAYERS> {
    type Output = BatchManager<Quad>;

    fn index(&self, index: usize) -> &Self::Output {
        self.layer(index)
    }
}
impl<const LAYERS: usize> BatchingLayerCompositor<LAYERS> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocate *each* layer [`batch manager`](BatchManager) with the given
    /// `capacity`.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            layers: std::array::from_fn(|_| BatchManager::with_capacity(capacity)),
        }
    }

    /// Returns a chained iterator of all layers' batches.
    pub fn batches(&self) -> impl Iterator<Item = &Batch<Quad>> {
        self.layers[0]
            .batches()
            .iter()
            .chain(self.layers[1].batches())
            .chain(self.layers[2].batches())
            .chain(self.layers[3].batches())
            .chain(self.layers[4].batches())
            .chain(self.layers[5].batches())
            .chain(self.layers[6].batches())
            .chain(self.layers[7].batches())
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
