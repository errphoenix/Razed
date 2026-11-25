use std::fmt::Debug;

use ethel::Resolution;

use crate::{
    CommonWidgetOptions, ComponentKind, HasOptions, InterfaceSystem, LayoutOptions, LayoutPosition,
    Point, Rectangle, Value, WidgetId, look::LookId,
};

pub trait HasUi: Debug {
    fn build_node(&self, parent: Option<WidgetId>, interface: &mut InterfaceSystem) -> WidgetId;

    fn archetype(&self) -> ComponentKind;

    fn additional_margin(&self) -> Option<Rectangle> {
        None
    }

    fn additional_padding(&self) -> Option<Rectangle> {
        None
    }

    fn is_panel(&self) -> bool {
        matches!(self.archetype(), ComponentKind::Panel(..))
    }

    fn is_button(&self) -> bool {
        matches!(self.archetype(), ComponentKind::Button { .. })
    }

    fn is_text(&self) -> bool {
        matches!(self.archetype(), ComponentKind::Text(..))
    }

    fn is_image(&self) -> bool {
        matches!(self.archetype(), ComponentKind::Image(..))
    }

    fn is_slider(&self) -> bool {
        matches!(self.archetype(), ComponentKind::Slider { .. })
    }

    fn is_checkbox(&self) -> bool {
        matches!(self.archetype(), ComponentKind::Checkbox { .. })
    }

    fn is_dropdown(&self) -> bool {
        matches!(self.archetype(), ComponentKind::Dropdown { .. })
    }

    fn has_callback(&self) -> bool {
        self.is_slider() || self.is_button()
    }
}

pub type UiPanel = UiNode<crate::Panel>;
pub type UiButton = UiNode<crate::Button>;
pub type UiText = UiNode<crate::Text>;
pub type UiImage = UiNode<crate::Image>;
pub type UiSlider = UiNode<crate::Slider>;
pub type UiCheckbox = UiNode<crate::Checkbox>;
pub type UiDropdown = UiNode<crate::Dropdown>;

#[derive(Debug)]
pub struct UiNodeStack<'stack> {
    pub(crate) reference_resolution: Resolution,
    pub(crate) inner: &'stack mut Vec<Box<dyn HasUi>>,
}
impl UiNodeStack<'_> {
    pub fn push_mut(&mut self, node: impl HasUi + 'static) -> &mut Box<dyn HasUi> {
        self.inner.push_mut(Box::new(node))
    }

    pub fn push(&mut self, node: impl HasUi + 'static) {
        let _ = self.push_mut(node);
    }

    pub const fn reference_resolution(&self) -> Resolution {
        self.reference_resolution
    }
}

#[derive(Debug)]
pub struct UiNode<T: HasOptions> {
    reference_resolution: Resolution,
    commons: CommonWidgetOptions,
    options: T::Options,
    children: Vec<Box<dyn HasUi>>,
}
impl<T: HasOptions> Default for UiNode<T> {
    fn default() -> Self {
        Self {
            reference_resolution: Resolution::default(),
            commons: CommonWidgetOptions::new(),
            options: Default::default(),
            children: Vec::new(),
        }
    }
}
impl<T: HasOptions + Debug> HasUi for UiNode<T> {
    fn build_node(&self, parent: Option<WidgetId>, interface: &mut InterfaceSystem) -> WidgetId {
        let mut commons = self.commons.clone();
        if parent.is_some() {
            commons.parent = parent;
        }

        if let Some(margin) = self.additional_margin() {
            let current = commons.layout.margin.unwrap_or_default();
            commons.layout.margin = Some(sum_rects(current, margin, self.reference_resolution));
        }
        if let Some(padding) = self.additional_padding() {
            let current = commons.layout.padding.unwrap_or_default();
            commons.layout.padding = Some(sum_rects(current, padding, self.reference_resolution));
        }

        let options = T::complement_options(commons, self.options, self.reference_resolution);
        let id = interface.create_element(options).unwrap().0;
        for child in &self.children {
            child.build_node(Some(id), interface);
        }
        id
    }

