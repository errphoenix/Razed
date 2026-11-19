use taffy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum ItemAlignment {
    Auto,
    #[default]
    Stretch,
    Baseline,

    Start,
    Center,
    End,
}
impl TryFrom<ItemAlignment> for taffy::AlignItems {
    type Error = ();

    fn try_from(value: ItemAlignment) -> Result<Self, ()> {
        match value {
            ItemAlignment::Auto => Err(()),
            ItemAlignment::Stretch => Ok(taffy::AlignItems::Stretch),
            ItemAlignment::Baseline => Ok(taffy::AlignItems::Baseline),
            ItemAlignment::Start => Ok(taffy::AlignItems::Start),
            ItemAlignment::Center => Ok(taffy::AlignItems::Center),
            ItemAlignment::End => Ok(taffy::AlignItems::End),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum ContentAlignment {
    Auto,
    #[default]
    Stretch,

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
            ContentAlignment::Stretch => Ok(taffy::AlignContent::Stretch),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Wrap {
    DontWrap,
    #[default]
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
    Row,
    #[default]
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

    pub fn sum(self, other: impl Into<Self>, scaling_reference: f32) -> Self {
        crate::declare::sum_values(self, other.into(), scaling_reference)
    }

    pub const fn px(px: f32) -> Self {
        Self::Absolute(px)
    }

    pub const fn perc(perc: f32) -> Self {
        Self::Percentage(perc)
    }

    pub const fn absolute(px: f32) -> Self {
        Self::Absolute(px)
    }

    pub const fn relative(perc: f32) -> Self {
        Self::Percentage(perc)
    }

    pub const fn to_pixels(self, scaling_reference: f32) -> f32 {
        self.try_to_pixels(Some(scaling_reference)).unwrap()
    }

    /// Returns `None` only if `self` is [`Value::Percentage`] and no
    /// `scaling_reference` is given.
    pub const fn try_to_pixels(self, scaling_reference: Option<f32>) -> Option<f32> {
        match self {
            Value::Absolute(px) => Some(px),
            Value::Percentage(perc) => {
                if let Some(scaling) = scaling_reference {
                    Some(perc * scaling)
                } else {
                    None
                }
            }
        }
    }
}
impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Self::Absolute(value)
    }
}
impl std::ops::Mul<f32> for Value {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        match self {
            Self::Absolute(px) => Self::Absolute(px * rhs),
            Self::Percentage(perc) => Self::Absolute(perc * rhs),
        }
    }
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

    pub const fn px(x: f32, y: f32) -> Self {
        Self::new(Value::px(x), Value::px(y))
    }

    pub const fn perc(x: f32, y: f32) -> Self {
        Self::new(Value::perc(x), Value::perc(y))
    }

    pub const fn splat(v: Value) -> Self {
        Self::new(v, v)
    }

    pub const fn splat_px(px: f32) -> Self {
        Self::splat(Value::px(px))
    }

    pub const fn splat_perc(perc: f32) -> Self {
        Self::splat(Value::perc(perc))
    }
}
impl From<f32> for Point {
    fn from(value: f32) -> Self {
        Self::px(value, value)
    }
}
impl From<Value> for Point {
    fn from(value: Value) -> Self {
        Self::new(value, value)
    }
}
impl From<glam::Vec2> for Point {
    fn from(value: glam::Vec2) -> Self {
        Self::px(value.x, value.y)
    }
}
impl From<(f32, f32)> for Point {
    fn from(value: (f32, f32)) -> Self {
        Self::px(value.0, value.1)
    }
}
impl From<[f32; 2]> for Point {
    fn from(value: [f32; 2]) -> Self {
        Self::px(value[0], value[1])
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

    pub const fn px(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left: Value::px(left),
            top: Value::px(top),
            right: Value::px(right),
            bottom: Value::px(bottom),
        }
    }

    pub const fn perc(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left: Value::perc(left),
            top: Value::perc(top),
            right: Value::perc(right),
            bottom: Value::perc(bottom),
        }
    }

    pub const fn splat(value: Value) -> Self {
        Self {
            left: value,
            top: value,
            right: value,
            bottom: value,
        }
    }

    pub const fn splat_px(px: f32) -> Self {
        Self::splat(Value::px(px))
    }

    pub const fn splat_perc(perc: f32) -> Self {
        Self::splat(Value::perc(perc))
    }

    pub const fn new(left: Value, top: Value, right: Value, bottom: Value) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub const fn even(horiz: Value, vert: Value) -> Self {
        Self::new(horiz, vert, horiz, vert)
    }

    pub const fn even_px(horiz: f32, vert: f32) -> Self {
        Self::even(Value::px(horiz), Value::px(vert))
    }

    pub const fn even_perc(horiz: f32, vert: f32) -> Self {
        Self::even(Value::perc(horiz), Value::perc(vert))
    }
}
impl From<f32> for Rectangle {
    fn from(value: f32) -> Self {
        Self::splat_px(value)
    }
}
impl From<Value> for Rectangle {
    fn from(value: Value) -> Self {
        Self::splat(value)
    }
}
impl From<(f32, f32)> for Rectangle {
    fn from((horiz, vert): (f32, f32)) -> Self {
        Self::even_px(horiz, vert)
    }
}
impl From<[f32; 2]> for Rectangle {
    fn from([horiz, vert]: [f32; 2]) -> Self {
        Self::even_px(horiz, vert)
    }
}
impl From<glam::Vec2> for Rectangle {
    fn from(p: glam::Vec2) -> Self {
        Self::even_px(p.x, p.y)
    }
}
impl From<(Value, Value)> for Rectangle {
    fn from((horiz, vert): (Value, Value)) -> Self {
        Self::even(horiz, vert)
    }
}
impl From<Point> for Rectangle {
    fn from(value: Point) -> Self {
        Self::even(value.x, value.y)
    }
}
impl From<(f32, f32, f32, f32)> for Rectangle {
    fn from((left, top, right, bottom): (f32, f32, f32, f32)) -> Self {
        Self::px(left, top, right, bottom)
    }
}
impl From<[f32; 4]> for Rectangle {
    fn from([left, top, right, bottom]: [f32; 4]) -> Self {
        Self::px(left, top, right, bottom)
    }
}
impl From<glam::Vec4> for Rectangle {
    fn from(p: glam::Vec4) -> Self {
        Self::px(p.x, p.y, p.z, p.w)
    }
}
impl From<(Value, Value, Value, Value)> for Rectangle {
    fn from((left, top, right, bottom): (Value, Value, Value, Value)) -> Self {
        Self::new(left, top, right, bottom)
    }
}
impl From<(Point, Point)> for Rectangle {
    fn from((horiz, vert): (Point, Point)) -> Self {
        Self::new(horiz.x, vert.x, horiz.y, vert.y)
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
