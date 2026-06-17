use std::vec::Drain;

use ethel::{
    assets::TextureId,
    render::command::DrawGroups,
    state::data::{IndirectIndex, table::TableView},
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
                crate::ComponentKind::Null => continue,
                crate::ComponentKind::Panel(indirect_index) => {
                    self.gather_panel(index, indirect_index)
                }
                crate::ComponentKind::Image(indirect_index) => {
                    self.gather_image(index, indirect_index)
                }
                crate::ComponentKind::Button {
                    handle,
                    text_handle: _text_handle,
                } => self.gather_button(index, handle),
                crate::ComponentKind::Text(_indirect_index) => {
                    // todo
                    continue;
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

        QuadElement {
            color,
            attachment: None,
        }
    }

    fn gather_text(&self, common_handle: usize, text_index: IndirectIndex) -> QuadElement {
        todo!()
    }

    fn gather_image(&self, common_handle: usize, image_index: IndirectIndex) -> QuadElement {
        todo!()
    }

    fn gather_button(&self, common_handle: usize, button_index: IndirectIndex) -> QuadElement {
        todo!()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuadElement {
    pub color: glam::Vec4,
    pub attachment: Option<InterfaceAttachment>,
}
impl QuadElement {
    pub const fn new(color: glam::Vec4, attachment: Option<InterfaceAttachment>) -> Self {
        Self { color, attachment }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InterfaceAttachment {
    pub back_handle: usize,
    pub object: AttachedObject,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AttachedObject {
    Texture(TextureId),
}
impl AttachedObject {
    /// Indicates where the attached object must be drawn in relation to the
    /// root.
    pub const fn layer_ordering(&self) -> LayerOrdering {
        match self {
            AttachedObject::Texture(_) => LayerOrdering::Under,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LayerOrdering {
    /// Over the original root element, one layer up.
    Over,
    /// Under the original root element, one layer down.
    Under,
}
