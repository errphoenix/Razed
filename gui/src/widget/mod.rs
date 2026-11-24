pub mod button;
pub mod checkbox;
pub mod dropdown;
pub mod image;
pub mod panel;
pub mod slider;
pub mod text;

use std::collections::HashMap;

pub use button::{
    Button, ButtonCallback, ButtonOptions, InterfaceButtonRowTable, InterfaceButtonRowTableView,
};
pub use checkbox::{
    Checkbox, CheckboxOptions, InterfaceCheckboxRowTable, InterfaceCheckboxRowTableView,
};
pub use dropdown::{
    Dropdown, DropdownEntry, DropdownOptions, InterfaceDropdownRowTable,
    InterfaceDropdownRowTableView,
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
use ethel::{IndirectIndex, Resolution};

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
    Slider(IndirectIndex),
    Checkbox(IndirectIndex),
    Dropdown {
        handle: IndirectIndex,
        display_text_handle: WidgetId,
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
            ComponentKind::Slider(_) => write!(f, "slider"),
            ComponentKind::Checkbox(_) => write!(f, "checkbox"),
            ComponentKind::Dropdown { .. } => write!(f, "dropdown-menu"),
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LayoutDisplayModeVisible {
    #[default]
    Block,
    Flex,
    Grid,
}
impl From<taffy::Display> for LayoutDisplayModeVisible {
    fn from(value: taffy::Display) -> Self {
        match value {
            taffy::Display::Block => Self::Block,
            taffy::Display::Flex => Self::Flex,
            taffy::Display::Grid => Self::Grid,
            taffy::Display::None => unimplemented!("only accepts visible display mode"),
        }
    }
}
impl From<LayoutDisplayModeVisible> for taffy::Display {
    fn from(value: LayoutDisplayModeVisible) -> Self {
        match value {
            LayoutDisplayModeVisible::Block => Self::Block,
            LayoutDisplayModeVisible::Flex => Self::Flex,
            LayoutDisplayModeVisible::Grid => Self::Grid,
        }
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
        visible: bool;
        restore_display: LayoutDisplayModeVisible;

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
    pub visible: bool,
}
impl CommonWidgetOptions {
    pub const DEFAULT_LAYER: u32 = 5;

    pub const fn new() -> Self {
        Self {
            parent: None,
            look: None,
            layer: Self::DEFAULT_LAYER,
            layout: LayoutOptions::new(),
            visible: true,
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

    pub const fn with_visible(self, visible: bool) -> Self {
        Self { visible, ..self }
    }
}

pub trait HasOptions {
    type Options: Default + Clone + Copy + std::fmt::Debug;

    fn complement_options(
        common: CommonWidgetOptions,
        this: Self::Options,
        reference_resolution: Resolution,
    ) -> ElementOptions;

    fn archetype() -> ComponentKind;
}

#[derive(Clone, Debug)]
pub enum ElementOptions {
    Panel(CommonWidgetOptions, PanelOptions),
    Button(CommonWidgetOptions, ButtonOptions),
    Text(CommonWidgetOptions, TextOptions),
    Image(CommonWidgetOptions, ImageOptions),
    Slider(CommonWidgetOptions, SliderOptions),
    Checkbox(CommonWidgetOptions, CheckboxOptions),
    Dropdown(CommonWidgetOptions, DropdownOptions),
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

    pub const fn checkbox(core: CommonWidgetOptions, checkbox: CheckboxOptions) -> Self {
        Self::Checkbox(core, checkbox)
    }

    pub const fn dropdown(core: CommonWidgetOptions, dropdown: DropdownOptions) -> Self {
        Self::Dropdown(core, dropdown)
    }

    pub fn common_options(&self) -> &CommonWidgetOptions {
        match self {
            ElementOptions::Panel(core_element_params, _) => core_element_params,
            ElementOptions::Button(core_element_params, _) => core_element_params,
            ElementOptions::Text(core_element_params, _) => core_element_params,
            ElementOptions::Image(core_element_params, _) => core_element_params,
            ElementOptions::Slider(core_element_params, _) => core_element_params,
            ElementOptions::Checkbox(core_element_params, _) => core_element_params,
            ElementOptions::Dropdown(core_element_params, _) => core_element_params,
        }
    }
}

pub type ParentGraph = HashMap<WidgetId, WidgetId>;
pub type ChildrenGraph = HashMap<WidgetId, Vec<WidgetId>>;

#[derive(Clone, Debug, Default)]
pub struct RelationGraph {
    // map child to one parent
    parents: ParentGraph,
    // map parent to many children
    children: ChildrenGraph,

    remove_queries: Vec<WidgetId>,
    removed_propagated: Vec<WidgetId>,
    pub removed_widgets: Vec<WidgetId>,
}
impl RelationGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_parent(&self, id: &WidgetId) -> bool {
        self.parent_of(id).is_some()
    }

    pub fn has_children(&self, id: &WidgetId) -> bool {
        self.children_of(id).is_some()
    }

    pub fn parent_of(&self, id: &WidgetId) -> Option<WidgetId> {
        self.parents.get(id).copied()
    }

    pub fn children_of(&self, id: &WidgetId) -> Option<&[WidgetId]> {
        self.children
            .get(id)
            .map(Vec::as_slice)
            .filter(|v| !v.is_empty())
    }

    pub fn removed(&self) -> &[WidgetId] {
        &self.removed_widgets
    }

    pub fn query_remove(&mut self, id: WidgetId) {
        self.remove_queries.push(id);
    }

    pub fn resolve_removals(&mut self) {
        self.removed_widgets.clear();
        while !self.remove_queries.is_empty() {
            self.resolve_removals_impl();
        }
    }

    fn resolve_removals_impl(&mut self) {
        for node in self.remove_queries.drain(..) {
            self.removed_widgets.push(node);
            self.parents.remove(&node);

            // vector allocation is kept
            if let Some(children) = self.children.get_mut(&node) {
                for child in children.drain(..) {
                    self.removed_propagated.push(child);
                }
            }
        }
        {
            let propagated = self.removed_propagated.drain(..);
            self.remove_queries.extend(propagated);
        }
    }

    pub fn register_parent(&mut self, child: WidgetId, parent: WidgetId) {
        self.parents.insert(child, parent);
    }

    pub fn register_child(&mut self, parent: WidgetId, child: WidgetId) {
        self.children
            .entry(parent)
            .or_insert(Vec::new())
            .push(child);
    }

    pub fn register_children(&mut self, parent: WidgetId, children: &[WidgetId]) {
        self.children
            .entry(parent)
            .or_insert(Vec::new())
            .extend(children);
    }

    pub fn register(
        &mut self,
        subject: WidgetId,
        parent_of_subject: Option<WidgetId>,
        children_of_subject: Option<&[WidgetId]>,
    ) {
        if let Some(parent) = parent_of_subject {
            self.register_parent(subject, parent);
            self.register_child(parent, subject);
        }
        if let Some(children) = children_of_subject {
            self.register_children(subject, children);
            for &child in children {
                self.register_parent(child, subject);
            }
        }
    }

    pub const fn parents(&self) -> &HashMap<WidgetId, WidgetId> {
        &self.parents
    }

    pub const fn children(&self) -> &HashMap<WidgetId, Vec<WidgetId>> {
        &self.children
    }
}