    fn archetype(&self) -> ComponentKind {
        T::archetype()
    }
}
impl<T: HasOptions + Debug> UiNode<T> {
    pub fn new(reference_resolution: Resolution) -> Self {
        Self {
            reference_resolution,
            ..Default::default()
        }
    }

    pub fn from(options: T::Options, reference_resolution: Resolution) -> Self {
        Self {
            options,
            reference_resolution,
            ..Default::default()
        }
    }

    pub const fn with_parent(mut self, parent: WidgetId) -> Self {
        self.commons = self.commons.with_parent(parent);
        self
    }

    pub const fn with_look(mut self, look: LookId) -> Self {
        self.commons = self.commons.with_look(look);
        self
    }

    pub const fn with_layout(mut self, layout: LayoutOptions) -> Self {
        self.commons = self.commons.with_layout(layout);
        self
    }

    /// Set an absolute position on screen.
    pub fn with_offset(mut self, offset: impl Into<Point>) -> Self {
        let offset = offset.into();
        let layout = &mut self.commons.layout;
        layout.layout_position = match layout.layout_position {
            LayoutPosition::Relative => {
                crate::style_layout!(position Some(offset.x), Some(offset.y))
            }
            LayoutPosition::Absolute { x, y } => {
                let ref_x = self.reference_resolution.width;
                let ref_y = self.reference_resolution.height;
                let x = x.unwrap_or_default().sum(offset.x, ref_x);
                let y = y.unwrap_or_default().sum(offset.y, ref_y);
                crate::style_layout!(position Some(x), Some(y))
            }
        };
        self
    }

    pub fn with_size(mut self, size: impl Into<Point>) -> Self {
        let size = size.into();
        let layout = &mut self.commons.layout;
        layout.size = Some(size);
        self
    }

    pub fn with_margin(mut self, margin: impl Into<Rectangle>) -> Self {
        let margin = margin.into();
        let layout = &mut self.commons.layout;
        layout.margin = Some(margin);
        self
    }

    pub fn with_padding(mut self, padding: impl Into<Rectangle>) -> Self {
        let padding = padding.into();
        let layout = &mut self.commons.layout;
        layout.padding = Some(padding);
        self
    }

    pub const fn with_layer(mut self, layer: u32) -> Self {
        self.commons = self.commons.with_layer(layer);
        self
    }

    pub const fn with_visible(mut self, visible: bool) -> Self {
        self.commons = self.commons.with_visible(visible);
        self
    }

    pub fn builder<F: FnOnce(UiNodeStack, T::Options) -> T::Options>(
        mut self,
        operation: F,
    ) -> Self {
        self.options = operation(
            UiNodeStack {
                reference_resolution: self.reference_resolution,
                inner: &mut self.children,
            },
            self.options,
        );
        self
    }

    pub fn children(&self) -> &[Box<dyn HasUi>] {
        &self.children
    }

    pub const fn children_mut(&mut self) -> &mut Vec<Box<dyn HasUi>> {
        &mut self.children
    }

    pub const fn inner_options(&self) -> &T::Options {
        &self.options
    }

    pub const fn inner_options_mut(&mut self) -> &mut T::Options {
        &mut self.options
    }

    pub const fn inner_common_options(&self) -> &CommonWidgetOptions {
        &self.commons
    }

    pub const fn inner_common_options_mut(&mut self) -> &mut CommonWidgetOptions {
        &mut self.commons
    }

    pub fn apply(self, interface: &mut InterfaceSystem) -> WidgetId {
        self.build_node(None, interface)
    }
}

