#[macro_export]
macro_rules! style {
    () => {
        std::default::Default::default()
    };

    // position mode
    (position-relative) => {
        $crate::style::LayoutPosition::Relative
    };
    (position-absolute x $x:expr) => {{
        #[allow(unused)]
        let (mut x, mut y) = (None, None);
        $crate::style::LayoutPosition::Absolute { x, y }
    }};
    (position-absolute x $x:expr) => {{
        #[allow(unused)]
        let (mut x, mut y) = (None, None);
        x = Some($crate::style::Value::Absolute($x));
        $crate::style::LayoutPosition::Absolute { x, y }
    }};
    (position-absolute y $y:expr) => {{
        #[allow(unused)]
        let (mut x, mut y) = (None, None);
        y = Some($crate::style::Value::Absolute($y));
        $crate::style::LayoutPosition::Absolute { x, y }
    }};
    (position-absolute x $x:expr, y $y:expr) => {{
        #[allow(unused)]
        let (mut x, mut y) = (None, None);
        x = Some($crate::style::Value::Absolute($x));
        y = Some($crate::style::Value::Absolute($y));
        $crate::style::LayoutPosition::Absolute { x, y }
    }};

    // item alignment
    (align-item-auto) => {
        $crate::style::ItemAlignment::Auto
    };
    (align-item-stretch) => {
        $crate::style::ItemAlignment::Stretch
    };
    (align-item-baseline) => {
        $crate::style::ItemAlignment::Baseline
    };
    (align-item-start) => {
        $crate::style::ItemAlignment::Start
    };
    (align-item-center) => {
        $crate::style::ItemAlignment::Center
    };
    (align-item-end) => {
        $crate::style::ItemAlignment::End
    };

    // content alignment
    (align-content-auto) => {
        $crate::style::ContentAlignment::Auto
    };
    (align-content-stretch) => {
        $crate::style::ContentAlignment::Stretch
    };
    (align-content-start) => {
        $crate::style::ContentAlignment::Start
    };
    (align-content-center) => {
        $crate::style::ContentAlignment::Center
    };
    (align-content-end) => {
        $crate::style::ContentAlignment::End
    };
    (align-content-space-evenly) => {
        $crate::style::ContentAlignment::SpaceEvenly
    };
    (align-content-space-around) => {
        $crate::style::ContentAlignment::SpaceAround
    };
    (align-content-space-between) => {
        $crate::style::ContentAlignment::SpaceBetween
    };

    // flex direction
    (flex-row) => {
        $crate::style::FlexDirection::Row
    };
    (flex-column) => {
        $crate::style::FlexDirection::Column
    };
    (flex-row-reverse) => {
        $crate::style::FlexDirection::RowReverse
    };
    (flex-column-reverse) => {
        $crate::style::FlexDirection::ColumnReverse
    };

    // flex wrap
    (flex-wrap) => {
        $crate::style::Wrap::Wrap
    };
    (flex-no-wrap) => {
        $crate::style::Wrap::DontWrap
    };
    (flex-wrap-revere) => {
        $crate::style::Wrap::Reverse
    };

    // grid flow
    (grid-flow-row) => {
        $crate::style::GridFlow::Row
    };
    (grid-flow-column) => {
        $crate::style::GridFlow::Column
    };
    (grid-flow-row-dense) => {
        $crate::style::GridFlow::RowDense
    };
    (grid-flow-column-dense) => {
        $crate::style::GridFlow::ColumnDense
    };

    // grid line/span
    (grid-line) => {
        $crate::style::GridLine::Single(0)
    };
    (grid-line-$v:expr) => {
        $crate::style::GridLine::Single($v)
    };
    (grid-span) => {
        $crate::style::GridLine::Span(0)
    };
    (grid-span-$v:expr) => {
        $crate::style::GridLine::Span($v)
    };

    // overflow
    (overflow-visible) => {
        $crate::style::Overflow::Visible
    };
    (overflow-hidden) => {
        $crate::style::Overflow::Hidden
    };
    (overflow-clip) => {
        $crate::style::Overflow::Clip
    };
    (overflow-scroll) => {
        $crate::style::Overflow::Scroll
    };

    // real (absolute)
    (px) => {
        $crate::style::Value::Absolute(1f32)
    };
    (px $v:literal) => {
        $crate::style::Value::Absolute($v)
    };
    (pixel $v:expr) => {
        $crate::style::Value::Absolute($v)
    };
    (pixels $v:expr) => {
        $crate::style::Value::Absolute($v)
    };
    (abs $v:expr) => {
        $crate::style::Value::Absolute($v)
    };
    (absolute $v:expr) => {
        $crate::style::Value::Absolute($v)
    };
    (0) => {
        $crate::style::Value::Absolute(0f32)
    };
    (0.0) => {
        $crate::style::Value::Absolute(0f32)
    };

    // real (relative)
    (full) => {
        $crate::style::Value::Percentage(1f32)
    };
    (half) => {
        $crate::style::Value::Percentage(1f32 / 2f32)
    };
    (1/2) => {
        $crate::style::Value::Percentage(1f32 / 2f32)
    };
    (1/3) => {
        $crate::style::Value::Percentage(1f32 / 3f32)
    };
    (1/4) => {
        $crate::style::Value::Percentage(1f32 / 4f32)
    };
    (1/5) => {
        $crate::style::Value::Percentage(1f32 / 5f32)
    };
    (1/6) => {
        $crate::style::Value::Percentage(1f32 / 6f32)
    };
    (1/7) => {
        $crate::style::Value::Percentage(1f32 / 7f32)
    };
    (1/8) => {
        $crate::style::Value::Percentage(1f32 / 8f32)
    };
    (1/9) => {
        $crate::style::Value::Percentage(1f32 / 9f32)
    };
    (1/10) => {
        $crate::style::Value::Percentage(1f32 / 10f32)
    };
    (relative $v:expr) => {
        $crate::style::Value::Percentage($v)
    };
    (rel $v:expr) => {
        $crate::style::Value::Percentage($v)
    };
    (percentage $v:expr) => {
        $crate::style::Value::Percentage($v)
    };
    (perc $v:expr) => {
        $crate::style::Value::Percentage($v)
    };

    // point
    ($x:expr, $y:expr) => {
        $crate::style::Point {
            x: $x,
            y: $y,
        }
    };
    (splat $($t:tt)*) => {
        $crate::style::Point {
            x: $crate::style!($($t)*),
            y: $crate::style!($($t)*),
        }
    };

    // rect
    ($l:expr, $t:expr, $r:expr, $b:expr) => {
        $crate::style::Rectangle {
            left: $l,
            top: $t,
            right: $r,
            bottom: $b,
        }
    };
    (horiz $h:expr, vert $v:expr) => {
        $crate::style::Rectangle {
            left: $h,
            top: $v,
            right: $h,
            bottom: $v,
        }
    };
    (left $v:expr) => {
        $crate::style::Rectangle {
            left: $v,
            top: $crate::style!(0.0),
            right: $crate::style!(0.0),
            bottom: $crate::style!(0.0),
        }
    };
    (top $v:expr) => {
        $crate::style::Rectangle {
            top: $v,
            left: $crate::style!(0.0),
            right: $crate::style!(0.0),
            bottom: $crate::style!(0.0),
        }
    };
    (right $v:expr) => {
        $crate::style::Rectangle {
            right: $v,
            top: $crate::style!(0.0),
            left: $crate::style!(0.0),
            bottom: $crate::style!(0.0),
        }
    };
    (bottom $v:expr) => {
        $crate::style::Rectangle {
            bottom: $v,
            top: $crate::style(0.0),
            right: $crate::style(0.0),
            left: $crate::style(0.0),
        }
    };
}
