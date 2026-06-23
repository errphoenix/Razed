use std::{
    ops::{Index, IndexMut},
    vec::Drain,
};

use ethel::{
    assets::{AssetRegistry, Import, RawTexture, TextureId, Upload},
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
pub struct DrawBatch {
    elements: Vec<QuadElement>,
}
impl DrawBatch {
    pub fn drain(&mut self) -> Drain<'_, QuadElement> {
        self.elements.drain(..)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ElementBatcher<'t> {
    pub commons: InterfaceCommonRowTableView<'t>,
    pub panels: InterfacePanelRowTableView<'t>,
    pub images: InterfaceImageRowTableView<'t>,
    pub buttons: InterfaceButtonRowTableView<'t>,
}
impl ElementBatcher<'_> {
    pub fn fill_quad_elements(&self, mut buffer: Vec<QuadElement>) -> DrawBatch {
        let count = self.commons.len();
        let reserve = count.saturating_sub(buffer.len());
        buffer.reserve(reserve);

        for index in 1..count {
            let archetype = self.commons.archetype[index];
            let element = match archetype {
                crate::ComponentKind::Null => QuadElement::default(),
                crate::ComponentKind::Panel(indirect_index) => {
                    self.gather_panel(index, indirect_index)
                }
                crate::ComponentKind::Image(indirect_index) => {
                    self.gather_image(index, indirect_index)
                }
                crate::ComponentKind::Button {
                    handle,
                    text_handle,
                } => self.gather_button(index, handle, text_handle),
                crate::ComponentKind::Text(_indirect_index) => {
                    // todo
                    QuadElement::default()
                }
            };
            buffer.push(element);
        }

        DrawBatch { elements: buffer }
    }

    fn gather_panel(&self, common_handle: usize, panel_index: IndirectIndex) -> QuadElement {
        let hovered = self.commons.hovered[common_handle];
        let (bg_c, hover_t, &opacity) = self.panels.coalesced(panel_index);

        let bg_c = glam::vec4(bg_c.x, bg_c.y, bg_c.y, opacity);
        let hover_c = hover_t + glam::vec4(0f32, 0f32, 0f32, opacity);

        let hovered_f = hovered as u32 as f32;
        let color = bg_c * (1.0 - hovered_f) + hovered_f * hover_c;

        let bounds = self.commons.feedback_bounds[common_handle];

        QuadElement {
            position: bounds.min,
            size: bounds.size(),
            color,
            attachment: None,
        }
    }

    fn _gather_text(&self, _common_handle: usize, _text_index: IndirectIndex) -> QuadElement {
        todo!()
    }

    fn gather_image(&self, common_handle: usize, image_index: IndirectIndex) -> QuadElement {
        let (tint, &opacity, &texture) = self.images.coalesced(image_index);
        let opacity = (tint.w + opacity).min(1.0);

        let color = glam::vec4(tint.x, tint.y, tint.z, opacity);
        let attachment = InterfaceAttachment::Texture(texture);

        let bounds = self.commons.feedback_bounds[common_handle];

        QuadElement {
            position: bounds.min,
            size: bounds.size(),
            color,
            attachment: Some(attachment),
        }
    }

    fn gather_button(
        &self,
        common_handle: usize,
        button_index: IndirectIndex,
        _text_index: IndirectIndex,
    ) -> QuadElement {
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

        QuadElement {
            position: bounds.min,
            size: bounds.size(),
            color,
            attachment: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuadElement {
    /// top-left position of quad
    pub position: glam::Vec2,
    // size in pixels
    pub size: glam::Vec2,
    pub color: glam::Vec4,
    pub attachment: Option<InterfaceAttachment>,
}
impl QuadElement {
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
pub struct InterfaceCompositor<const LAYERS: usize> {
    layers: [InterfaceLayer; LAYERS],
}
impl<const LAYERS: usize> Default for InterfaceCompositor<LAYERS> {
    fn default() -> Self {
        Self {
            layers: std::array::from_fn(|_| InterfaceLayer::default()),
        }
    }
}
impl<const LAYERS: usize> IndexMut<usize> for InterfaceCompositor<LAYERS> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.layer_mut(index)
    }
}

impl<const LAYERS: usize> Index<usize> for InterfaceCompositor<LAYERS> {
    type Output = InterfaceLayer;

    fn index(&self, index: usize) -> &Self::Output {
        self.layer(index)
    }
}
impl<const LAYERS: usize> InterfaceCompositor<LAYERS> {
    pub fn new() -> Self {
        Self::default()
    }

    /// The `capacity` is applied to each layer array.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            layers: std::array::from_fn(|_| InterfaceLayer::with_capacity(capacity)),
        }
    }

    const fn assert_layer_index(layer: usize) {
        assert!(layer < LAYERS, "invalid layer index provided")
    }

    pub fn insert<T>(&mut self, layer: usize, element: QuadElement, registry: &AssetRegistry<T>)
    where
        T: Import + Upload<AsGpu = Texture>,
    {
        Self::assert_layer_index(layer);
        self.layer_mut(layer).insert(element, registry);
    }

    pub fn layer(&self, index: usize) -> &InterfaceLayer {
        Self::assert_layer_index(index);
        &self.layers[index]
    }

    pub fn layer_mut(&mut self, index: usize) -> &mut InterfaceLayer {
        Self::assert_layer_index(index);
        &mut self.layers[index]
    }
}

