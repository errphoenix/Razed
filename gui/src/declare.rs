use std::fmt::Debug;

use ethel::Resolution;

use crate::{
    ButtonOptions, CommonWidgetOptions, ComponentKind, HasOptions, ImageOptions, InterfaceSystem,
    LayoutOptions, LayoutPosition, PanelOptions, Point, Rectangle, TextContents, TextOptions,
    Value, WidgetId, look::LookId, options::SliderOptions,
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
}

pub type UiPanel = UiNode<crate::options::Panel>;
pub type UiButton = UiNode<crate::options::Button>;
pub type UiText = UiNode<crate::options::Text>;
pub type UiImage = UiNode<crate::options::Image>;
pub type UiSlider = UiNode<crate::options::Slider>;

#[derive(Debug)]
pub struct UiNodeStack<'stack> {
    reference_resolution: Resolution,
    inner: &'stack mut Vec<Box<dyn HasUi>>,
}
impl UiNodeStack<'_> {
    pub fn push_mut(&mut self, node: impl HasUi + 'static) -> &mut Box<dyn HasUi> {
        self.inner.push_mut(Box::new(node))
    }

    pub fn push(&mut self, node: impl HasUi + 'static) {
        let _ = self.push_mut(node);
    }

    pub fn panel(&mut self, panel_op: impl FnOnce(UiPanel) -> UiPanel) {
        self.push(panel_op(UiPanel::new(self.reference_resolution)));
    }

    pub fn panel_from(&mut self, panel_options: PanelOptions) {
        self.push(UiPanel::from(panel_options, self.reference_resolution));
    }

    pub fn button(&mut self, button_op: impl FnOnce(UiButton) -> UiButton) {
        self.push(button_op(UiButton::new(self.reference_resolution)));
    }

    pub fn button_from(&mut self, button_options: ButtonOptions) {
        self.push(UiButton::from(button_options, self.reference_resolution));
    }

    pub fn text(&mut self, text_op: impl FnOnce(UiText) -> UiText) {
        self.push(text_op(UiText::new(self.reference_resolution)));
    }

    pub fn text_from(&mut self, contents: TextContents) {
        self.text(|text| text.builder(|_, props| props.with_contents(contents)));
    }

    pub fn text_literal(&mut self, text_literal: &'static str) {
        self.push(
            UiText::new(self.reference_resolution)
                .builder(|_, props| props.with_contents_literal(text_literal)),
        );
    }

    pub fn text_from_options(&mut self, text_options: TextOptions) {
        self.push(UiText::from(text_options, self.reference_resolution));
    }

    pub fn image(&mut self, image_op: impl FnOnce(UiImage) -> UiImage) {
        self.push(image_op(UiImage::new(self.reference_resolution)));
    }

    pub fn image_from(&mut self, image_options: ImageOptions) {
        self.push(UiImage::from(image_options, self.reference_resolution));
    }

    pub fn slider(&mut self, slider_op: impl FnOnce(UiSlider) -> UiSlider) {
        self.push(slider_op(UiSlider::new(self.reference_resolution)));
    }

    pub fn slider_from(&mut self, slider_options: SliderOptions) {
        self.push(UiSlider::from(slider_options, self.reference_resolution));
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

        let options = T::complement_options(commons, self.options);
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

/// Generate text parameters.
///
/// Available fields:
/// - `content` of type [`TextContents`] - the string to display.
///
///    Alternatively, the type can be of `&'static str` as a shortcut for
///    a static literal string.
/// - `font` of type `&'static str` - the name of the font (defaults to
///   `Arial`)
/// - `color` of type [`glam::Vec4`] - the foreground color
/// - `size` of type `f32` - the font size in pixels
/// - `height` of type `f32` - the line height of the text in pixels
/// - `always_invalidate` field - whether the text should re-layout every frame
///
///   Note: re-layouting text may be expensive, this option should not be
///   used unless necessary, if drastic content changes may require it.
///
/// The above fields can be defined in any order, and are optional.
///
/// Any non-defined field will default to the value as according to
/// [`crate::TextParams::default`].
///
/// [`TextContents`]: crate::TextContents
#[macro_export]
macro_rules! wd_text {
    (@prop $out:ident; content $v:literal $(; $($t:tt)*)?) => {
        $out.contents = $crate::TextContents::literal($v);
        $($crate::wd_text!(@prop $out; $($t:tt)*);)?
    };
    (@prop $out:ident; content $v:expr $(; $($t:tt)*)?) => {
        $out.contents = $v;
        $($crate::wd_text!(@prop $out; $($t:tt)*);)?
    };
    (@prop $out:ident; font $v:expr $(; $($t:tt)*)?) => {
        $out.font_name = $v;
        $($crate::wd_text!(@prop $out; $($t:tt)*);)?
    };
    (@prop $out:ident; color $v:expr $(; $($t:tt)*)?) => {
        $out.color = $v;
        $($crate::wd_text!(@prop $out; $($t:tt)*);)?
    };
    (@prop $out:ident; size $v:expr $(; $($t:tt)*)?) => {
        $out.font_size = $v;
        $($crate::wd_text!(@prop $out; $($t:tt)*);)?
    };
    (@prop $out:ident; height $v:expr $(; $($t:tt)*)?) => {
        $out.line_height = $v;
        $($crate::wd_text!(@prop $out; $($t:tt)*);)?
    };
    (@prop $out:ident; always_invalidate $(; $($t:tt)*)?) => {
        $out.never_invalidate = false;
        $($crate::wd_text!(@prop $out; $($t:tt)*);)?
    };
    (@prop $out:ident; $($t:tt)*) => {};

    ($sys:ident, $core:expr => $($t:tt)*) => { $sys.create_element($crate::ElementOptions::Text($core, $crate::wd_text!($($t)*))) };
    ($($t:tt)*) => {{
        let mut params = $crate::TextOptions::default();
        $crate::wd_text!(@prop params; $($t)*);
        params
    }};
}

/// Generate inner text contents string.
///
/// The string is composed from a series of nodes, described below.
/// Nodes must be listed sequentially, separated by a comma.
///
/// A maximum of [`18`](crate::TextContents::NODE_COUNT) nodes is supported.
///
/// Available nodes:
/// * Static - a literal string: `"Hello world"`
/// * Variable - polled every frame from the UI environment by the given ID
/// * Custom Variable - like `Variable`, but the value is captured to return
///   another:
///
///   `|VAR_NAME : var| { /* do something with `var`, and return another `EnvValue` */ }`
#[macro_export]
macro_rules! text_contents {
    (@node $i:ident, $a:ident; ) => {};
    (@node $i:ident, $a:ident; $str:literal $(, $($o:tt)*)?) => {
        $a[$i] = $crate::TextNode::Static($str);
        $({
            #[allow(unused)]
            {
                $i += 1;
            }
            $crate::text_contents!(@node $i, $a; $($o)*);
        })?
    };
    (@node $i:ident, $a:ident; $var:ident $(, $($o:tt)*)?) => {
        $a[$i] = $crate::TextNode::Variable($var);
        $({
            #[allow(unused)]
            {
                $i += 1;
            }
            $crate::text_contents!(@node $i, $a; $($o)*);
        })?
    };
    (@node $i:ident, $a:ident; |$var:ident| $opfn:ident $(, $($o:tt)*)?) => {
        $a[$i] = $crate::TextNode::VariableAnd {
            env_id: $var,
            operation: $opfn,
        };
        $({
            #[allow(unused)]
            {
                $i += 1;
            }
            $crate::text_contents!(@node $i, $a; $($o)*);
        })?
    };
    (@node $i:ident, $a:ident; |$var:ident: $vname:ident| $op:block $(, $($o:tt)*)?) => {
        $a[$i] = $crate::TextNode::VariableAnd {
            env_id: $var,
            #[allow(unused)]
            operation: |$vname| $op,
        };
        $({
            #[allow(unused)]
            {
                $i += 1;
            }
            $crate::text_contents!(@node $i, $a; $($o)*);
        })?
    };

    ($($t:tt)*) => {{
        #[allow(unused)]
        let mut index = 0;
        #[allow(unused)]
        let mut array = [$crate::TextNode::Empty; $crate::TextContents::NODE_COUNT];
        $crate::text_contents!(@node index, array; $($t)*);
        $crate::TextContents::from_array(array)
    }};
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

#[macro_export]
macro_rules! button_callback {
    (set $id:ident $value:expr) => {
        $crate::ButtonCallback::Once(|env, _| {
            env.insert($id, $value);
        })
    };
    (select $id:ident => $cond:expr; ? $t:expr; : $f:expr) => {
        $crate::ButtonCallback::Once(|env, _| {
            if env.get_boolean(&$cond) {
                env.insert($id, $t);
            } else {
                env.insert($id, $f);
            }

        })
    };
    (toggle $id:ident) => {
        $crate::ButtonCallback::Once(|env, _| env.toggle_boolean(&$id))
    };
    (increment $id:ident $($by:expr)?) => {
        $crate::ButtonCallback::Once(|env, _| {
            let value = env.get_integer(&$id);
            env.insert($id, value + 1 $(+ $by - 1)?);
        })
    };
    (hold $id:ident $($factor:expr)?) => {
        $crate::ButtonCallback::Repeating(|env, data| {
            env.insert($id, data.seconds $(* $factor)?);
        })
    };
}

#[macro_export]
macro_rules! slider_callback {
    (copy $id:ident) => {
        $crate::SliderCallback::Repeating(|env, value| {
            env.insert($id, *value);
        })
    };
    (map $id:ident $op:expr) => {
        $crate::SliderCallback::Repeating(|env, value| {
            let value = $op(*value);
            env.insert($id, value);
        })
    };
    (map $id:ident $min:expr, $max:expr) => {
        $crate::SliderCallback::Repeating(|env, value| {
            let value = ($min as f32) + (($max as f32 - $min as f32) * *value);
            env.insert($id, value);
        })
    };
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
        let mut sys = InterfaceSystem::<1>::new(Resolution::default());

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
                    stack.slider(|slider| {
                        slider.builder(|_, options| {
                            options.with_text_literal("slideee").with_vertical(true)
                        })
                    });
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

        let _ = button_callback!(set ID 0.3);
        let _ = button_callback!(toggle ID);
        let _ = button_callback!(increment ID);
        let _ = button_callback!(hold ID);
        let _ = button_callback!(hold ID 0.5);

        fn map_test_1(v: f32) -> f32 {
            v + 0.5
        }

        struct Mapping;
        impl Mapping {
            fn map(v: f32) -> f32 {
                v + 0.25
            }
        }

        let _ = slider_callback!(copy ID);
        let _ = slider_callback!(map ID -1.0, 1.0);
        let _ = slider_callback!(map ID map_test_1);
        let _ = slider_callback!(map ID Mapping::map);
    }
}