#[macro_export]
macro_rules! style_layout {
    () => {
        std::default::Default::default()
    };

    // basic container types
    (container-block) => {
        $crate::style::ContainerLayout::Block
    };
    (container-none) => {
        $crate::style::ContainerLayout::None
    };

    // position mode
    (position $x:expr, $y:expr) => {
        $crate::style::LayoutPosition::Absolute { x: $x, y: $y }
    };
    (position-relative) => {
        $crate::style::LayoutPosition::Relative
    };
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
    (position-absolute $x:expr, $y:expr) => {{
        #[allow(unused)]
        let (mut x, mut y) = (None, None);
        x = Some($crate::style::Value::Absolute($x));
        y = Some($crate::style::Value::Absolute($y));
        $crate::style::LayoutPosition::Absolute { x, y }
    }};

    // item alignment
    (align-item-auto) => { $crate::style::ItemAlignment::Auto };
    (align-item-stretch) => { $crate::style::ItemAlignment::Stretch };
    (align-item-baseline) => { $crate::style::ItemAlignment::Baseline };
    (align-item-start) => { $crate::style::ItemAlignment::Start };
    (align-item-center) => { $crate::style::ItemAlignment::Center };
    (align-item-end) => { $crate::style::ItemAlignment::End };

    // content alignment
    (align-content-auto) => { $crate::style::ContentAlignment::Auto };
    (align-content-stretch) => { $crate::style::ContentAlignment::Stretch };
    (align-content-start) => { $crate::style::ContentAlignment::Start };
    (align-content-center) => { $crate::style::ContentAlignment::Center };
    (align-content-end) => { $crate::style::ContentAlignment::End };
    (align-content-space-evenly) => { $crate::style::ContentAlignment::SpaceEvenly };
    (align-content-space-around) => { $crate::style::ContentAlignment::SpaceAround };
    (align-content-space-between) => { $crate::style::ContentAlignment::SpaceBetween };

    // flex direction
    (flex-row) => { $crate::style::FlexDirection::Row };
    (flex-column) => { $crate::style::FlexDirection::Column };
    (flex-row-reverse) => { $crate::style::FlexDirection::RowReverse };
    (flex-column-reverse) => { $crate::style::FlexDirection::ColumnReverse };

    // flex wrap
    (flex-wrap) => { $crate::style::Wrap::Wrap };
    (flex-no-wrap) => { $crate::style::Wrap::DontWrap };
    (flex-wrap-revere) => { $crate::style::Wrap::Reverse };

    // grid flow
    (grid-flow-row) => { $crate::style::GridFlow::Row };
    (grid-flow-column) => { $crate::style::GridFlow::Column };
    (grid-flow-row-dense) => { $crate::style::GridFlow::RowDense };
    (grid-flow-column-dense) => { $crate::style::GridFlow::ColumnDense };

    // grid line/span
    (grid-line) => { $crate::style::GridLine::Single(0) };
    (grid-line-$v:expr) => { $crate::style::GridLine::Single($v) };
    (grid-span) => { $crate::style::GridLine::Span(0) };
    (grid-span-$v:expr) => { $crate::style::GridLine::Span($v) };

    // overflow
    (overflow-visible) => { $crate::style::Overflow::Visible };
    (overflow-hidden) => { $crate::style::Overflow::Hidden };
    (overflow-clip) => { $crate::style::Overflow::Clip };
    (overflow-scroll) => { $crate::style::Overflow::Scroll };

    // real (absolute)
    (px) => { $crate::style::Value::Absolute(1f32) };
    (px $v:literal) => { $crate::style::Value::Absolute($v) };
    (pixel $v:expr) => { $crate::style::Value::Absolute($v) };
    (pixels $v:expr) => { $crate::style::Value::Absolute($v) };
    (abs $v:expr) => { $crate::style::Value::Absolute($v) };
    (absolute $v:expr) => {
        $crate::style::Value::Absolute($v)
    };
    ($num:literal) => {
        $crate::style::Value::Absolute($num)
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
    ($x:literal, $y:literal) => {
        $crate::style::Point {
            x: $crate::style_layout!(px $x),
            y: $crate::style_layout!(px $y),
        }
    };
    ($x:expr, $y:expr) => {
        $crate::style::Point {
            x: $x,
            y: $y,
        }
    };
    (splat $($t:tt)*) => {
        $crate::style::Point {
            x: $crate::style_layout!($($t)*),
            y: $crate::style_layout!($($t)*),
        }
    };

    // rect
    ($l:literal, $t:literal, $r:literal, $b:literal) => {
        $crate::style::Rectangle {
            left: $crate::style_layout!(px $l),
            top: $crate::style_layout!(px $t),
            right: $crate::style_layout!(px $r),
            bottom: $crate::style_layout!(px $b),
        }
    };
    (sq $v:literal) => { $crate::style_layout!($v, $v, $v, $v) };
    (square $v:literal) => { $crate::style_layout!(sq $v) };
    (rect $v:literal) => { $crate::style_layout!(sq $v) };
    (horiz $h:literal, vert $v:literal) => { $crate::style_layout!($h, $v, $h, $v) };
    (left $v:literal) => { $crate::style_layout!($v, 0.0, 0.0, 0.0) };
    (top $v:literal) => { $crate::style_layout!(0.0, $v, 0.0, 0.0) };
    (right $v:literal) => { $crate::style_layout!(0.0, 0.0, $v, 0.0) };
    (bottom $v:literal) => { $crate::style_layout!(0.0, 0.0, 0.0, $v) };
    ($l:expr, $t:expr, $r:expr, $b:expr) => {
        $crate::style::Rectangle {
            left: $l,
            top: $t,
            right: $r,
            bottom: $b,
        }
    };
    (sq $v:expr) => { $crate::style_layout!($v, $v, $v, $v) };
    (square $v:expr) => { $crate::style_layout!(sq $v) };
    (rect $v:expr) => { $crate::style_layout!(sq $v) };
    (horiz $h:expr, vert $v:expr) => { $crate::style_layout!($h, $v, $h, $v) };
    (left $v:expr) => { $crate::style_layout!($v, $crate::style_layout!(0.0), $crate::style_layout!(0.0), $crate::style_layout!(0.0)) };
    (top $v:expr) => { $crate::style_layout!($crate::style_layout!(0.0), $v, $crate::style_layout!(0.0), $crate::style_layout!(0.0)) };
    (right $v:expr) => { $crate::style_layout!($crate::style_layout!(0.0), $crate::style_layout!(0.0), $v, $crate::style_layout!(0.0)) };
    (bottom $v:expr) => { $crate::style_layout!($crate::style_layout!(0.0), $crate::style_layout!(0.0), $crate::style_layout!(0.0), $v) };
}

pub(crate) const fn sum_rects(
    a: Rectangle,
    b: Rectangle,
    scaling_reference: Resolution,
) -> Rectangle {
    Rectangle {
        left: sum_values(a.left, b.left, scaling_reference.width),
        top: sum_values(a.top, b.top, scaling_reference.height),
        right: sum_values(a.right, b.right, scaling_reference.width),
        bottom: sum_values(a.bottom, b.bottom, scaling_reference.height),
    }
}

#[allow(unused)]
pub(crate) const fn sum_points(a: Point, b: Point, scaling_reference: Resolution) -> Point {
    Point {
        x: sum_values(a.x, b.x, scaling_reference.width),
        y: sum_values(a.y, b.y, scaling_reference.height),
    }
}

pub(crate) const fn sum_values(a: Value, b: Value, scaling_reference: f32) -> Value {
    match a {
        Value::Absolute(a_px) => match b {
            Value::Absolute(b_px) => Value::Absolute(a_px + b_px),
            Value::Percentage(b_perc) => {
                let a_perc = a_px / scaling_reference;
                Value::Percentage(a_perc + b_perc)
            }
        },
        Value::Percentage(a_perc) => match b {
            Value::Absolute(b_px) => {
                let b_perc = b_px / scaling_reference;
                Value::Percentage(a_perc + b_perc)
            }
            Value::Percentage(b_perc) => Value::Percentage(a_perc + b_perc),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use ethel::Resolution;
    use janus::StringHash;

    use crate::{ButtonOptions, CommonWidgetOptions, InterfaceSystem, env::UiEnv};

    #[test]
    fn declare_text_string() {
        let mut env = UiEnv::new();
        let mut buf = String::new();

        const TEST_ID_0: janus::StringHash = janus::hash_string("__test.id-0");
        const TEST_ID_1: janus::StringHash = janus::hash_string("__test.id-1");

        env.insert(TEST_ID_0, "people");
        env.insert(TEST_ID_1, 15);

        let txt0 = crate::text_contents!("hello", " ", "world", "!",);
        let txt1 = crate::text_contents!("hello", " ", TEST_ID_0, "!");
        let txt2 = crate::text_contents!(
            "hello",
            " ",
            |TEST_ID_1: v| {
                let num = v.as_integer().unwrap();
                crate::env::EnvValue::DynamicString(format!("{num} baboons"))
            },
            "!"
        );

        const TEST_0: &str = "hello world!";
        const TEST_1: &str = "hello people!";
        const TEST_2: &str = "hello 15 baboons!";

        txt0.resolve(&env, &mut buf);
        println!("{buf}");
        assert_eq!(janus::hash_string(&buf), janus::hash_string(&TEST_0));
        buf.clear();

        txt1.resolve(&env, &mut buf);
        println!("{buf}");
        assert_eq!(janus::hash_string(&buf), janus::hash_string(&TEST_1));
        buf.clear();

        txt2.resolve(&env, &mut buf);
        println!("{buf}");
        assert_eq!(janus::hash_string(&buf), janus::hash_string(&TEST_2));
        buf.clear();
    }

    #[test]
    fn declare_widgets() {
        let mut sys = InterfaceSystem::new(Resolution::default());

        #[allow(unused)]
        const AAA: janus::StringHash = janus::hash_string("a");

        let _ = crate::wd_text! {
            content "Hello people";
            font "Times New Roman";
            always_invalidate
        };

        let core = CommonWidgetOptions::default();
        let _ = crate::wd_text! { sys, core =>
            size 15.0;
            content crate::text_contents!("Hello ", AAA, " baboons");
            font "Times New Roman"
        };
    }

    #[test]
    fn declare_with_stack() {
        let resolution = Resolution::default();
        let _ = UiPanel::new(resolution).builder(|mut stack, options| {
            stack.button_from(ButtonOptions::labelled_literal("some button"));
            stack.panel(|panel| {
                panel.builder(|mut stack, options| {
                    stack.slider(|slider| slider.builder(|_, options| options.with_vertical(true)));
                    stack.button(|button| {
                        button.builder(|_, options| options.with_label_literal("another button"))
                    });
                    options
                })
            });
            options
        });
    }

    #[test]
    fn declare_callbacks() {
        const ID: StringHash = janus::hash_string("test-id");

        let _ = crate::button_callback!(set ID 0.3);
        let _ = crate::button_callback!(toggle ID);
        let _ = crate::button_callback!(increment ID);
        let _ = crate::button_callback!(hold ID);
        let _ = crate::button_callback!(hold ID 0.5);

        fn map_test_1(v: f32) -> f32 {
            v + 0.5
        }

        struct Mapping;
        impl Mapping {
            fn map(v: f32) -> f32 {
                v + 0.25
            }
        }

        let _ = crate::slider_callback!(copy ID);
        let _ = crate::slider_callback!(map ID -1.0, 1.0);
        let _ = crate::slider_callback!(map ID map_test_1);
        let _ = crate::slider_callback!(map ID Mapping::map);
    }
}
