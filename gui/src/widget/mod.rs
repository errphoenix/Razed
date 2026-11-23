pub mod button;
pub mod image;
pub mod panel;
pub mod slider;
pub mod text;

pub use button::{
    Button, ButtonCallback, ButtonOptions, InterfaceButtonRowTable, InterfaceButtonRowTableView,
};
pub use image::{Image, ImageOptions, InterfaceImageRowTable, InterfaceImageRowTableView};
pub use panel::{
    FloatGrabArea, FloatPanelOptions, FloatState, InterfaceFloatDataRowTable,
    InterfaceFloatDataRowTableView, InterfacePanelRowTable, InterfacePanelRowTableView, Panel,
    PanelOptions,
};
pub use slider::{
    InterfaceSliderRowTable, InterfaceSliderRowTableView, Slider, SliderCallback, SliderOptions,
};
pub use text::{
    InterfaceTextRowTable, InterfaceTextRowTableView, Text, TextContents, TextNode, TextOptions,
};

use crate::{Box2d, LayoutOptions, TaffyNodeId, env::UiEnv, look::LookId};
use ethel::IndirectIndex;

#[derive(Clone, Copy, Default, Debug)]
pub struct InteractionTime {
    pub seconds: f32,
    pub frames: u32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct InteractionData {
    pub hovered: bool,
    pub pressed: bool,
    pub hover_time: InteractionTime,
    pub press_time: InteractionTime,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum InteractableCallback<T: Clone + Copy + Default + std::fmt::Debug> {
    #[default]
    None,
    Once(fn(&mut UiEnv, &mut T)),
    Repeating(fn(&mut UiEnv, &mut T)),
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
        /// text root id
        text_handle: WidgetId,
    },
    Slider {
        handle: IndirectIndex,
        /// text root id, if has a label
        text_handle: Option<WidgetId>,
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
            ComponentKind::Slider { .. } => write!(f, "slider"),
        }
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

ethel::table_spec! {
    struct InterfaceCommon {
        archetype: ComponentKind;
        look: LookId;

        parent: WidgetId;
        taffy_id: TaffyNodeId;

        layout_options: LayoutOptions;
        layer: u32;

        // feedback values from taffy tree after evaluation
        feedback_anchor: glam::Vec2; // top left corner
        feedback_bounds: Box2d;

        interaction_data: InteractionData;
    }
}

#[derive(Clone, Debug, Default)]
pub struct CommonWidgetOptions {
    pub parent: Option<WidgetId>,
    pub look: Option<LookId>,
    pub layout: LayoutOptions,
    pub layer: u32,
}
impl CommonWidgetOptions {
    pub const DEFAULT_LAYER: u32 = 5;

    pub const fn new() -> Self {
        Self {
            parent: None,
            look: None,
            layer: Self::DEFAULT_LAYER,
            layout: LayoutOptions::new(),
        }
    }

    pub const fn with_parent(self, parent: WidgetId) -> Self {
        Self {
            parent: Some(parent),
            ..self
        }
    }

    pub const fn with_look(self, look: LookId) -> Self {
        Self {
            look: Some(look),
            ..self
        }
    }

    pub const fn with_layout(self, layout: LayoutOptions) -> Self {
        Self { layout, ..self }
    }

    pub const fn with_layer(self, layer: u32) -> Self {
        Self { layer, ..self }
    }
}

pub trait HasOptions {
    type Options: Default + Clone + Copy + std::fmt::Debug;

    fn complement_options(common: CommonWidgetOptions, this: Self::Options) -> ElementOptions;

    fn archetype() -> ComponentKind;
}

#[derive(Clone, Debug)]
pub enum ElementOptions {
    Panel(CommonWidgetOptions, PanelOptions),
    Button(CommonWidgetOptions, ButtonOptions),
    Text(CommonWidgetOptions, TextOptions),
    Image(CommonWidgetOptions, ImageOptions),
    Slider(CommonWidgetOptions, SliderOptions),
}
impl ElementOptions {
    pub const fn panel(core: CommonWidgetOptions, panel: PanelOptions) -> Self {
        Self::Panel(core, panel)
    }

    pub const fn button(core: CommonWidgetOptions, button: ButtonOptions) -> Self {
        Self::Button(core, button)
    }

    pub const fn text(core: CommonWidgetOptions, text: TextOptions) -> Self {
        Self::Text(core, text)
    }

    pub const fn image(core: CommonWidgetOptions, image: ImageOptions) -> Self {
        Self::Image(core, image)
    }

    pub const fn slider(core: CommonWidgetOptions, slider: SliderOptions) -> Self {
        Self::Slider(core, slider)
    }

    pub fn common_options(&self) -> &CommonWidgetOptions {
        match self {
            ElementOptions::Panel(core_element_params, _) => core_element_params,
            ElementOptions::Button(core_element_params, _) => core_element_params,
            ElementOptions::Text(core_element_params, _) => core_element_params,
            ElementOptions::Image(core_element_params, _) => core_element_params,
            ElementOptions::Slider(core_element_params, _) => core_element_params,
        }
    }
}
