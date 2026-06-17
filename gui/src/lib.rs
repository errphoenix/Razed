use ethel::{
    assets::TextureId,
    render::Resolution,
    state::data::{Column, DirectIndex, IndirectIndex},
};

use janus::{
    StringHash,
    context::DeltaTime,
    input::{KeyEvent, Keys, MouseButton},
};

pub mod draw;
pub mod style;

pub use style::*;

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
        parent: WidgetId;
        children: Vec<WidgetId>;
        taffy_id: TaffyNodeId;
        layout_style: LayoutStyle;

        // feedback values from taffy tree after evaluation
        feedback_anchor: glam::Vec2; // top left corner
        feedback_bounds: Box2d;

        hovered: bool;
        pressed: bool;
        hover_time: f32;
        press_time: f32;
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
impl std::fmt::Display for WidgetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.as_int())
    }
}
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
impl std::fmt::Display for ComponentKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComponentKind::Null => write!(f, "null"),
            ComponentKind::Panel(_) => write!(f, "panel"),
            ComponentKind::Text(_) => write!(f, "text"),
            ComponentKind::Image(_) => write!(f, "image"),
            ComponentKind::Button { .. } => write!(f, "button"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Box2d {
    min: glam::Vec2,
    max: glam::Vec2,
}
impl Box2d {
    pub const NULL: Self = Box2d::new(glam::Vec2::ZERO, glam::Vec2::ZERO);
    pub const UNIT: Self = Box2d::new(glam::Vec2::ONE, glam::Vec2::ONE);

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

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x > self.min.x && y > self.min.y && x < self.max.x && y < self.max.y
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeJointId {
    pub table_id: WidgetId,
    #[cfg(feature = "taffy")]
    pub tree_id: NodeId,
}

#[derive(Debug, thiserror::Error)]
pub enum WidgetError {
    #[error("archetype '{0}' is already defined")]
    ArchetypeAlreadyDefined(ComponentKind),

    #[error("parent by given ID {0} not found")]
    ParentNotFound(WidgetId),

    #[error("child by given ID {0} not found")]
    ChildNotFound(WidgetId),

    #[error("taffy layout error: {0}")]
    TaffyLayoutError(taffy::TaffyError),

    #[error("invalid widget handle {0}: cannot be resolved")]
    InvalidWidgetHandle(u32),
}

#[derive(Debug)]
pub struct InterfaceSystem {
    layout: TaffyTree<WidgetId>,
    root_node: NodeJointId,

    resolution: Resolution,

    commons: InterfaceCommonRowTable,
    panels: InterfacePanelRowTable,
    texts: InterfaceTextRowTable,
    images: InterfaceImageRowTable,
    buttons: InterfaceButtonRowTable,
}
impl InterfaceSystem {
    pub fn new(resolution: Resolution) -> Self {
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
            resolution,
            commons: InterfaceCommonRowTable::new(),
            panels: InterfacePanelRowTable::new(),
            texts: InterfaceTextRowTable::new(),
            images: InterfaceImageRowTable::new(),
            buttons: InterfaceButtonRowTable::new(),
        }
    }

    pub fn with_capacity(resolution: Resolution, capacity: usize) -> Self {
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
            resolution,
            commons: InterfaceCommonRowTable::with_capacity(capacity),
            panels: InterfacePanelRowTable::with_capacity(capacity),
            texts: InterfaceTextRowTable::with_capacity(capacity),
            images: InterfaceImageRowTable::with_capacity(capacity),
            buttons: InterfaceButtonRowTable::with_capacity(capacity),
        }
    }

    pub const fn resolution(&self) -> Resolution {
        self.resolution
    }

    /// Update the internal `resolution` of the interface layout.
    ///
    /// # Returns
    /// Returns the previously stored `Resolution`.
    pub const fn set_resolution(&mut self, resolution: Resolution) -> Resolution {
        std::mem::replace(&mut self.resolution, resolution)
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

    pub fn process_hover_events(&mut self, x: f32, y: f32, delta: DeltaTime) {
        let delta = delta.as_f32();

        let count = self.commons.len();
        let bounds = &self.commons.feedback_bounds;
        let hovered = &mut self.commons.hovered;
        let hover_time = &mut self.commons.hover_time;

        for i in 1..count {
            let bounds = bounds[i];
            if bounds.contains(x, y) {
                hovered[i] = true;
                hover_time[i] += delta;
            } else {
                hovered[i] = false;
                hover_time[i] = 0f32;
            }
        }
    }

    fn process_key_event(&mut self, table_index: usize, event: KeyEvent, delta: DeltaTime) {
        let delta = delta.as_f32();
        let hovered = &self.commons.hovered;
        let pressed = &mut self.commons.pressed;
        let press_time = &mut self.commons.press_time;

        // mouse press, hold, release
        {
            let click = event.is_mouse() && !event.is_released();
            let press = hovered[table_index] & click;
            let pt0 = press_time[table_index];
            let pt1 = (pt0 + delta) * press as u32 as f32;
            pressed[table_index] = press;
            press_time[table_index] = pt1;
        }

        // keyboard (todo)
        {}
    }

    pub fn feed_key_events(&mut self, events: &[KeyEvent], delta: DeltaTime) {
        let count = self.commons.len();
        for i in 1..count {
            for event in events {
                self.process_key_event(i, *event, delta);
            }
        }
    }

    pub fn process_key_events(&mut self, keys: &Keys, delta: DeltaTime) {
        let click = keys.mouse_down(MouseButton::Left) | keys.mouse_down(MouseButton::Right);
        let delta = delta.as_f32();

        let count = self.commons.len();
        let hovered = &self.commons.hovered;
        let pressed = &mut self.commons.pressed;
        let press_time = &mut self.commons.press_time;

        for i in 1..count {
            let press = hovered[i] & click;
            let pt0 = press_time[i];
            let pt1 = (pt0 + delta) * press as u32 as f32;

            pressed[i] = press;
            press_time[i] = pt1;

            // if hovered[i] {
            //     pressed[i] = click;
            //     if click {
            //         press_time[i] += delta;
            //     } else {
            //         press_time[i] = 0f32;
            //     }
            // } else {
            //     pressed[i] = false;
            //     press_time[i] = 0f32;
            // }
        }
    }

    pub fn synchronise_layout(&mut self) {
        let count = self.commons.len();
        let feedback_anchor = &mut self.commons.feedback_anchor;
        let feedback_bounds = &mut self.commons.feedback_bounds;
        let taffy_ids = &self.commons.taffy_id;

        for i in 1..count {
            let taffy_id = taffy_ids[i];
            if taffy_id.is_null() {
                continue;
            }

            let taffy_id = taffy_id.0;
            let fb_anchor = &mut feedback_anchor[i];
            let fb_bounds = &mut feedback_bounds[i];

            let node = self.layout.get_final_layout(taffy_id);
            let position = node.location;
            let size = node.size;
            let min = glam::vec2(position.x, position.y - size.height);
            let max = glam::vec2(position.x + size.height, position.y);

            *fb_anchor = glam::vec2(position.x, position.y);
            *fb_bounds = Box2d { min, max };
        }
    }

    pub fn evaluate_layout(&mut self) {
        let available = Size {
            width: AvailableSpace::Definite(self.resolution.width),
            height: AvailableSpace::Definite(self.resolution.height),
        };

        self.layout
            .compute_layout(self.root_node.tree_id, available)
            .expect("failed to evaluate taffy layout");
    }

    fn assert_null_archetype(&self, root_id: DirectIndex) -> Result<(), WidgetError> {
        let archetype = self.commons.archetype[root_id.as_index()];
        if matches!(archetype, ComponentKind::Null) {
            Ok(())
        } else {
            Err(WidgetError::ArchetypeAlreadyDefined(archetype))
        }
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
    ) -> Result<IndirectIndex, WidgetError> {
        if let Some(commons_id) = self.commons.solve_indirect(id.0) {
            self.assert_null_archetype(commons_id)?;
            let panel_element = (color, hover_tint, opacity);
            let panel_id = self.panels.insert(panel_element);
            self.commons.archetype[commons_id.as_index()] = ComponentKind::Panel(panel_id);
            Ok(panel_id)
        } else {
            Err(WidgetError::InvalidWidgetHandle(id.0.as_int()))
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
    ) -> Result<IndirectIndex, WidgetError> {
        if let Some(commons_id) = self.commons.solve_indirect(id.0) {
            self.assert_null_archetype(commons_id)?;
            let text_id = self.create_text(string, font_name, color, size);
            self.commons.archetype[commons_id.as_index()] = ComponentKind::Text(text_id);
            Ok(text_id)
        } else {
            Err(WidgetError::InvalidWidgetHandle(id.0.as_int()))
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
    ) -> Result<IndirectIndex, WidgetError> {
        if let Some(commons_id) = self.commons.solve_indirect(id.0) {
            self.assert_null_archetype(commons_id)?;
            let image_element = (tint, opacity, texture);
            let image_id = self.images.insert(image_element);
            self.commons.archetype[commons_id.as_index()] = ComponentKind::Image(image_id);
            Ok(image_id)
        } else {
            Err(WidgetError::InvalidWidgetHandle(id.0.as_int()))
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
    ) -> Result<IndirectIndex, WidgetError> {
        if let Some(commons_id) = self.commons.solve_indirect(root_id.0) {
            self.assert_null_archetype(commons_id)?;
            let button_element = (text_id, hover_tint, press_tint, callback);
            let button_id = self.buttons.insert(button_element);
            self.commons.archetype[commons_id.as_index()] = ComponentKind::Button {
                handle: button_id,
                text_handle: text_id,
            };
            Ok(button_id)
        } else {
            Err(WidgetError::InvalidWidgetHandle(root_id.0.as_int()))
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
        children: Option<&[WidgetId]>,
        layout_style: LayoutStyle,
    ) -> Result<WidgetId, WidgetError> {
        let taffy_style = layout_style.into_taffy_style();
        let node_id = self
            .layout
            .new_leaf(taffy_style)
            .map_err(WidgetError::TaffyLayoutError)?;

        let parent = parent.unwrap_or(self.root_node.table_id);

        // add this to parent
        {
            let parent_direct = self
                .commons
                .solve_indirect(parent.0)
                .ok_or(WidgetError::ParentNotFound(parent))?;
            let parent_taffy_id = self.commons.taffy_id[parent_direct.as_index()];
            self.layout
                .add_child(parent_taffy_id.0, node_id)
                .map_err(WidgetError::TaffyLayoutError)?;
        }
        // add children to this
        let children = if let Some(children) = children {
            children.iter().try_for_each(|child| {
                if let Some(direct) = self.commons.solve_indirect(child.0) {
                    let taffy_id = self.commons.taffy_id[direct.as_index()];

                    self.layout
                        .add_child(node_id, taffy_id.0)
                        .map_err(WidgetError::TaffyLayoutError)
                } else {
                    Err(WidgetError::ChildNotFound(*child))
                }
            })?;
            children.to_vec()
        } else {
            // does not allocate
            Vec::new()
        };

        Ok(WidgetId(self.commons.insert((
            ComponentKind::Null,
            parent,
            children,
            TaffyNodeId(node_id),
            layout_style,
            // init default feedback values
            glam::Vec2::ZERO,
            Box2d::NULL,
            false,
            false,
            0f32,
            0f32,
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
}