#[derive(Debug, Default, Clone)]
pub struct InterfaceLayer {
    texture_map: rustc_hash::FxHashMap<TextureKey, QuadArrays>,
}
impl InterfaceLayer {
    pub fn new() -> Self {
        Self {
            texture_map: rustc_hash::FxHashMap::default(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            texture_map: rustc_hash::FxHashMap::with_capacity_and_hasher(
                capacity,
                Default::default(),
            ),
        }
    }

    pub fn insert<T>(&mut self, element: QuadElement, registry: &AssetRegistry<T>)
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
        let entry = self.texture_map.entry(key);

        entry
            .or_default()
            .push(element.position, element.size, element.color, uv);
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

#[derive(Clone, Debug, Default)]
pub struct QuadArrays {
    pub positions: Vec<glam::Vec2>,
    pub sizes: Vec<glam::Vec2>,
    pub colors: Vec<glam::Vec4>,
    pub uvs: Vec<[f32; 4]>,
}
impl QuadArrays {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            positions: Vec::with_capacity(capacity),
            sizes: Vec::with_capacity(capacity),
            colors: Vec::with_capacity(capacity),
            uvs: Vec::with_capacity(capacity),
        }
    }

    pub fn push(
        &mut self,
        position: glam::Vec2,
        size: glam::Vec2,
        color: glam::Vec4,
        uv: [f32; 4],
    ) {
        self.positions.push(position);
        self.sizes.push(size);
        self.colors.push(color);
        self.uvs.push(uv);
    }
}

#[macro_export]
macro_rules! layout_interface_buffer {
    (instances: $ic:expr) => {
        layout_mesh_buffer!(InterfaceStorage; instances: $ic);
    };
    ($name:ident; instances: $ic:expr) => {
        layout_buffer! {
            const $name: 4, {
                enum positions: $ic => {
                    type [f32; 2];
                    bind 0;
                    shader 5;
                };
                enum sizes: $ic => {
                    type [f32; 2];
                    bind 1;
                    shader 6;
                };
                enum colors: $ic => {
                    type [f32; 4];
                    bind 2;
                    shader 7;
                };
                enum uv: $ic => {
                    type [f32; 4];
                    bind 3;
                    shader 8;
                };
            }
        }

        paste::paste! {
            #[derive(Debug, Default)]
            pub struct InterfaceStorageBuffers(
                pub ethel::render::buffer::PartitionedTriBuffer<4>
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
