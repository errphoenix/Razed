use std::fmt::Write;

use ethel::{Column, IndirectIndex};
use janus::StringHash;

use crate::{
    CommonWidgetOptions, ComponentKind, ElementOptions, HasOptions, InterfaceSystem, WidgetError,
    WidgetId,
    declare::{UiNodeStack, UiText},
    env::{EnvValue, UiEnv},
    text::{FontMetrics, TextMeasurement},
};

#[derive(Clone, Copy, Debug, Default)]
pub struct TextContents(pub [TextNode; Self::NODE_COUNT]);
impl TextContents {
    pub const NODE_COUNT: usize = 18;

    pub const fn literal(string: &'static str) -> Self {
        Self::from_node(TextNode::Static(string))
    }

    pub const fn from_node(node: TextNode) -> Self {
        let mut nodes = [TextNode::Empty; Self::NODE_COUNT];
        nodes[0] = node;
        Self(nodes)
    }

    pub const fn from_array(array: [TextNode; Self::NODE_COUNT]) -> Self {
        Self(array)
    }

    pub fn from_nodes(input_nodes: &[TextNode]) -> Self {
        let mut nodes = [TextNode::Empty; Self::NODE_COUNT];
        input_nodes
            .iter()
            .take(Self::NODE_COUNT)
            .enumerate()
            .for_each(|(i, node)| {
                nodes[i] = *node;
            });
        Self(nodes)
    }

    pub fn resolve(&self, env: &UiEnv, out: &mut String) {
        for node in &self.0 {
            match node {
                TextNode::Static(literal) => out.write_str(literal).unwrap(),
                TextNode::Variable(string_hash) => {
                    if let Some(value) = env.get(string_hash) {
                        value.write(out).unwrap()
                    }
                }
                TextNode::VariableAnd { env_id, operation } => {
                    if let Some(value) = env.get(env_id) {
                        let value = operation(value);
                        value.write(out).unwrap()
                    }
                }
                _ => {}
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Hash)]
pub enum TextNode {
    #[default]
    Empty,
    Static(&'static str),
    /// Variable, dynamic String text content polled from the
    /// [`environment`](env::UiEnv).
    Variable(StringHash),
    VariableAnd {
        env_id: StringHash,
        operation: fn(&EnvValue) -> EnvValue,
    },
}

ethel::table_spec! {
    struct InterfaceText {
        contents: TextContents;
        cached_text: StringHash;
        font_name: &'static str;
        metrics: FontMetrics;
        measure: TextMeasurement;
    }
}

impl<const LAYERS: usize> InterfaceSystem<LAYERS> {
    pub fn invalidate_text_changes(&mut self) {
        self.text_dynamic_list
            .retain(|&w_id| self.commons.solve_indirect(w_id.0).is_some());

        for w_id in &self.text_dynamic_list {
            let root = self.commons.solve_indirect(w_id.0).unwrap();
            if let ComponentKind::Text(text_id) = self.commons.archetype[root.as_index()] {
                let tdid = self.texts.solve_indirect(text_id).unwrap().as_index();
                self.text_resolve_buf.clear();
                self.texts.contents[tdid].resolve(&self.environment, &mut self.text_resolve_buf);
                let current = janus::hash_string(&self.text_resolve_buf);
                if self.texts.cached_text[tdid] != current {
                    let taffy_id = self.commons.taffy_id[root.as_index()];
                    self.layout.mark_dirty(taffy_id.0).unwrap();
                    self.texts.cached_text[tdid] = current;
                }
            }
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
        contents: TextContents,
        font_name: &'static str,
        metrics: FontMetrics,
        never_invalidate: bool,
    ) -> Result<IndirectIndex, WidgetError> {
        if let Some(commons_id) = self.commons.solve_indirect(id.0) {
            self.assert_null_archetype(commons_id)?;
            let text_element = (
                contents,
                StringHash::default(),
                font_name,
                metrics,
                TextMeasurement::default(),
            );
            let text_id = self.texts.insert(text_element);
            self.commons.archetype[commons_id.as_index()] = ComponentKind::Text(text_id);
            if !never_invalidate {
                self.text_dynamic_list.push(id);
            }

            Ok(text_id)
        } else {
            Err(WidgetError::InvalidWidgetHandle(id.0.as_int()))
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct TextOptions {
    pub contents: TextContents,
    pub font_name: &'static str,
    pub font_size: f32,
    pub line_height: f32,
    pub never_invalidate: bool,
}
impl TextOptions {
    pub const DEFAULT_TEXT: TextContents = TextContents::literal("Invalid Text");
    pub const DEFAULT_FONT: &'static str = "Arial";

    pub const DEFAULT_FONT_SIZE: f32 = 14.0;
    pub const DEFAULT_LINE_HEIGHT: f32 = 20.0;
}
impl Default for TextOptions {
    fn default() -> Self {
        Self::new()
    }
}
impl TextOptions {
    pub const fn new() -> Self {
        Self {
            contents: Self::DEFAULT_TEXT,
            font_name: Self::DEFAULT_FONT,
            font_size: Self::DEFAULT_FONT_SIZE,
            line_height: Self::DEFAULT_LINE_HEIGHT,
            never_invalidate: true,
        }
    }

    pub const fn from_literal(literal: &'static str) -> Self {
        Self::new().with_contents_literal(literal)
    }

    pub const fn from_contents(contents: TextContents) -> Self {
        Self::new().with_contents(contents)
    }

    pub const fn with_contents_literal(self, contents_literal: &'static str) -> Self {
        Self {
            contents: TextContents::literal(contents_literal),
            ..self
        }
    }

    pub const fn with_contents(self, contents: TextContents) -> Self {
        Self { contents, ..self }
    }

    pub const fn with_font_family(self, font_name: &'static str) -> Self {
        Self { font_name, ..self }
    }

    pub const fn with_font_size(self, font_size: f32) -> Self {
        Self { font_size, ..self }
    }

    pub const fn with_line_height(self, line_height: f32) -> Self {
        Self {
            line_height,
            ..self
        }
    }

    pub const fn with_revalidation(self, revalidation: bool) -> Self {
        Self {
            never_invalidate: !revalidation,
            ..self
        }
    }
}

#[derive(Debug)]
pub struct Text;
impl HasOptions for Text {
    type Options = TextOptions;

    fn complement_options(common: CommonWidgetOptions, this: Self::Options) -> ElementOptions {
        ElementOptions::Text(common, this)
    }

    fn archetype() -> ComponentKind {
        ComponentKind::Text(Default::default())
    }
}

impl UiNodeStack<'_> {
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
