use ethel::{Column, IndirectIndex, Resolution};

use crate::{
    CommonWidgetOptions, ComponentKind, ElementOptions, HasOptions, InteractableCallback,
    InteractionTime, InterfaceSystem, TextContents, TextOptions, WidgetError, WidgetId,
    declare::{UiButton, UiNodeStack},
};

pub type ButtonCallbackCtx = InteractionTime;
pub type ButtonCallback = InteractableCallback<ButtonCallbackCtx>;

ethel::table_spec! {
    struct InterfaceButton {
        // handle in text table
        text_id: IndirectIndex;
        callback: ButtonCallback;
    }
}

impl<const LAYERS: usize> InterfaceSystem<LAYERS> {
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
        text_root_id: WidgetId,
        text_id: IndirectIndex,
        callback: ButtonCallback,
    ) -> Result<IndirectIndex, WidgetError> {
        if let Some(commons_id) = self.data.commons.solve_indirect(root_id.0) {
            self.assert_null_archetype(commons_id)?;
            let button_element = (text_id, callback);
            let button_id = self.data.buttons.insert(button_element);
            self.data.commons.archetype[commons_id.as_index()] = ComponentKind::Button {
                handle: button_id,
                text_handle: text_root_id,
            };
            Ok(button_id)
        } else {
            Err(WidgetError::InvalidWidgetHandle(root_id.0.as_int()))
        }
    }
}

#[derive(Clone, Debug, Copy, Default)]
pub struct ButtonOptions {
    pub text: TextOptions,
    pub callback: ButtonCallback,
}
impl ButtonOptions {
    pub const fn new() -> Self {
        Self {
            text: TextOptions::new(),
            callback: ButtonCallback::None,
        }
    }

    pub const fn labelled(label: TextOptions) -> Self {
        Self::new().with_label(label)
    }

    pub const fn labelled_literal(label_literal: &'static str) -> Self {
        Self::new().with_label_literal(label_literal)
    }

    pub const fn with_label(self, label: TextOptions) -> Self {
        Self {
            text: label,
            ..self
        }
    }

    pub const fn with_label_literal(self, label_literal: &'static str) -> Self {
        Self {
            text: TextOptions::new().with_contents(TextContents::literal(label_literal)),
            ..self
        }
    }

    pub const fn with_callback(self, callback: ButtonCallback) -> Self {
        Self { callback, ..self }
    }
}

#[derive(Debug)]
pub struct Button;
impl HasOptions for Button {
    type Options = ButtonOptions;

    fn complement_options(
        common: CommonWidgetOptions,
        this: Self::Options,
        _: Resolution,
    ) -> ElementOptions {
        ElementOptions::Button(common, this)
    }

    fn archetype() -> ComponentKind {
        ComponentKind::Button {
            handle: Default::default(),
            text_handle: Default::default(),
        }
    }
}

impl UiNodeStack<'_> {
    pub fn button(&mut self, button_op: impl FnOnce(UiButton) -> UiButton) {
        self.push(button_op(UiButton::new(self.reference_resolution)));
    }

    pub fn button_from(&mut self, button_options: ButtonOptions) {
        self.push(UiButton::from(button_options, self.reference_resolution));
    }
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
