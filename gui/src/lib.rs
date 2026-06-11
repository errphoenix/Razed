use ethel::{assets::TextureId, state::data::IndirectIndex};

use janus::StringHash;
#[cfg(feature = "taffy")]
pub use taffy::prelude::*;

#[cfg(not(feature = "taffy"))]
use taffy::prelude::*;

ethel::table_spec! {
    struct InterfaceCommon {
        archetype: ComponentKind;
        anchor: glam::Vec2;
        bounds: Box2d;
        parent: Option<WidgetId>;
        children: Vec<WidgetId>;
    }
}

ethel::table_spec! {
    struct InterfacePanel {
        background_color: glam::Vec3;
        hover_tint: glam::Vec4;
        opacity: f32;
    }
}

ethel::table_spec! {
    struct InterfaceText {
        string: StringHash;
        font_name: StringHash;
        color: glam::Vec4;
        size: u32;
    }
}

ethel::table_spec! {
    struct InterfaceImage {
        tint: glam::Vec4;
        opacity: f32;
        texture: TextureId;
    }
}

ethel::table_spec! {
    struct InterfaceButton {
        label: StringHash;
        font_name: StringHash;
        text_color: glam::Vec3;
        text_size: u32;

        hover_tint: glam::Vec3;
        press_tint: glam::Vec3;

        callback: ButtonCallback;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ButtonCallback(pub fn());

impl ButtonCallback {
    pub const fn new(callback: fn()) -> Self {
        Self(callback)
    }

    pub const fn null() -> Self {
        Self(|| ())
    }
}

impl From<ButtonCallback> for fn() {
    fn from(value: ButtonCallback) -> Self {
        value.0
    }
}

impl Default for ButtonCallback {
    fn default() -> Self {
        Self::null()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WidgetId(pub IndirectIndex);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ComponentKind {
    #[default]
    Null,
    Panel,
    Text,
    Image,
    Button,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Box2d {
    min: glam::Vec2,
    max: glam::Vec2,
}

impl Box2d {
    pub const fn new(min: glam::Vec2, max: glam::Vec2) -> Self {
        Self { min, max }
    }

    pub const fn from_center(center: glam::Vec2, extents: glam::Vec2) -> Self {
        Self {
            min: glam::vec2(center.x - extents.x, center.y - extents.y),
            max: glam::vec2(center.x + extents.x, center.y + extents.y),
        }
    }

    pub const fn min(&self) -> glam::Vec2 {
        self.min
    }

    pub const fn max(&self) -> glam::Vec2 {
        self.max
    }

    pub const fn center(&self) -> glam::Vec2 {
        glam::vec2(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
        )
    }

    pub const fn size(&self) -> glam::Vec2 {
        glam::vec2(self.max.x - self.min.x, self.max.y - self.min.y)
    }

    pub const fn translate(&mut self, translation: glam::Vec2) {
        self.min.x += translation.x;
        self.min.y += translation.y;
        self.max.x += translation.x;
        self.max.y += translation.y;
    }

    pub const fn make_center(&mut self) -> glam::Vec2 {
        let center = self.center();
        self.min.x -= center.x;
        self.min.y -= center.y;
        self.max.x -= center.x;
        self.max.y -= center.y;
        center
    }

    /// See [`Self::scale`].
    pub fn scale_uniform(&mut self, scaling: f32, anchor: Option<glam::Vec2>) {
        self.scale(glam::Vec2::splat(scaling), anchor);
    }

    /// Scale the 2d box by a given `scaling` respective to an `anchor`.
    ///
    /// The `anchor` must be of length [-1, 1]. A `None` value will default to
    /// a (0, 0) point, which is the local center of the box.
    ///
    /// The length of `anchor` is relative to the half size of the box, which
    /// is centered around (0, 0). For example, an `anchor` of (1, 0.5) for a
    /// box of size (400, 200) equals to an `anchor` point of (200, 50).
    pub fn scale(&mut self, scaling: glam::Vec2, anchor: Option<glam::Vec2>) {
        let anchor = anchor.unwrap_or_default() * self.size();
        let offset = self.make_center() - anchor;
        self.translate(anchor);
        self.min.x *= scaling.x;
        self.min.y *= scaling.y;
        self.max.x *= scaling.x;
        self.max.y *= scaling.y;
        self.translate(offset);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeJointId {
    pub table_id: WidgetId,
    #[cfg(feature = "taffy")]
    pub tree_id: NodeId,
}

#[derive(Debug)]
pub struct InterfaceSystem {
    layout: TaffyTree<WidgetId>,
    commons: InterfaceCommonRowTable,
    root_node: NodeJointId,
}

impl InterfaceSystem {
    pub fn new() -> Self {
        let mut layout = TaffyTree::with_capacity(1);
        const ROOT_ID: WidgetId = WidgetId(IndirectIndex::null(0));

        let tree_id = layout
            .new_leaf_with_context(Style::default(), ROOT_ID)
            .unwrap();

        let root_node = NodeJointId {
            table_id: ROOT_ID,
            #[cfg(feature = "taffy")]
            tree_id,
        };

        Self {
            layout,
            root_node,
            commons: InterfaceCommonRowTable::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let mut layout = TaffyTree::with_capacity(capacity + 1);
        const ROOT_ID: WidgetId = WidgetId(IndirectIndex::null(0));

        let tree_id = layout
            .new_leaf_with_context(Style::default(), ROOT_ID)
            .unwrap();

        let root_node = NodeJointId {
            table_id: ROOT_ID,
            #[cfg(feature = "taffy")]
            tree_id,
        };

        Self {
            layout,
            root_node,
            commons: InterfaceCommonRowTable::with_capacity(capacity),
        }
    }

    pub const fn root_id(&self) -> NodeJointId {
        self.root_node
    }

    #[cfg(feature = "taffy")]
    pub const fn taffy_layout(&self) -> &TaffyTree<WidgetId> {
        &self.layout
    }

    pub fn evaluate_layout(&mut self) {}
}
