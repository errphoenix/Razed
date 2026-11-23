use ethel::{Column, IndirectIndex, Resolution};
use janus::StringHash;

use crate::{
    CommonWidgetOptions, ComponentKind, ElementOptions, HasOptions, InteractableCallback,
    InterfaceSystem, Property, WidgetError, WidgetId,
    declare::{UiNodeStack, UiSlider},
};

pub type SliderCallback = InteractableCallback<f32>;

ethel::table_spec! {
    struct InterfaceSlider {
        scroll_step: f32;
        value_cache: f32;
        // if the slider value may be affected by external systems, its value
        // will be polled every frame from the environment before other updates
        value_sync: Option<StringHash>;
        callback: SliderCallback;

        vertical: bool;
    }
}

impl<const LAYERS: usize> InterfaceSystem<LAYERS> {
    pub fn make_slider(
        &mut self,
        root_id: WidgetId,
        scroll_step: f32,
        value_init: f32,
        value_sync: Option<StringHash>,
        callback: SliderCallback,
        vertical: bool,
    ) -> Result<IndirectIndex, WidgetError> {
        if let Some(commons_id) = self.commons.solve_indirect(root_id.0) {
            self.assert_null_archetype(commons_id)?;

            let value_cache = if let Some(sync_v) = value_sync {
                self.environment
                    .get(&sync_v)
                    .and_then(|v| v.as_float())
                    .unwrap_or(value_init)
            } else {
                value_init
            };

            let slider_element = (scroll_step, value_cache, value_sync, callback, vertical);
            let slider_id = self.sliders.insert(slider_element);
            self.commons.archetype[commons_id.as_index()] = ComponentKind::Slider(slider_id);
            Ok(slider_id)
        } else {
            Err(WidgetError::InvalidWidgetHandle(root_id.0.as_int()))
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct SliderOptions {
    pub scroll_step: f32,
    /// Normalized (0.0, 1.0)
    pub value_init: f32,
    pub value_sync_handle: Option<StringHash>,
    pub callback: SliderCallback,
    pub vertical: bool,
}
impl Default for SliderOptions {
    fn default() -> Self {
        Self::new()
    }
}
impl SliderOptions {
    /// 5%
    pub const DEFAULT_SCROLL_STEP: f32 = 0.05;

    pub const fn new() -> Self {
        Self {
            scroll_step: Self::DEFAULT_SCROLL_STEP,
            value_init: 0f32,
            value_sync_handle: None,
            callback: InteractableCallback::None,
            vertical: false,
        }
    }

    pub const fn from_prop<const PROP_ID: u64>(prop: Property<f32, PROP_ID>) -> Self {
        Self {
            value_init: *prop.get(),
            value_sync_handle: Some(Property::<f32, PROP_ID>::name()),
            callback: SliderCallback::Repeating(|env, value| {
                env.insert(Property::<f32, PROP_ID>::name(), *value);
            }),
            scroll_step: Self::DEFAULT_SCROLL_STEP,
            vertical: false,
        }
    }

    pub const fn with_scroll_step(self, scroll_step: f32) -> Self {
        Self {
            scroll_step,
            ..self
        }
    }

    pub const fn with_starting_value(self, value_init: f32) -> Self {
        Self { value_init, ..self }
    }

    pub const fn with_sync_handle(self, value_sync_handle: StringHash) -> Self {
        Self {
            value_sync_handle: Some(value_sync_handle),
            ..self
        }
    }

    pub const fn with_callback(self, callback: SliderCallback) -> Self {
        Self { callback, ..self }
    }

    pub const fn with_vertical(self, vertical: bool) -> Self {
        Self { vertical, ..self }
    }
}

#[derive(Debug)]
pub struct Slider;
impl HasOptions for Slider {
    type Options = SliderOptions;

    fn complement_options(
        common: CommonWidgetOptions,
        this: Self::Options,
        _: Resolution,
    ) -> ElementOptions {
        ElementOptions::Slider(common, this)
    }

    fn archetype() -> ComponentKind {
        ComponentKind::Slider(Default::default())
    }
}

impl UiNodeStack<'_> {
    pub fn slider(&mut self, slider_op: impl FnOnce(UiSlider) -> UiSlider) {
        self.push(slider_op(UiSlider::new(self.reference_resolution)));
    }

    pub fn slider_from(&mut self, slider_options: SliderOptions) {
        self.push(UiSlider::from(slider_options, self.reference_resolution));
    }
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
