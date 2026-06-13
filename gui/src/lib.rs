use ethel::{
    assets::TextureId,
    render::Resolution,
    state::data::{Column, DirectIndex, IndirectIndex},
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum ItemAlignment {
    #[default]
    Auto,

    Start,
    Center,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum ContentAlignment {
    #[default]
    Auto,

    Start,
    Center,
    End,

    SpaceEvenly,
    SpaceAround,
    SpaceBetween,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Wrap {
    #[default]
    DontWrap,
    Wrap,
    Reverse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum GridFlow {
    #[default]
    Row,
    Column,
    RowDense,
    ColumnDense,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    Clip,
    Scroll,
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum Value {
    Absolute(f32),
    Percentage(f32),
}
impl Default for Value {
    fn default() -> Self {
        Self::ZERO
    }
}
impl Value {
    pub const ZERO: Self = Self::Absolute(0f32);
    pub const ONE: Self = Self::Absolute(1f32);
    pub const MAX: Self = Self::Percentage(1f32);
    pub const HALF: Self = Self::Percentage(0.5);
    pub const QUARTER: Self = Self::Percentage(0.25);
    pub const THREE_QUARTERS: Self = Self::Percentage(0.75);
    pub const FIFTH: Self = Self::Percentage(0.2);
    pub const THIRD: Self = Self::Percentage(0.3334);
}
impl From<Value> for LengthPercentage {
    fn from(value: Value) -> Self {
        match value {
            Value::Absolute(length) => LengthPercentage::length(length),
            Value::Percentage(percent) => LengthPercentage::percent(percent),
        }
    }
}
impl From<Value> for LengthPercentageAuto {
    fn from(value: Value) -> Self {
        match value {
            Value::Absolute(length) => LengthPercentageAuto::length(length),
            Value::Percentage(percent) => LengthPercentageAuto::percent(percent),
        }
    }
}
impl From<Value> for Dimension {
    fn from(value: Value) -> Self {
        match value {
            Value::Absolute(length) => Dimension::length(length),
            Value::Percentage(percent) => Dimension::percent(percent),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Default)]
pub struct Point {
    pub x: Value,
    pub y: Value,
}
impl Point {
    pub const ZERO: Self = Self::new(Value::ZERO, Value::ZERO);
    pub const ONE: Self = Self::new(Value::ONE, Value::ONE);
    pub const MAX: Self = Self::new(Value::MAX, Value::MAX);
    pub const HALF: Self = Self::new(Value::HALF, Value::HALF);

    pub const fn new(x: Value, y: Value) -> Self {
        Self { x, y }
    }
}
impl<T: From<Value>> From<Point> for Size<T> {
    fn from(value: Point) -> Self {
        Size {
            width: value.x.into(),
            height: value.y.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Default)]
pub struct Rectangle {
    pub left: Value,
    pub top: Value,
    pub right: Value,
    pub bottom: Value,
}
impl Rectangle {
    pub const ZERO: Self = Self::new(Value::ZERO, Value::ZERO, Value::ZERO, Value::ZERO);
    pub const ONE: Self = Self::new(Value::ONE, Value::ONE, Value::ONE, Value::ONE);
    pub const MAX: Self = Self::new(Value::MAX, Value::MAX, Value::MAX, Value::MAX);
    pub const HALF: Self = Self::new(Value::HALF, Value::HALF, Value::HALF, Value::HALF);

    pub const fn new(left: Value, top: Value, right: Value, bottom: Value) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }
}
impl<T: From<Value>> From<Rectangle> for Rect<T> {
    fn from(value: Rectangle) -> Self {
        Rect {
            left: value.left.into(),
            top: value.top.into(),
            right: value.right.into(),
            bottom: value.bottom.into(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CommonLayoutOptions {
    pub size: Option<Point>,
    pub min_size: Option<Point>,
    pub max_size: Option<Point>,
    pub aspect_ratio: Option<f32>,

    pub overflow_x: Overflow,
    pub overflow_y: Overflow,

    pub margin: Option<Rectangle>,
    pub padding: Option<Rectangle>,
}

#[derive(Clone, Debug, Default)]
pub enum LayoutStyle {
    #[default]
    Null,

    Block {
        inset: Option<Rectangle>,
        position: Option<Point>,
        common: CommonLayoutOptions,
    },

    Flexbox {
        direction: FlexDirection,
        justify_content: ContentAlignment,
        align_content: ContentAlignment,
        justify_self: ItemAlignment,
        align_self: ItemAlignment,

        common: CommonLayoutOptions,
    },

    Grid {
        template_rows: u16,
        template_columns: u16,
        item_row: GridLine,
        item_column: GridLine,
        flow: GridFlow,

        justify_content: ContentAlignment,
        align_content: ContentAlignment,
        justify_self: ItemAlignment,
        align_self: ItemAlignment,

        common: CommonLayoutOptions,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GridLine {
    /// A value of 0 equals to `auto`
    Single(i16),
    Span(u16),
}

impl Default for GridLine {
    fn default() -> Self {
        Self::Single(0)
    }
}

impl<S: taffy::CheapCloneStr> From<GridLine> for taffy::Line<GridPlacement<S>> {
    fn from(value: GridLine) -> Self {
        match value {
            GridLine::Single(index) => taffy::style_helpers::line(index),
            GridLine::Span(length) => taffy::style_helpers::span(length),
        }
    }
}

impl From<FlexDirection> for taffy::FlexDirection {
    fn from(value: FlexDirection) -> Self {
        match value {
            FlexDirection::Row => taffy::FlexDirection::Row,
            FlexDirection::Column => taffy::FlexDirection::Column,
            FlexDirection::RowReverse => taffy::FlexDirection::RowReverse,
            FlexDirection::ColumnReverse => taffy::FlexDirection::ColumnReverse,
        }
    }
}

impl From<Overflow> for taffy::Overflow {
    fn from(value: Overflow) -> Self {
        match value {
            Overflow::Visible => taffy::Overflow::Visible,
            Overflow::Hidden => taffy::Overflow::Hidden,
            Overflow::Clip => taffy::Overflow::Clip,
            Overflow::Scroll => taffy::Overflow::Scroll,
        }
    }
}

impl From<GridFlow> for taffy::GridAutoFlow {
    fn from(value: GridFlow) -> Self {
        match value {
            GridFlow::Row => taffy::GridAutoFlow::Row,
            GridFlow::Column => taffy::GridAutoFlow::Column,
            GridFlow::RowDense => taffy::GridAutoFlow::RowDense,
            GridFlow::ColumnDense => taffy::GridAutoFlow::ColumnDense,
        }
    }
}

impl TryFrom<ItemAlignment> for taffy::AlignItems {
    type Error = ();

    fn try_from(value: ItemAlignment) -> Result<Self, ()> {
        match value {
            ItemAlignment::Auto => Err(()),
            ItemAlignment::Start => Ok(taffy::AlignItems::Start),
            ItemAlignment::Center => Ok(taffy::AlignItems::Center),
            ItemAlignment::End => Ok(taffy::AlignItems::End),
        }
    }
}

impl TryFrom<ContentAlignment> for taffy::AlignContent {
    type Error = ();

    fn try_from(value: ContentAlignment) -> Result<Self, ()> {
        match value {
            ContentAlignment::Auto => Err(()),
            ContentAlignment::Start => Ok(taffy::AlignContent::Start),
            ContentAlignment::Center => Ok(taffy::AlignContent::Center),
            ContentAlignment::End => Ok(taffy::AlignContent::End),
            ContentAlignment::SpaceEvenly => Ok(taffy::AlignContent::SpaceEvenly),
            ContentAlignment::SpaceAround => Ok(taffy::AlignContent::SpaceAround),
            ContentAlignment::SpaceBetween => Ok(taffy::AlignContent::SpaceBetween),
        }
    }
}

impl LayoutStyle {
    pub(crate) fn into_taffy_style(self) -> Style {
        match self {
            LayoutStyle::Null => Style {
                display: Display::None,
                ..Default::default()
            },
            LayoutStyle::Block {
                position,
                inset,
                common:
                    CommonLayoutOptions {
                        size,
                        min_size,
                        max_size,
                        aspect_ratio,
                        overflow_x,
                        overflow_y,
                        margin,
                        padding,
                    },
            } => {
                let size = size.map_or(Size::auto(), |size| size.into());
                let min_size = min_size.map_or(Size::auto(), |size| size.into());
                let max_size = max_size.map_or(Size::auto(), |size| size.into());
                let margin = margin.map_or(taffy::Rect::auto(), |margin| margin.into());
                let padding = padding.map_or(taffy::Rect::zero(), |padding| padding.into());
                let inset = inset.map_or(taffy::Rect::auto(), |inset| inset.into());

                taffy::Style {
                    display: taffy::Display::Block,
                    position: taffy::Position::Absolute,

                    inset,
                    size,
                    min_size,
                    max_size,
                    aspect_ratio,
                    margin,
                    padding,

                    overflow: taffy::Point {
                        x: overflow_x.into(),
                        y: overflow_y.into(),
                    },

                    ..Default::default()
                }
            }
            LayoutStyle::Flexbox {
                direction,
                justify_content,
                justify_self,
                align_content,
                align_self,
                common:
                    CommonLayoutOptions {
                        size,
                        min_size,
                        max_size,
                        aspect_ratio,
                        overflow_x,
                        overflow_y,
                        margin,
                        padding,
                    },
            } => {
                let size = size.map_or(Size::auto(), |size| size.into());
                let min_size = min_size.map_or(Size::auto(), |size| size.into());
                let max_size = max_size.map_or(Size::auto(), |size| size.into());
                let margin = margin.map_or(taffy::Rect::auto(), |margin| margin.into());
                let padding = padding.map_or(taffy::Rect::zero(), |padding| padding.into());

                Style {
                    display: taffy::Display::Flex,
                    position: taffy::Position::Relative,

                    flex_direction: direction.into(),
                    justify_content: justify_content.try_into().ok(),
                    align_content: align_content.try_into().ok(),
                    justify_self: justify_self.try_into().ok(),
                    align_self: align_self.try_into().ok(),

                    size,
                    min_size,
                    max_size,
                    aspect_ratio,
                    margin,
                    padding,

                    overflow: taffy::Point {
                        x: overflow_x.into(),
                        y: overflow_y.into(),
                    },

                    ..Default::default()
                }
            }
            LayoutStyle::Grid {
                template_rows,
                template_columns,
                item_row,
                item_column,
                flow,
                justify_content,
                align_content,
                justify_self,
                align_self,
                common:
                    CommonLayoutOptions {
                        size,
                        min_size,
                        max_size,
                        aspect_ratio,
                        overflow_x,
                        overflow_y,
                        margin,
                        padding,
                    },
            } => {
                let size = size.map_or(Size::auto(), |size| size.into());
                let min_size = min_size.map_or(Size::auto(), |size| size.into());
                let max_size = max_size.map_or(Size::auto(), |size| size.into());
                let margin = margin.map_or(taffy::Rect::auto(), |margin| margin.into());
                let padding = padding.map_or(taffy::Rect::zero(), |padding| padding.into());

                let grid_template_rows = taffy::style_helpers::evenly_sized_tracks(template_rows);
                let grid_template_columns =
                    taffy::style_helpers::evenly_sized_tracks(template_columns);

                Style {
                    display: taffy::Display::Flex,
                    position: taffy::Position::Relative,

                    grid_template_rows,
                    grid_template_columns,
                    grid_row: item_row.into(),
                    grid_column: item_column.into(),
                    grid_auto_flow: flow.into(),

                    justify_content: justify_content.try_into().ok(),
                    align_content: align_content.try_into().ok(),
                    justify_self: justify_self.try_into().ok(),
                    align_self: align_self.try_into().ok(),

                    size,
                    min_size,
                    max_size,
                    aspect_ratio,
                    margin,
                    padding,

                    overflow: taffy::Point {
                        x: overflow_x.into(),
                        y: overflow_y.into(),
                    },

                    ..Default::default()
                }
            }
        }
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
        feedback_anchor: glam::Vec2;
        feedback_bounds: Box2d;
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

    pub fn evaluate_layout(&mut self) {
        let available = Size::MAX_CONTENT;
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
        let node_id = self
            .layout
            .new_leaf(Style::DEFAULT)
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
