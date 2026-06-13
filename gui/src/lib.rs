use ethel::{
    assets::TextureId,
    state::data::{Column, IndirectIndex},
};

use janus::StringHash;
#[cfg(feature = "taffy")]
pub use taffy::prelude::*;

#[cfg(not(feature = "taffy"))]
use taffy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct TaffyNodeId(pub(crate) NodeId);

impl TaffyNodeId {
    pub fn is_null(self) -> bool {
        self.0 == NodeId::new(0)
    }
}

impl Default for TaffyNodeId {
    fn default() -> Self {
        Self(NodeId::new(0))
    }
}

ethel::table_spec! {
    struct InterfaceCommon {
        archetype: ComponentKind;
        anchor: glam::Vec2;
        bounds: Box2d;
        parent: WidgetId;
        children: Vec<WidgetId>;
        taffy_id: TaffyNodeId;
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
        text_id: IndirectIndex;

        hover_tint: glam::Vec4;
        press_tint: glam::Vec4;

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

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WidgetId(pub IndirectIndex);

impl WidgetId {
    pub const fn new(index: IndirectIndex) -> Self {
        Self(index)
    }

    pub const fn is_null(self) -> bool {
        self.0.as_int() == 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ComponentKind {
    #[default]
    Null,
    Panel(IndirectIndex),
    Text(IndirectIndex),
    Image(IndirectIndex),
    Button {
        handle: IndirectIndex,
        text_handle: IndirectIndex,
    },
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
    root_node: NodeJointId,

    commons: InterfaceCommonRowTable,
    panels: InterfacePanelRowTable,
    texts: InterfaceTextRowTable,
    images: InterfaceImageRowTable,
    buttons: InterfaceButtonRowTable,
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
            panels: InterfacePanelRowTable::new(),
            texts: InterfaceTextRowTable::new(),
            images: InterfaceImageRowTable::new(),
            buttons: InterfaceButtonRowTable::new(),
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
            panels: InterfacePanelRowTable::with_capacity(capacity),
            texts: InterfaceTextRowTable::with_capacity(capacity),
            images: InterfaceImageRowTable::with_capacity(capacity),
            buttons: InterfaceButtonRowTable::with_capacity(capacity),
        }
    }

    pub const fn root_id(&self) -> NodeJointId {
        self.root_node
    }

    #[cfg(feature = "taffy")]
    pub const fn taffy_layout(&self) -> &TaffyTree<WidgetId> {
        &self.layout
    }

    pub fn parent_of(&self, widget: WidgetId) -> Option<WidgetId> {
        if let Some(direct) = self.commons.solve_indirect(widget.0) {
            self.commons.parent.get(direct.as_index()).copied()
        } else {
            None
        }
    }

    pub fn has_parent(&self, widget: WidgetId) -> Option<bool> {
        self.parent_of(widget).map(|id| !id.is_null())
    }

    /// Make a core element into a panel element.
    ///
    /// This requires the core element's `id`.
    ///
    /// # Returns
    /// Returns the `IndirectIndex` of the panel data.
    /// Or returns `None` if the given core `id` is invalid.
    pub fn make_panel(
        &mut self,
        id: WidgetId,
        color: glam::Vec3,
        hover_tint: glam::Vec4,
        opacity: f32,
    ) -> Option<IndirectIndex> {
        if let Some(commons_id) = self.commons.solve_indirect(id.0) {
            let panel_element = (color, hover_tint, opacity);
            let panel_id = self.panels.insert(panel_element);
            self.commons.archetype[commons_id.as_index()] = ComponentKind::Panel(panel_id);
            Some(panel_id)
        } else {
            None
        }
    }

    /// Make a core element into a text element.
    ///
    /// This requires the core element's `id`.
    ///
    /// # Returns
    /// Returns the `IndirectIndex` of the text data.
    /// Or returns `None` if the given core `id` is invalid.
    pub fn make_text(
        &mut self,
        id: WidgetId,
        string: StringHash,
        font_name: StringHash,
        color: glam::Vec4,
        size: u32,
    ) -> Option<IndirectIndex> {
        if let Some(commons_id) = self.commons.solve_indirect(id.0) {
            let text_id = self.create_text(string, font_name, color, size);
            self.commons.archetype[commons_id.as_index()] = ComponentKind::Text(text_id);
            Some(text_id)
        } else {
            None
        }
    }

    /// Make a core element into an image element.
    ///
    /// This requires the core element's `id`.
    ///
    /// # Returns
    /// Returns the `IndirectIndex` of the image data.
    /// Or returns `None` if the given core `id` is invalid.
    pub fn make_image(
        &mut self,
        id: WidgetId,
        tint: glam::Vec4,
        opacity: f32,
        texture: TextureId,
    ) -> Option<IndirectIndex> {
        if let Some(commons_id) = self.commons.solve_indirect(id.0) {
            let image_element = (tint, opacity, texture);
            let image_id = self.images.insert(image_element);
            self.commons.archetype[commons_id.as_index()] = ComponentKind::Image(image_id);
            Some(image_id)
        } else {
            None
        }
    }

    /// Make a core element into a button element.
    ///
    /// This, other than the core element's `root_id`, also requires the
    /// `text_id` of a text element, created with [`Self::create_text`].
    ///
    /// # Returns
    /// Returns the `IndirectIndex` of the button data.
    /// Or returns `None` if the given core `root_id` is invalid.
    pub fn make_button(
        &mut self,
        root_id: WidgetId,
        text_id: IndirectIndex,
        hover_tint: glam::Vec4,
        press_tint: glam::Vec4,
        callback: ButtonCallback,
    ) -> Option<IndirectIndex> {
        if let Some(commons_id) = self.commons.solve_indirect(root_id.0) {
            let button_element = (text_id, hover_tint, press_tint, callback);
            let button_id = self.buttons.insert(button_element);
            self.commons.archetype[commons_id.as_index()] = ComponentKind::Button {
                handle: button_id,
                text_handle: text_id,
            };
            Some(button_id)
        } else {
            None
        }
    }

    /// Create a new empty or core element.
    ///
    /// This returns a new [`WidgetId`] which can be used to create
    /// specific element types through [`Self::make_panel`],
    /// [`Self::make_text`], [`Self::make_image`], or [`Self::make_button`].
    pub fn add_new(
        &mut self,
        parent: Option<WidgetId>,
        anchor: glam::Vec2,
        bounds: Box2d,
        children: Option<&[WidgetId]>,
    ) -> WidgetId {
        let children = if let Some(children) = children {
            children.to_vec()
        } else {
            // does not allocate
            Vec::new()
        };

        let parent = parent.unwrap_or_default();

        Ok(WidgetId(self.commons.insert((
            ComponentKind::Null,
            anchor,
            bounds,
            parent,
            children,
            TaffyNodeId(node_id),
        ))))
    }

    /// Create a new text element without direct associations to the widget
    /// tree.
    ///
    /// This text element can then be used for a button element with
    /// [`Self::make_button`].
    pub fn create_text(
        &mut self,
        string: StringHash,
        font_name: StringHash,
        color: glam::Vec4,
        size: u32,
    ) -> IndirectIndex {
        let text_element = (string, font_name, color, size);
        self.texts.insert(text_element)
    }

    pub fn core_data(&self) -> &InterfaceCommonRowTable {
        &self.commons
    }

    pub fn core_data_mut(&mut self) -> &mut InterfaceCommonRowTable {
        &mut self.commons
    }

    pub fn panel_data(&self) -> &InterfacePanelRowTable {
        &self.panels
    }

    pub fn panel_data_mut(&mut self) -> &mut InterfacePanelRowTable {
        &mut self.panels
    }

    pub fn image_data(&self) -> &InterfaceImageRowTable {
        &self.images
    }

    pub fn image_data_mut(&mut self) -> &mut InterfaceImageRowTable {
        &mut self.images
    }

    pub fn text_data(&self) -> &InterfaceTextRowTable {
        &self.texts
    }

    pub fn text_data_mut(&mut self) -> &mut InterfaceTextRowTable {
        &mut self.texts
    }

    pub fn button_data(&self) -> &InterfaceButtonRowTable {
        &self.buttons
    }

    pub fn button_data_mut(&mut self) -> &mut InterfaceButtonRowTable {
        &mut self.buttons
    }

    pub fn evaluate_layout(&mut self) {}
}
