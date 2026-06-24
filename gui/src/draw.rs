use std::{
    ops::{Index, IndexMut},
    vec::Drain,
};

use ethel::{
    assets::{AssetRegistry, Import, TextureId, Upload},
    render::command::DrawGroups,
    state::data::{IndirectIndex, table::TableView},
};
use janus::{
    GpuResource,
    texture::{Texture, TextureView},
};

use crate::{
    InterfaceButtonRowTableView, InterfaceCommonRowTableView, InterfaceImageRowTableView,
    InterfacePanelRowTableView,
};

pub trait UiDrawGroup: DrawGroups + Sized {
    fn ui_draw_group() -> Self;
}

#[derive(Debug, Default)]
pub struct InterfaceObjects {
    objects: Vec<InterfaceObject>,
}
impl InterfaceObjects {
    pub fn drain(&mut self) -> Drain<'_, InterfaceObject> {
        self.objects.drain(..)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct InterfaceAggregator<'t> {
    pub commons: InterfaceCommonRowTableView<'t>,
    pub panels: InterfacePanelRowTableView<'t>,
    pub images: InterfaceImageRowTableView<'t>,
    pub buttons: InterfaceButtonRowTableView<'t>,
}
impl InterfaceAggregator<'_> {
    pub fn fill_quad_elements(&self, mut buffer: Vec<InterfaceObject>) -> InterfaceObjects {
        let count = self.commons.len();
        let reserve = count.saturating_sub(buffer.len());
        buffer.reserve(reserve);

        for index in 1..count {
            let archetype = self.commons.archetype[index];
            match archetype {
                crate::ComponentKind::Null => {}
                crate::ComponentKind::Panel(indirect_index) => {
                    self.gather_panel(index, indirect_index, &mut buffer)
                }
                crate::ComponentKind::Image(indirect_index) => {
                    self.gather_image(index, indirect_index, &mut buffer)
                }
                crate::ComponentKind::Button {
                    handle,
                    text_handle,
                } => self.gather_button(index, handle, text_handle, &mut buffer),
                crate::ComponentKind::Text(_indirect_index) => {}
            };
        }

        InterfaceObjects { objects: buffer }
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

        out.push(InterfaceObject {
            position: bounds.min,
            size: bounds.size(),
            color,
            attachment: None,
        });
    }

    fn _gather_text(
        &self,
        _common_handle: usize,
        _text_index: IndirectIndex,
        _out: &mut Vec<InterfaceObject>,
    ) {
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

        out.push(InterfaceObject {
            position: bounds.min,
            size: bounds.size(),
            color,
            attachment: Some(attachment),
        });
    }

    fn gather_button(
        &self,
        common_handle: usize,
        button_index: IndirectIndex,
        _text_index: IndirectIndex,
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

        // todo: text

        let bounds = self.commons.feedback_bounds[common_handle];

        out.push(InterfaceObject {
            position: bounds.min,
            size: bounds.size(),
            color,
            attachment: None,
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
}
impl InterfaceObject {
    pub const fn new(
        position: glam::Vec2,
        size: glam::Vec2,
        color: glam::Vec4,
        attachment: Option<InterfaceAttachment>,
    ) -> Self {
        Self {
            position,
            size,
            color,
            attachment,
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

    pub fn drain_batches(&mut self) -> Drain<'_, Batch> {
        self.output.drain(..)
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

    pub fn insert<T>(&mut self, layer: usize, element: InterfaceObject, registry: &AssetRegistry<T>)
    where
        T: Import + Upload<AsGpu = Texture>,
    {
        self.layer_mut(layer).insert(element, registry);
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
    arrays: [QuadsArray; Self::UNITS],
    textures: [Option<TextureKey>; Self::UNITS],
    head: usize,
}
impl Batch {
    pub const UNITS: usize = 16;

    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if the batch is exhausted.
    ///
    /// I.e. if the total amount of texture groups has reached the defined
    /// [`Self::UNITS`] maximum.
    pub fn is_exhausted(&self) -> bool {
        self.head >= Self::UNITS
    }

    pub fn push(
        &mut self,
        texture: TextureKey,
        array: QuadsArray,
    ) -> Result<usize, (TextureKey, QuadsArray)> {
        if self.head >= Self::UNITS {
            return Err((texture, array));
        }

        let i = self.head;
        self.head += 1;

        self.arrays[i] = array;
        self.textures[i] = Some(texture);

        Ok(i)
    }

    pub fn array(&self, index: BatchIndex) -> &QuadsArray {
        &self.arrays[index.get()]
    }

    pub fn array_mut(&mut self, index: BatchIndex) -> &mut QuadsArray {
        &mut self.arrays[index.get()]
    }

    pub fn texture(&self, index: BatchIndex) -> Option<TextureKey> {
        self.textures[index.get()]
    }

    pub fn clear(&mut self) {
        self.arrays.iter_mut().for_each(QuadsArray::clear);
        self.textures.iter_mut().for_each(|opt| *opt = None);
        self.head = 0;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct BatchLayerGroup(usize);

#[derive(Debug, Default, Clone)]
pub struct BatchingLayer {
    groups: Vec<TextureKey>,
    arrays: Vec<QuadsArray>,
}
impl BatchingLayer {
    pub fn new() -> Self {
        Self {
            groups: Vec::new(),
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
        let mut groups = self.groups.drain(..);
        let mut arrays = self.arrays.drain(..groups.len());

        let mut batch = Batch::default();
        let mut c = 1;
        while let Some(group) = groups.next()
            && let Some(array) = arrays.next()
        {
            if !batch.is_exhausted() {
                batch.push(group, array);
            } else {
                buffer.push(batch);
                batch = Batch::default();
                c += 1;
            }
        }
        c
    }

    pub fn fetch_location_or_create(&mut self, texture: TextureKey) -> BatchLayerGroup {
        let existing = self.fetch_location(texture);
        if let Some(existing) = existing {
            existing
        } else {
            let location = BatchLayerGroup(self.groups.len());
            self.groups.push(texture);
            self.arrays.push(QuadsArray::new());
            location
        }
    }

    pub fn fetch_location(&self, texture: TextureKey) -> Option<BatchLayerGroup> {
        self.groups
            .iter()
            .position(|key| *key == texture)
            .map(BatchLayerGroup)
    }

    pub fn clear(&mut self) {
        self.groups.clear();
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

    pub fn insert<T>(&mut self, element: InterfaceObject, registry: &AssetRegistry<T>)
    where
        T: Import + Upload<AsGpu = Texture>,
    {
        let mut quad_uv = [0f32; 4];
        let key = if let Some(attachment) = element.attachment {
            let texture = match attachment {
                InterfaceAttachment::Texture(texture_id) => registry.get_gpu_view(texture_id),
                InterfaceAttachment::TextureSection { texture_id, uv } => {
                    quad_uv = uv;
                    registry.get_gpu_view(texture_id)
                }
            };

            texture.map(TextureKey::from).unwrap_or_default()
        } else {
            TextureKey::default()
        };

        let uv = quad_uv;
        let location = self.fetch_location_or_create(key);
        self.arrays[location.0].push(element.position, element.size, element.color, uv);
    }
}

/// A unique OpenGL texture key used for interface elements per-texture
/// draw-call mapping.
///
/// Contains an OpenGL texture object. This can be 0 if the texture is to be
/// specified.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextureKey(pub u32);
impl From<TextureView> for TextureKey {
    fn from(value: TextureView) -> Self {
        Self(value.resource_id())
    }
}
impl From<Texture> for TextureKey {
    fn from(value: Texture) -> Self {
        Self(value.resource_id())
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Quad {
    pub position: glam::Vec2,
    pub size: glam::Vec2,
    pub color: glam::Vec4,
    pub uv: [f32; 4],
}

#[derive(Clone, Debug, Default)]
pub struct QuadsArray {
    pub array: Vec<Quad>,
}
impl QuadsArray {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            array: Vec::with_capacity(capacity),
        }
    }

    pub fn push(
        &mut self,
        position: glam::Vec2,
        size: glam::Vec2,
        color: glam::Vec4,
        uv: [f32; 4],
    ) {
        self.array.push(Quad {
            position,
            size,
            color,
            uv,
        });
    }

    pub fn push_quad(&mut self, quad: Quad) {
        self.array.push(quad);
    }

    pub fn len(&self) -> usize {
        self.array.len()
    }

    pub fn clear(&mut self) {
        self.array.clear();
    }
}

#[macro_export]
macro_rules! layout_interface_buffer {
    (instances: $ic:expr) => {
        layout_mesh_buffer!(InterfaceStorage; instances: $ic);
    };
    ($name:ident; instances: $ic:expr) => {
        layout_buffer! {
            const $name: 1, {
                enum quads: $ic => {
                    type Quad;
                    bind 0;
                    shader 5;
                };
            }
        }

        paste::paste! {
            #[derive(Debug, Default)]
            pub struct InterfaceStorageBuffers(
                pub ethel::render::buffer::PartitionedTriBuffer<1>
            );

            impl InterfaceStorageBuffers {
                pub fn new() -> Self {
                    let layout = [< Layout $name >]::create();
                    let buffer = ethel::render::buffer::PartitionedTriBuffer::new(layout);
                    [< Layout $name >]::initialise_partitions(&buffer);
                    Self(buffer)
                }
            }
        }
    };
}
