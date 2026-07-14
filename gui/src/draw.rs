use std::ops::{Index, IndexMut};

use ethel::{
    assets::{AssetMetadataRegistry, TextureId, TextureMetadata},
    state::data::{IndirectIndex, table::TableView},
};
use janus::texture::{TextureKey, TextureTarget};

use crate::{
    InterfaceButtonRowTableView, InterfaceCommonRowTableView, InterfaceImageRowTableView,
    InterfacePanelRowTableView, InterfaceTextRowTableView,
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
        text_composer.set_text(text_buf);
        text_composer.set_font(font);

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
    layers: [BatchingLayer; LAYERS],
    output: Vec<Batch>,
}
impl<const LAYERS: usize> Default for BatchingLayerCompositor<LAYERS> {
    fn default() -> Self {
        Self {
            layers: std::array::from_fn(|_| BatchingLayer::default()),
            output: Vec::default(),
        }
    }
}
impl<const LAYERS: usize> IndexMut<usize> for BatchingLayerCompositor<LAYERS> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.layer_mut(index)
    }
}
impl<const LAYERS: usize> Index<usize> for BatchingLayerCompositor<LAYERS> {
    type Output = BatchingLayer;

    fn index(&self, index: usize) -> &Self::Output {
        self.layer(index)
    }
}
impl<const LAYERS: usize> BatchingLayerCompositor<LAYERS> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            output: Vec::with_capacity(capacity),
            ..Default::default()
        }
    }

    pub fn clear_batches(&mut self) {
        self.output.clear();
    }

    pub fn batches(&self) -> &[Batch] {
        &self.output
    }

    /// Consumes all layers and prepares the internal [`Batch`] output buffer.
    ///
    /// This can then be accessed with [`Self::batches`] or
    /// [`Self::drain_batches`].
    ///
    /// Since this consumes layers, no [`Self::clear_layers`] is explicitly
    /// necessary.
    pub fn pull_batches(&mut self) {
        self.layers.iter_mut().for_each(|layer| {
            layer.export_batches(&mut self.output);
        });
    }

    pub fn clear_layers(&mut self) {
        self.layers.iter_mut().for_each(BatchingLayer::clear);
    }

    pub fn insert(
        &mut self,
        element: InterfaceObject,
        registry: &AssetMetadataRegistry<TextureMetadata>,
    ) {
        self.layer_mut(element.layer as usize)
            .insert(element, registry);
    }

    pub fn layer(&self, index: usize) -> &BatchingLayer {
        &self.layers[index]
    }

    pub fn layer_mut(&mut self, index: usize) -> &mut BatchingLayer {
        &mut self.layers[index]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct BatchIndex(usize);
impl BatchIndex {
    pub const fn from_index(index: usize) -> Option<Self> {
        if index < Batch::UNITS {
            Some(Self(index))
        } else {
            None
        }
    }

    pub const fn get(self) -> usize {
        self.0
    }
}

/// Represents a single draw-call, on 16 concurrent texture units.
///
/// Textures are distributed among `N` amount of texture units, each command
/// samples from a different unit in order to minimize the total number of draw
/// calls.
///
/// This allows up to `N` draw-calls to be submitted concurrently.
#[derive(Debug, Default, Clone)]
pub struct Batch {
    array: QuadsArray,
    textures: [Option<TextureKey>; Self::UNITS],
    head: usize,
}
impl Batch {
    pub const UNITS: usize = 16;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn bind_textures(&self) {
        for (i, texture) in self.textures.iter().enumerate() {
            let texture = texture.unwrap_or_default();
            let unit = i as u32;
            janus::texture::bind_without_meta(TextureTarget::Flat, texture, unit);
        }
    }

    /// Returns `true` if the batch is exhausted.
    ///
    /// I.e. if the total amount of texture groups has reached the defined
    /// [`Self::UNITS`] maximum.
    pub fn is_exhausted(&self) -> bool {
        self.head >= Self::UNITS
    }

    pub fn push(&mut self, texture: TextureKey, array: &QuadsArray) -> Option<usize> {
        if self.head >= Self::UNITS {
            return None;
        }

        let i = self.head;
        self.head += 1;

        self.array.inner.extend_from_slice(&array.inner);
        self.textures[i] = Some(texture);

        Some(i)
    }

    pub fn array(&self) -> &QuadsArray {
        &self.array
    }

    pub fn array_mut(&mut self) -> &mut QuadsArray {
        &mut self.array
    }

    pub fn texture(&self, index: BatchIndex) -> Option<TextureKey> {
        self.textures[index.get()]
    }

    pub fn textures(&self) -> [Option<TextureKey>; Self::UNITS] {
        self.textures
    }

    pub fn clear(&mut self) {
        self.textures.iter_mut().for_each(|opt| *opt = None);
        self.array.clear();
        self.head = 0;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct BatchLayerGroup(usize);

#[derive(Debug, Default, Clone)]
pub struct BatchingLayer {
    units: Vec<TextureKey>,
    arrays: Vec<QuadsArray>,
}
impl BatchingLayer {
    pub fn new() -> Self {
        Self {
            units: Vec::new(),
            arrays: Vec::new(),
        }
    }

    /// Export batches to a preallocated `buffer`.
    ///
    /// This also empties the current [`TextureKey`] groups and [`QuadArrays`]
    /// collections, so no [`Self::clear`] is explicitly necessary.
    ///
    /// # Returns
    /// Returns the total amount of batches created and pushed to `buffer`.
    pub fn export_batches(&mut self, buffer: &mut Vec<Batch>) -> u32 {
        let groups = self.units.drain(..);
        let arrays = self.arrays.drain(..groups.len());

        let mut batch = Batch::default();
        let mut c = 1u32;

        for (group, mut array) in groups.zip(arrays) {
            if batch.is_exhausted() {
                buffer.push(batch);
                batch = Batch::default();
                c += 1;
            }

            // offset texture unit index to batch-local index, which
            // is capped to 16
            let offset = (c - 1) * Batch::UNITS as u32;
            array
                .inner
                .iter_mut()
                .for_each(|quad| quad.texture_unit -= offset);
            batch.push(group, &array);
        }
        buffer.push(batch);

        c
    }

    pub fn fetch_location_or_create(&mut self, texture: TextureKey) -> BatchLayerGroup {
        let existing = self.fetch_location(texture);
        if let Some(existing) = existing {
            existing
        } else {
            let location = BatchLayerGroup(self.units.len());
            self.units.push(texture);
            self.arrays.push(QuadsArray::new());
            location
        }
    }

    pub fn fetch_location(&self, texture: TextureKey) -> Option<BatchLayerGroup> {
        self.units
            .iter()
            .position(|key| *key == texture)
            .map(BatchLayerGroup)
    }

    pub fn clear(&mut self) {
        self.units.clear();
        self.arrays.clear();
    }

    pub fn array_count(&self) -> usize {
        self.arrays.len()
    }

    pub fn get_array(&self, location: BatchLayerGroup) -> Option<&QuadsArray> {
        self.arrays.get(location.0)
    }

    pub fn get_array_mut(&mut self, location: BatchLayerGroup) -> Option<&mut QuadsArray> {
        self.arrays.get_mut(location.0)
    }

    pub fn insert(
        &mut self,
        element: InterfaceObject,
        registry: &AssetMetadataRegistry<TextureMetadata>,
    ) {
        let mut quad_uv = [0., 0., 1., 1.];
        let key = if let Some(attachment) = element.attachment {
            let texture = match attachment {
                InterfaceAttachment::Texture(texture_id) => registry.get(texture_id),
                InterfaceAttachment::TextureSection { texture_id, uv } => {
                    quad_uv = uv;
                    registry.get(texture_id)
                }
            };

            texture.and_then(|tex| tex.gl_object).unwrap_or_default()
        } else {
            TextureKey::default()
        };

        let quad_uv = quad_uv;
        let location = self.fetch_location_or_create(key);
        self.arrays[location.0].push(
            element.position,
            element.size,
            element.color,
            quad_uv,
            location.0 as u32,
        );
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Quad {
    pub position: glam::Vec2,
    pub size: glam::Vec2,
    pub color: glam::Vec4,
    pub uv: [f32; 4],
    pub texture_unit: u32,
}

#[derive(Clone, Debug, Default)]
pub struct QuadsArray {
    pub inner: Vec<Quad>,
}
impl QuadsArray {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Vec::with_capacity(capacity),
        }
    }

    pub fn push(
        &mut self,
        position: glam::Vec2,
        size: glam::Vec2,
        color: glam::Vec4,
        uv: [f32; 4],
        texture_unit: u32,
    ) {
        self.inner.push(Quad {
            position,
            size,
            color,
            uv,
            texture_unit,
        });
    }

    pub fn push_quad(&mut self, quad: Quad) {
        self.inner.push(quad);
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }
}
