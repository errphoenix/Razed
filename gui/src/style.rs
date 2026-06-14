use taffy::{Dimension, GridPlacement, LengthPercentage, LengthPercentageAuto, Rect, Size, Style};

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

impl LayoutStyle {
    pub(crate) fn into_taffy_style(&self) -> Style {
        match self {
            LayoutStyle::Null => Style {
                display: taffy::Display::None,
                ..Default::default()
            },
            LayoutStyle::Block {
                // todo
                #[allow(unused)]
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

                Style {
                    display: taffy::Display::Block,
                    position: taffy::Position::Absolute,

                    inset,
                    size,
                    min_size,
                    max_size,
                    margin,
                    padding,

                    aspect_ratio: *aspect_ratio,
                    overflow: taffy::Point {
                        x: (*overflow_x).into(),
                        y: (*overflow_y).into(),
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

                    flex_direction: (*direction).into(),
                    justify_content: (*justify_content).try_into().ok(),
                    align_content: (*align_content).try_into().ok(),
                    justify_self: (*justify_self).try_into().ok(),
                    align_self: (*align_self).try_into().ok(),

                    size,
                    min_size,
                    max_size,
                    margin,
                    padding,

                    aspect_ratio: *aspect_ratio,
                    overflow: taffy::Point {
                        x: (*overflow_x).into(),
                        y: (*overflow_y).into(),
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

                let grid_template_rows = taffy::style_helpers::evenly_sized_tracks(*template_rows);
                let grid_template_columns =
                    taffy::style_helpers::evenly_sized_tracks(*template_columns);

                Style {
                    display: taffy::Display::Flex,
                    position: taffy::Position::Relative,

                    grid_template_rows,
                    grid_template_columns,
                    grid_row: (*item_row).into(),
                    grid_column: (*item_column).into(),
                    grid_auto_flow: (*flow).into(),

                    justify_content: (*justify_content).try_into().ok(),
                    align_content: (*align_content).try_into().ok(),
                    justify_self: (*justify_self).try_into().ok(),
                    align_self: (*align_self).try_into().ok(),

                    size,
                    min_size,
                    max_size,
                    margin,
                    padding,

                    aspect_ratio: *aspect_ratio,
                    overflow: taffy::Point {
                        x: (*overflow_x).into(),
                        y: (*overflow_y).into(),
                    },

                    ..Default::default()
                }
            }
        }
    }
}
