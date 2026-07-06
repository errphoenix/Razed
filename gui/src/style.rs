use taffy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum ItemAlignment {
    #[default]
    Auto,

    Start,
    Center,
    End,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Wrap {
    #[default]
    DontWrap,
    Wrap,
    Reverse,
}
impl From<Wrap> for FlexWrap {
    fn from(value: Wrap) -> Self {
        match value {
            Wrap::DontWrap => Self::NoWrap,
            Wrap::Wrap => Self::Wrap,
            Wrap::Reverse => Self::WrapReverse,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum GridFlow {
    #[default]
    Row,
    Column,
    RowDense,
    ColumnDense,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
    RowReverse,
    ColumnReverse,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    Clip,
    Scroll,
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

    pub const fn splat(value: Value) -> Self {
        Self {
            left: value,
            top: value,
            right: value,
            bottom: value,
        }
    }

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

#[derive(Clone, Copy, Debug, Default)]
pub enum ContainerLayout {
    Flexbox {
        direction: FlexDirection,
        wrap: Wrap,
        justify_content: ContentAlignment,
        align_content: ContentAlignment,
        align_items: ItemAlignment,
    },
    Grid {
        template_rows: u16,
        template_columns: u16,
        gap: Point,
        flow: GridFlow,
        justify_content: ContentAlignment,
        align_content: ContentAlignment,
        align_items: ItemAlignment,
        justify_items: ItemAlignment,
    },
    #[default]
    Block,
    None,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum LayoutPosition {
    #[default]
    Relative,
    Absolute {
        x: Option<Value>,
        y: Option<Value>,
    },
}

#[derive(Clone, Debug, Default)]
pub struct LayoutOptions {
    pub container: ContainerLayout,

    pub align_self: ItemAlignment,
    pub justify_self: ItemAlignment,

    pub layout_position: LayoutPosition,
    pub size: Option<Point>,
    pub min_size: Option<Point>,
    pub max_size: Option<Point>,
    pub aspect_ratio: Option<f32>,

    pub overflow_x: Overflow,
    pub overflow_y: Overflow,

    pub margin: Option<Rectangle>,
    pub padding: Option<Rectangle>,

    pub grid_row: GridLine,
    pub grid_column: GridLine,
}
impl LayoutOptions {
    pub const fn new() -> Self {
        Self {
            container: ContainerLayout::Block,
            align_self: ItemAlignment::Center,
            justify_self: ItemAlignment::Center,
            layout_position: LayoutPosition::Relative,
            size: None,
            min_size: None,
            max_size: None,
            aspect_ratio: None,
            overflow_x: Overflow::Clip,
            overflow_y: Overflow::Clip,
            margin: None,
            padding: None,
            grid_row: GridLine::Single(0),
            grid_column: GridLine::Single(0),
        }
    }

    pub(crate) fn into_taffy_style(&self) -> Style {
        let size = self.size.map_or(Size::auto(), |size| size.into());
        let min_size = self.min_size.map_or(Size::auto(), |size| size.into());
        let max_size = self.max_size.map_or(Size::auto(), |size| size.into());
        let aspect_ratio = self.aspect_ratio;
        let margin = self.margin.map_or(taffy::Rect::zero(), |mrg| mrg.into());
        let padding = self.padding.map_or(taffy::Rect::zero(), |pag| pag.into());
        let overflow = taffy::Point {
            x: self.overflow_x.into(),
            y: self.overflow_y.into(),
        };
        let (position, inset) = match self.layout_position {
            LayoutPosition::Relative => (Position::Relative, Rect::auto()),
            LayoutPosition::Absolute { x, y } => (
                Position::Absolute,
                Rect {
                    left: x.map_or(LengthPercentageAuto::auto(), Value::into),
                    top: y.map_or(LengthPercentageAuto::auto(), Value::into),
                    right: auto(),
                    bottom: auto(),
                },
            ),
        };
        let grid_row = self.grid_row.into();
        let grid_column = self.grid_column.into();
        let justify_self = self.justify_self.try_into().ok();
        let align_self = self.align_self.try_into().ok();

        let mut display = Display::Block;
        let mut flex_direction = Default::default();
        let mut flex_wrap = Default::default();
        let mut justify_content = Default::default();
        let mut align_content = Default::default();
        let mut justify_items = Default::default();
        let mut align_items = Default::default();
        let mut template_rows = Default::default();
        let mut template_columns = Default::default();
        let mut gap = Size::zero();
        let mut grid_flow = Default::default();

        match self.container {
            ContainerLayout::Flexbox {
                direction: f_direction,
                wrap: f_wrap,
                justify_content: f_justify_content,
                align_content: f_align_content,
                align_items: f_align_items,
            } => {
                display = Display::Flex;
                flex_direction = f_direction.into();
                flex_wrap = f_wrap.into();
                justify_content = f_justify_content.try_into().ok();
                align_content = f_align_content.try_into().ok();
                align_items = f_align_items.try_into().ok();
            }
            ContainerLayout::Grid {
                template_rows: g_template_rows,
                template_columns: g_template_columns,
                gap: g_gap,
                flow: g_flow,
                justify_content: g_justify_content,
                align_content: g_align_content,
                align_items: g_align_items,
                justify_items: g_justify_items,
            } => {
                display = Display::Grid;
                template_rows = taffy::style_helpers::evenly_sized_tracks(g_template_rows);
                template_columns = taffy::style_helpers::evenly_sized_tracks(g_template_columns);
                gap = g_gap.into();
                grid_flow = g_flow.into();
                justify_content = g_justify_content.try_into().ok();
                align_content = g_align_content.try_into().ok();
                align_items = g_align_items.try_into().ok();
                justify_items = g_justify_items.try_into().ok();
            }
            ContainerLayout::None => {
                display = Display::None;
            }
            ContainerLayout::Block => {
                // already set
            }
        };

        Style {
            display,
            overflow,
            // todo: scrollbar_width: (),
            position,
            inset,
            size,
            min_size,
            max_size,
            aspect_ratio,
            margin,
            padding,
            align_items,
            align_self,
            justify_items,
            justify_self,
            align_content,
            justify_content,
            gap,
            flex_direction,
            flex_wrap,
            grid_row,
            grid_column,
            grid_template_rows: template_rows,
            grid_template_columns: template_columns,
            grid_auto_flow: grid_flow,
            ..Default::default()
        }
    }
}
