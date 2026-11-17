use std::{
    fmt::{Debug, Write},
    sync::RwLock,
};

use cosmic_text::FontSystem;
use ethel::{
    assets::{AssetMetadataRegistry, TextureId, TextureMetadata},
    render::Resolution,
    state::{
        InputEvent,
        data::{Column, DirectIndex, IndirectIndex},
    },
};

use janus::{
    StringHash,
    context::DeltaTime,
    input::{Cursor, KeyEvent},
};

pub mod declare;
pub mod draw;
pub mod env;
pub mod look;
pub mod options;
pub mod render;
pub mod style;
pub mod text;

#[cfg(feature = "animation")]
pub mod animation;

use rendrs::batch::Batch;
pub use style::*;

use taffy::prelude::*;

use crate::{
    draw::{BatchingLayerCompositor, InterfaceAggregator, InterfaceObject, Quad},
    env::{EnvValue, UiEnv},
    look::{LookId, LooksManager},
    text::{FontMetrics, GlyphAtlas, TextComposer, TextMeasurement, font::FontLibrary},
};

#[allow(unused_imports)]
pub use options::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct TaffyNodeId(pub(crate) NodeId);

impl TaffyNodeId {
    pub fn is_null(self) -> bool {
        self.0 == NodeId::new(0)
    }
}

impl Default for TaffyNodeId {
    fn default() -> Self {
        Self(NodeId::new(0))
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct InteractionTime {
    pub seconds: f32,
    pub frames: u32,
}

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

#[derive(Clone, Copy, Debug, Default)]
pub struct InteractionData {
    pub hovered: bool,
    pub pressed: bool,
    pub hover_time: InteractionTime,
    pub press_time: InteractionTime,
}

ethel::table_spec! {
    struct InterfaceCommon {
        archetype: ComponentKind;
        look: LookId;

        parent: WidgetId;
        children: Vec<WidgetId>;
        taffy_id: TaffyNodeId;

        layout_options: LayoutOptions;
        layer: u32;

        // feedback values from taffy tree after evaluation
        feedback_anchor: glam::Vec2; // top left corner
        feedback_bounds: Box2d;

        interaction_data: InteractionData;
    }
}
ethel::table_spec! {
    struct InterfacePanel {
        root_rvid: WidgetId;

        hidden: bool;

        // index in InterfaceFloatData table
        floating: Option<IndirectIndex>;
    }
}
ethel::table_spec! {
    struct InterfaceFloatData {
        grab_area: FloatGrabArea;
        offset: glam::Vec2;
        state: FloatState;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum FloatGrabArea {
    SectionHoriz { height: f32 },
    SectionVert { width: f32 },
    Corner { width: f32, height: f32 },
}
impl FloatGrabArea {
    pub const DEFAULT_HEIGHT: f32 = 22f32;
}
impl Default for FloatGrabArea {
    fn default() -> Self {
        Self::SectionHoriz {
            height: Self::DEFAULT_HEIGHT,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Default)]
pub enum FloatState {
    #[default]
    Idle,
    Active,
    Released,
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
ethel::table_spec! {
    struct InterfaceImage {
        tint: glam::Vec4;
        opacity: f32;
        texture: TextureId;
    }
}
ethel::table_spec! {
    struct InterfaceButton {
        // handle in text table
        text_id: IndirectIndex;
        callback: ButtonCallback;
    }
}
ethel::table_spec! {
    struct InterfaceSlider {
        // handle in text table, if has label
        text_id: Option<IndirectIndex>;

        scroll_step: f32;
        value_cache: f32;
        // if the slider value may be affected by external systems, its value
        // will be polled every frame from the environment before other updates
        value_sync: Option<StringHash>;
        callback: SliderCallback;

        vertical: bool;
    }
}

pub type ButtonCallback = InteractableCallback<InteractionTime>;
pub type SliderCallback = InteractableCallback<f32>;

#[derive(Clone, Copy, Debug, Default)]
pub enum InteractableCallback<T: Clone + Copy + Debug + Default> {
    #[default]
    None,
    Once(fn(&mut UiEnv, &mut T)),
    Repeating(fn(&mut UiEnv, &mut T)),
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

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Box2d {
    min: glam::Vec2,
    max: glam::Vec2,
}
impl Box2d {
    pub const NULL: Self = Box2d::new(glam::Vec2::ZERO, glam::Vec2::ZERO);
    pub const UNIT: Self = Box2d::new(glam::Vec2::ONE, glam::Vec2::ONE);

    pub const fn new(min: glam::Vec2, max: glam::Vec2) -> Self {
        Self { min, max }
    }

    pub const fn from_center(center: glam::Vec2, extents: glam::Vec2) -> Self {
        Self {
            min: glam::vec2(center.x - extents.x, center.y - extents.y),
            max: glam::vec2(center.x + extents.x, center.y + extents.y),
        }
    }

    pub const fn min(&self) -> glam::Vec2 {
        self.min
    }

    pub const fn max(&self) -> glam::Vec2 {
        self.max
    }

    pub const fn center(&self) -> glam::Vec2 {
        glam::vec2(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
        )
    }

    pub const fn size(&self) -> glam::Vec2 {
        glam::vec2(self.max.x - self.min.x, self.max.y - self.min.y)
    }

    pub const fn translate(&mut self, translation: glam::Vec2) {
        self.min.x += translation.x;
        self.min.y += translation.y;
        self.max.x += translation.x;
        self.max.y += translation.y;
    }

    pub const fn make_center(&mut self) -> glam::Vec2 {
        let center = self.center();
        self.min.x -= center.x;
        self.min.y -= center.y;
        self.max.x -= center.x;
        self.max.y -= center.y;
        center
    }

    /// See [`Self::scale`].
    pub fn scale_uniform(&mut self, scaling: f32, anchor: Option<glam::Vec2>) {
        self.scale(glam::Vec2::splat(scaling), anchor);
    }

    /// Scale the 2d box by a given `scaling` respective to an `anchor`.
    ///
    /// The `anchor` must be of length [-1, 1]. A `None` value will default to
    /// a (0, 0) point, which is the local center of the box.
    ///
    /// The length of `anchor` is relative to the half size of the box, which
    /// is centered around (0, 0). For example, an `anchor` of (1, 0.5) for a
    /// box of size (400, 200) equals to an `anchor` point of (200, 50).
    pub fn scale(&mut self, scaling: glam::Vec2, anchor: Option<glam::Vec2>) {
        let anchor = anchor.unwrap_or_default() * self.size();
        let offset = self.make_center() - anchor;
        self.translate(anchor);
        self.min.x *= scaling.x;
        self.min.y *= scaling.y;
        self.max.x *= scaling.x;
        self.max.y *= scaling.y;
        self.translate(offset);
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x > self.min.x && y > self.min.y && x < self.max.x && y < self.max.y
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeJointId {
    pub table_id: WidgetId,
    #[cfg(feature = "taffy")]
    pub tree_id: NodeId,
}

#[derive(Debug, thiserror::Error)]
pub enum WidgetError {
    #[error("archetype '{0}' is already defined")]
    ArchetypeAlreadyDefined(ComponentKind),

    #[error("parent by given ID {0} not found")]
    ParentNotFound(WidgetId),

    #[error("child by given ID {0} not found")]
    ChildNotFound(WidgetId),

    #[error("taffy layout error: {0}")]
    TaffyLayoutError(taffy::TaffyError),

    #[error("invalid widget handle {0}: cannot be resolved")]
    InvalidWidgetHandle(u32),
}

#[derive(Debug)]
pub struct InterfaceSystem<const LAYERS: usize = 10> {
    layout: TaffyTree<WidgetId>,
    root_node: NodeJointId,

    resolution: Resolution,

    pub commons: InterfaceCommonRowTable,
    pub panels: InterfacePanelRowTable,
    pub floating: InterfaceFloatDataRowTable,
    pub texts: InterfaceTextRowTable,
    pub images: InterfaceImageRowTable,
    pub buttons: InterfaceButtonRowTable,
    pub sliders: InterfaceSliderRowTable,

    intermediate_buffer: Vec<InterfaceObject>,
    compositor: BatchingLayerCompositor<LAYERS>,

    text_composer: TextComposer,
    font_library: FontLibrary,

    pub environment: UiEnv,
    text_resolve_buf: String,

    /// list of text elements that may invalidate
    /// these will require a relayout if the contents
    /// change, which is expensive
    text_dynamic_list: Vec<WidgetId>,

    /// if cursor is currently grabbing a floating panel
    /// other cursor events should be ignored if true
    /// indirect into 'floating' table
    grabbing: Option<IndirectIndex>,

    looks_manager: LooksManager,
}
/// Safety:
/// TaffyTree is !Send and !SYnc due to internal implementation details
/// related to raw const* function pointers. This implementation is required
/// for ethel's threads initialization.
unsafe impl<const LAYERS: usize> Send for InterfaceSystem<LAYERS> {}
unsafe impl<const LAYERS: usize> Sync for InterfaceSystem<LAYERS> {}
impl<const LAYERS: usize> InterfaceSystem<LAYERS> {
    pub fn new(resolution: Resolution) -> Self {
        Self::with_environment(resolution, UiEnv::new())
    }

    pub fn with_environment(resolution: Resolution, environment: UiEnv) -> Self {
        let mut layout = TaffyTree::with_capacity(1);
        const ROOT_ID: WidgetId = WidgetId(IndirectIndex::null(0));

        // for some reason taffy defaults the layout strategy to flexbox if
        // the feature is enabled, which is kinda silly?
        // we explicitly default to block
        let default_style = {
            let mut ds = Style::default();
            ds.display = Display::Block;
            ds
        };

        let tree_id = layout
            .new_leaf_with_context(default_style, ROOT_ID)
            .unwrap();

        let root_node = NodeJointId {
            table_id: ROOT_ID,
            #[cfg(feature = "taffy")]
            tree_id,
        };

        // initialise root taffy id in data table
        let mut commons = InterfaceCommonRowTable::new();
        commons.taffy_id[ROOT_ID.0.as_index()] = TaffyNodeId(tree_id);

        Self {
            layout,
            root_node,
            resolution,
            commons,
            panels: InterfacePanelRowTable::new(),
            floating: InterfaceFloatDataRowTable::new(),
            texts: InterfaceTextRowTable::new(),
            images: InterfaceImageRowTable::new(),
            buttons: InterfaceButtonRowTable::new(),
            sliders: InterfaceSliderRowTable::new(),
            intermediate_buffer: Vec::new(),
            compositor: BatchingLayerCompositor::new(),
            text_composer: TextComposer::new(),
            font_library: FontLibrary::new(),
            environment,
            text_resolve_buf: String::new(),
            text_dynamic_list: Vec::new(),
            grabbing: None,
            looks_manager: LooksManager::default(),
        }
    }

    pub fn set_look_manager(&mut self, look_manager: LooksManager) {
        self.looks_manager = look_manager;
    }

    pub const fn is_grabbing(&self) -> bool {
        self.grabbing.is_some()
    }

    /// Indirect into 'floating' table
    pub const fn grabbing(&self) -> Option<IndirectIndex> {
        self.grabbing
    }

    pub const fn looks(&self) -> &LooksManager {
        &self.looks_manager
    }

    pub const fn looks_mut(&mut self) -> &mut LooksManager {
        &mut self.looks_manager
    }

    pub const fn env(&self) -> &UiEnv {
        &self.environment
    }

    pub const fn env_mut(&mut self) -> &mut UiEnv {
        &mut self.environment
    }

    pub const fn font_library(&mut self) -> &mut FontLibrary {
        &mut self.font_library
    }

    pub const fn resolution(&self) -> Resolution {
        self.resolution
    }

    /// Update the internal `resolution` of the interface layout.
    ///
    /// # Returns
    /// Returns the previously stored `Resolution`.
    pub const fn set_resolution(&mut self, resolution: Resolution) -> Resolution {
        std::mem::replace(&mut self.resolution, resolution)
    }

    pub const fn root_id(&self) -> NodeJointId {
        self.root_node
    }

    #[cfg(feature = "taffy")]
    pub const fn taffy_layout(&self) -> &TaffyTree<WidgetId> {
        &self.layout
    }

    pub fn parent_of(&self, widget: WidgetId) -> Option<WidgetId> {
        if let Some(direct) = self.commons.solve_indirect(widget.0) {
            self.commons.parent.get(direct.as_index()).copied()
        } else {
            None
        }
    }

    pub fn has_parent(&self, widget: WidgetId) -> Option<bool> {
        self.parent_of(widget).map(|id| !id.is_null())
    }

    pub fn prepare_elements(&mut self, glyph_atlas: &mut GlyphAtlas) {
        let mut aggregator = InterfaceAggregator {
            environment: &self.environment,
            looks: &self.looks_manager,
            text_resolve_buf: &mut self.text_resolve_buf,
            commons: InterfaceCommonRowTableView::from(&self.commons),
            panels: InterfacePanelRowTableView::from(&self.panels),
            floating: InterfaceFloatDataRowTableView::from(&self.floating),
            texts: InterfaceTextRowTableView::from(&self.texts),
            images: InterfaceImageRowTableView::from(&self.images),
            buttons: InterfaceButtonRowTableView::from(&self.buttons),
            sliders: InterfaceSliderRowTableView::from(&self.sliders),
        };
        aggregator.gather_quad_elements(
            &mut self.text_composer,
            glyph_atlas,
            &mut self.intermediate_buffer,
        );
    }

    pub fn composite_layers(&mut self, registry: &AssetMetadataRegistry<TextureMetadata>) {
        self.intermediate_buffer.drain(..).for_each(|object| {
            self.compositor.insert(object, registry);
        });
    }

    pub fn batches(&self) -> impl Iterator<Item = &Batch<Quad>> {
        self.compositor.batches()
    }

    pub fn clear_compositor_layers(&mut self) {
        self.compositor.clear_layers();
    }

    pub fn compositor(&self) -> &BatchingLayerCompositor<LAYERS> {
        &self.compositor
    }

    pub fn compositor_mut(&mut self) -> &mut BatchingLayerCompositor<LAYERS> {
        &mut self.compositor
    }

    /// Poll environment changes made by external systems.
    ///
    /// Must run before interface input updates to ensure correct
    /// behaviour.
    pub fn poll_environment_changes(&mut self) {
        let env = &self.environment;
        {
            let sliders = &mut self.sliders;
            let caches = &mut sliders.value_cache;
            let handles = &sliders.value_sync;
            for (id, cache) in handles.iter().zip(caches).filter(|(id, _)| id.is_some()) {
                let id = id.as_ref().unwrap();
                let env_value = env.get(id).and_then(|v| v.as_float());
                *cache = env_value.unwrap_or_default();
            }
        }
    }

    /// Process floating events and interactions.
    ///
    /// # Returns
    /// Returns the ID of the element that may require layout
    /// re-evaluation (with [`Self::evaluate_layout_node`]), if any.
    pub fn process_floating(&mut self, cursor: &Cursor) -> Option<WidgetId> {
        let panels_count = self.panels.len();
        let float_ids = &self.panels.floating;

        let (x, y, dx, dy) = {
            let c = cursor.current_f32();
            let d = cursor.delta_f32();
            (c.0, c.1, d.0, d.1)
        };

        for i in (1..panels_count).rev() {
            if let Some(float_id) = float_ids[i]
                && let Some(fdid) = self.floating.solve_indirect(float_id)
            {
                let fdid = fdid.as_index();
                let roots = &self.panels.root_rvid;
                let float_bounds = &self.floating.grab_area;
                let float_offset = &mut self.floating.offset;
                let float_state = &mut self.floating.state;

                let root = self.commons.solve_indirect(roots[i].0).unwrap();
                let layouts = &mut self.commons.layout_options;
                let interact = self.commons.interaction_data[root.as_index()];

                match float_state[fdid] {
                    FloatState::Idle => {
                        if interact.pressed && self.grabbing.is_none() {
                            let bounds = {
                                let p_bounds = self.commons.feedback_bounds[root.as_index()];
                                let (min, max) = (p_bounds.min, p_bounds.max);
                                let (min, max) = match float_bounds[fdid] {
                                    FloatGrabArea::SectionHoriz { height } => {
                                        (min, glam::vec2(max.x, min.y + height))
                                    }
                                    FloatGrabArea::SectionVert { width } => {
                                        (min, glam::vec2(min.x + width, max.y))
                                    }
                                    FloatGrabArea::Corner { width, height } => {
                                        (min, min + glam::vec2(width, height))
                                    }
                                };
                                Box2d::new(min, max)
                            };

                            if bounds.contains(x, y) {
                                self.grabbing = Some(float_id);
                                float_state[fdid] = FloatState::Active;
                            }
                        }
                    }
                    FloatState::Active => {
                        if let Some(grab_id) = self.grabbing
                            && grab_id == float_id
                        {
                            float_offset[fdid] += glam::vec2(dx, dy);
                        } else {
                            float_state[fdid] = FloatState::Released;
                        }
                    }
                    FloatState::Released => {
                        let layout = &mut layouts[root.as_index()];
                        let p_anchor = self.commons.feedback_anchor[root.as_index()];

                        let offset = &mut float_offset[fdid];
                        layout.layout_position = LayoutPosition::Absolute {
                            x: Some(Value::Absolute(p_anchor.x + offset.x)),
                            y: Some(Value::Absolute(p_anchor.y + offset.y)),
                        };

                        *offset = glam::Vec2::ZERO;
                        float_state[fdid] = FloatState::Idle;

                        return Some(roots[i]);
                    }
                }
            }
        }
        None
    }

    pub fn process_widget_states(&mut self, delta: DeltaTime) {
        let delta = delta.as_f32();
        let count = self.commons.len();

        let archetypes = &self.commons.archetype;
        let interacts = &mut self.commons.interaction_data;

        let button_callbacks = &self.buttons.callback;
        let slider_callbacks = &self.sliders.callback;

        for i in (1..count).rev() {
            let archetype = archetypes[i];
            let interact = &mut interacts[i];

            match archetype {
                ComponentKind::Button { handle, .. } => {
                    if interact.pressed {
                        let direct = self.buttons.solve_indirect(handle).unwrap();
                        let callback = button_callbacks[direct.as_index()];
                        let press_time = &mut interact.press_time;

                        match callback {
                            InteractableCallback::None => {}
                            e => match e {
                                InteractableCallback::Once(cb) => {
                                    if press_time.frames == 1 {
                                        cb(&mut self.environment, press_time)
                                    }
                                }
                                InteractableCallback::Repeating(cb) => {
                                    cb(&mut self.environment, press_time)
                                }
                                _ => unreachable!(),
                            },
                        }
                    }
                }
                ComponentKind::Slider { handle, .. } => {
                    let direct = self.sliders.solve_indirect(handle).unwrap();
                    let callback = slider_callbacks[direct.as_index()];
                    let value = &mut self.sliders.value_cache[direct.as_index()];

                    match callback {
                        InteractableCallback::None => {}
                        InteractableCallback::Once(cb) => {
                            // only call once slider is release
                            if !interact.pressed {
                                cb(&mut self.environment, value)
                            }
                        }
                        InteractableCallback::Repeating(cb) => {
                            // call contiuously as slider changes
                            if interact.pressed {
                                cb(&mut self.environment, value)
                            }
                        }
                    }
                }

                _ => {}
            }

            if interact.pressed {
                interact.press_time.frames += 1;
                interact.press_time.seconds += delta;
            } else {
                interact.press_time.frames = 0;
                interact.press_time.seconds = 0.0;
            }
            if interact.hovered {
                interact.hover_time.frames += 1;
                interact.hover_time.seconds += delta;
            } else {
                interact.hover_time.frames = 0;
                interact.hover_time.seconds = 0.0;
                interact.pressed = false;
            }
        }
    }

    pub fn process_hover_events(&mut self, x: f32, y: f32, delta: DeltaTime) {
        let delta = delta.as_f32();

        let count = self.commons.len();
        let bounds = &self.commons.feedback_bounds;
        let interacts = &mut self.commons.interaction_data;

        for i in (1..count).rev() {
            let bounds = bounds[i];
            let interact = &mut interacts[i];
            if bounds.contains(x, y) {
                interact.hovered = true;
                interact.hover_time.frames += 1;
                interact.hover_time.seconds += delta;
            } else {
                interact.hovered = false;
                interact.hover_time.frames = 0;
                interact.hover_time.seconds = 0.0;
            }
        }
    }

    fn process_key_input(&mut self, table_index: usize, event: KeyEvent, delta: DeltaTime) -> bool {
        let delta = delta.as_f32();
        let interact = &mut self.commons.interaction_data[table_index];

        // keyboard text (todo)
        {}

        // click (only kb), hold, release
        {
            const PRESS_KEY: u16 = janus::input::KeyCode::Space as u16;
            let click = matches!(
                event,
                KeyEvent::Keyboard {
                    code: PRESS_KEY,
                    release: false,
                    press_time: 1
                }
            );

            let press = interact.hovered & click;
            interact.pressed = press;
            if press {
                interact.press_time.frames += 1;
                interact.press_time.seconds += delta;
            } else {
                interact.press_time.frames = 0;
                interact.press_time.seconds = 0.0;
            }

            return press;
        }
    }

    fn process_scroll_input(&mut self, table_index: usize, scroll_delta: f32) {
        let interact = &mut self.commons.interaction_data[table_index];
        if !interact.hovered {
            return;
        };

        if let ComponentKind::Slider { handle, .. } = self.commons.archetype[table_index] {
            let slider_index = self.sliders.solve_indirect(handle).unwrap().as_index();
            let vd = self.sliders.scroll_step[slider_index] * scroll_delta;
            let v = self.sliders.value_cache[slider_index];
            self.sliders.value_cache[slider_index] = (v + vd).clamp(0.0, 1.0);

            if interact.press_time.frames == 0 {
                interact.pressed = true;
            }
        }
    }

    pub fn feed_input(
        &mut self,
        events: &[InputEvent],
        scroll_delta: f32,
        delta: DeltaTime,
    ) -> bool {
        const INTERACT_KEY_KB: u16 = janus::input::KeyCode::Space as u16;
        for &ev in events {
            if ev.key_and(|ev| {
                matches!(
                    ev,
                    KeyEvent::Keyboard {
                        code: INTERACT_KEY_KB,
                        release: true,
                        ..
                    }
                )
            }) {
                self.grabbing = None;
            }
        }

        let mut consumed = false;

        let count = self.commons.len();
        // start at 2 to skip null node at 0 and root node at 1
        for i in (2..count).rev() {
            for event in events {
                if let Some(event) = event.key() {
                    if self.process_key_input(i, event, delta) {
                        consumed = true;
                    }
                }
            }
            self.process_scroll_input(i, scroll_delta);
        }

        consumed
    }

    pub fn is_root(&self, id: WidgetId) -> bool {
        self.root_id().table_id == id
    }

    pub fn rebind_layout(&mut self, id: WidgetId) {
        if let Some(did) = self.commons.solve_indirect(id.0) {
            let layout = &self.commons.layout_options[did.as_index()];
            let taffy_id = self.commons.taffy_id[did.as_index()];
            let _ = self.layout.set_style(taffy_id.0, layout.into_taffy_style());
        }
    }

    fn impl_synchronise_layout(&mut self, did: DirectIndex) {
        let taffy_id = self.commons.taffy_id[did.as_index()];
        if taffy_id.is_null() {
            return;
        }

        let parent = self.commons.parent[did.as_index()];
        let (offset_x, offset_y) = if !self.is_root(parent) {
            let direct = unsafe { self.commons.solve_indirect_unchecked(parent.0) };
            let position = self.commons.feedback_anchor[direct.as_index()];
            (position.x, position.y)
        } else {
            (0.0, 0.0)
        };

        let fb_anchor = &mut self.commons.feedback_anchor[did.as_index()];
        let fb_bounds = &mut self.commons.feedback_bounds[did.as_index()];
        let taffy_id = taffy_id.0;

        let node = self.layout.get_final_layout(taffy_id);
        let position = node.location;
        let size = node.size;
        let position_x = position.x + offset_x;
        let position_y = position.y + offset_y;
        let min = glam::vec2(position_x, position_y);
        let mut max = glam::vec2(position_x + size.width, position_y + size.height);

        // correct axis if slider is vertical
        if let ComponentKind::Slider {
            handle: slider_id, ..
        } = self.commons.archetype[did.as_index()]
        {
            let sdid = self.sliders.solve_indirect(slider_id).unwrap();
            if self.sliders.vertical[sdid.as_index()] {
                let mut size = max - min;
                std::mem::swap(&mut size.x, &mut size.y);
                max = min + size;
            }
        }

        *fb_anchor = glam::vec2(position_x, position_y);
        *fb_bounds = Box2d { min, max };
    }

    pub fn synchronise_layout_node(&mut self, node: WidgetId) {
        if let Some(did) = self.commons.solve_indirect(node.0) {
            self.impl_synchronise_layout(did);
        }
    }

    pub fn synchronise_layout(&mut self) {
        let count = self.commons.len();
        for i in 1..count {
            self.impl_synchronise_layout(DirectIndex::from_index(i, 0));
        }
    }

    pub fn evaluate_layout_node(&mut self, node: WidgetId, force: bool) {
        if let Some(cdid) = self.commons.solve_indirect(node.0) {
            let taffy_id = self.commons.taffy_id[cdid.as_index()];
            if taffy_id.is_null() {
                return;
            }
            self.impl_evaluate_layout(taffy_id, force);
        }
    }

    fn impl_evaluate_layout(&mut self, taffy_id: TaffyNodeId, force: bool) {
        let available = Size {
            width: AvailableSpace::Definite(self.resolution.width),
            height: AvailableSpace::Definite(self.resolution.height),
        };
        if force {
            let _ = self.layout.mark_dirty(taffy_id.0);
        }
        self.layout
            .compute_layout_with_measure(
                taffy_id.0,
                available,
                |known_size, available, _id, ctx, _style| {
                    let id = ctx.expect("node has no associated widget with it");
                    let did = self.commons.solve_indirect(id.0).unwrap();
                    let archetype = self.commons.archetype[did.as_index()];
                    if let ComponentKind::Text(text_id) = archetype {
                        let width = known_size.width.or(available.width.into_option());
                        let height = known_size.height.or(available.height.into_option());
                        let tdid = self
                            .texts
                            .solve_indirect(text_id)
                            .expect("text id must be valid")
                            .as_index();

                        let contents = self.texts.contents[tdid];
                        let metrics = self.texts.metrics[tdid];
                        let font = self.texts.font_name[tdid];

                        let text_buf = &mut self.text_resolve_buf;
                        text_buf.clear();
                        contents.resolve(&self.environment, text_buf);

                        self.text_composer.set_buffer_size(width, height);
                        self.text_composer.set_font_metrics(metrics);
                        self.text_composer.set_font(font);
                        self.text_composer.set_text(text_buf);
                        let measurement = self.text_composer.measure();

                        self.texts.measure[tdid] = measurement;

                        const TEXT_LAYOUT_PADDING: f32 = 1.0;
                        return Size {
                            width: measurement.width + TEXT_LAYOUT_PADDING,
                            height: measurement.height + TEXT_LAYOUT_PADDING,
                        };
                    }
                    Size::zero()
                },
            )
            .expect("failed to evaluate taffy layout");
    }

    /// Evaluates the layout if necessary or `force`d by the caller.
    pub fn evaluate_layout(&mut self, force: bool) {
        self.impl_evaluate_layout(TaffyNodeId(self.root_node.tree_id), force);
    }

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

    pub fn bind_system_fonts(&mut self) {
        self.text_composer.set_font_system(FontSystem::new());
    }

    pub fn text_composer(&mut self) -> &mut TextComposer {
        &mut self.text_composer
    }

    fn assert_null_archetype(&self, root_id: DirectIndex) -> Result<(), WidgetError> {
        let archetype = self.commons.archetype[root_id.as_index()];
        if matches!(archetype, ComponentKind::Null) {
            Ok(())
        } else {
            Err(WidgetError::ArchetypeAlreadyDefined(archetype))
        }
    }

    pub fn mark_dirty(&mut self, id: WidgetId) {
        if let Some(commons_id) = self.commons.solve_indirect(id.0) {
            let taffy_id = self.commons.taffy_id[commons_id.as_index()];
            self.layout.mark_dirty(taffy_id.0).expect("never fails");
        }
    }

    /// Make a core element into a panel element.
    ///
    /// This requires the core element's `id`.
    ///
    /// # Returns
    /// Returns the `IndirectIndex` of the panel data.
    /// Or returns `None` if the given core `id` is invalid.
    pub fn make_panel(
        &mut self,
        id: WidgetId,
        hidden: bool,
        float_params: Option<FloatParams>,
    ) -> Result<IndirectIndex, WidgetError> {
        if let Some(commons_id) = self.commons.solve_indirect(id.0) {
            self.assert_null_archetype(commons_id)?;

            let float_id = float_params.map(|params| {
                if let Some(base_pos) = params.base_pos {
                    let cdid = commons_id.as_index();
                    self.commons.feedback_anchor[cdid] = base_pos;
                    let layout = &mut self.commons.layout_options[cdid];
                    layout.layout_position = LayoutPosition::Absolute {
                        x: Some(Value::Absolute(base_pos.x)),
                        y: Some(Value::Absolute(base_pos.y)),
                    };
                }

                self.floating
                    .insert((params.grab_area, glam::Vec2::ZERO, FloatState::Idle))
            });

            let panel_element = (id, hidden, float_id);
            let panel_id = self.panels.insert(panel_element);
            self.commons.archetype[commons_id.as_index()] = ComponentKind::Panel(panel_id);
            Ok(panel_id)
        } else {
            Err(WidgetError::InvalidWidgetHandle(id.0.as_int()))
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

    /// Make a core element into an image element.
    ///
    /// This requires the core element's `id`.
    ///
    /// # Returns
    /// Returns the `IndirectIndex` of the image data.
    /// Or returns `None` if the given core `id` is invalid.
    pub fn make_image(
        &mut self,
        id: WidgetId,
        tint: glam::Vec4,
        opacity: f32,
        texture: TextureId,
    ) -> Result<IndirectIndex, WidgetError> {
        if let Some(commons_id) = self.commons.solve_indirect(id.0) {
            self.assert_null_archetype(commons_id)?;
            let image_element = (tint, opacity, texture);
            let image_id = self.images.insert(image_element);
            self.commons.archetype[commons_id.as_index()] = ComponentKind::Image(image_id);
            Ok(image_id)
        } else {
            Err(WidgetError::InvalidWidgetHandle(id.0.as_int()))
        }
    }

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
        if let Some(commons_id) = self.commons.solve_indirect(root_id.0) {
            self.assert_null_archetype(commons_id)?;
            let button_element = (text_id, callback);
            let button_id = self.buttons.insert(button_element);
            self.commons.archetype[commons_id.as_index()] = ComponentKind::Button {
                handle: button_id,
                text_handle: text_root_id,
            };
            Ok(button_id)
        } else {
            Err(WidgetError::InvalidWidgetHandle(root_id.0.as_int()))
        }
    }

    pub fn make_slider(
        &mut self,
        root_id: WidgetId,
        text_ids: Option<(WidgetId, IndirectIndex)>,
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

            let slider_element = (
                text_ids.map(|(_, id)| id),
                scroll_step,
                value_cache,
                value_sync,
                callback,
                vertical,
            );
            let slider_id = self.sliders.insert(slider_element);
            self.commons.archetype[commons_id.as_index()] = ComponentKind::Slider {
                handle: slider_id,
                text_handle: text_ids.map(|(rid, _)| rid),
            };
            Ok(slider_id)
        } else {
            Err(WidgetError::InvalidWidgetHandle(root_id.0.as_int()))
        }
    }

    /// Create a new empty or core element.
    ///
    /// This returns a new [`WidgetId`] which can be used to create
    /// specific element types through [`Self::make_panel`],
    /// [`Self::make_text`], [`Self::make_image`], or [`Self::make_button`].
    pub fn add_new(
        &mut self,
        parent: Option<WidgetId>,
        children: Option<&[WidgetId]>,
        look_id: Option<impl Into<LookId>>,
        layout_options: LayoutOptions,
        layer: u32,
    ) -> Result<WidgetId, WidgetError> {
        let taffy_style = layout_options.into_taffy_style();
        let node_id = self
            .layout
            .new_leaf(taffy_style)
            .map_err(WidgetError::TaffyLayoutError)?;

        let parent = parent.unwrap_or(self.root_node.table_id);

        // add this to parent
        {
            let parent_direct = self
                .commons
                .solve_indirect(parent.0)
                .ok_or(WidgetError::ParentNotFound(parent))?;
            let parent_taffy_id = self.commons.taffy_id[parent_direct.as_index()];
            self.layout
                .add_child(parent_taffy_id.0, node_id)
                .map_err(WidgetError::TaffyLayoutError)?;
        }
        // add children to this
        let children = if let Some(children) = children {
            children.iter().try_for_each(|child| {
                if let Some(direct) = self.commons.solve_indirect(child.0) {
                    let taffy_id = self.commons.taffy_id[direct.as_index()];

                    self.layout
                        .add_child(node_id, taffy_id.0)
                        .map_err(WidgetError::TaffyLayoutError)
                } else {
                    Err(WidgetError::ChildNotFound(*child))
                }
            })?;
            children.to_vec()
        } else {
            // does not allocate
            Vec::new()
        };

        let id = WidgetId(self.commons.insert((
            ComponentKind::Null,
            look_id.map(Into::into).unwrap_or_default(),
            parent,
            children,
            TaffyNodeId(node_id),
            layout_options,
            layer.min(LAYERS as u32),
            // init default feedback values
            glam::Vec2::ZERO,
            Box2d::NULL,
            InteractionData::default(),
        )));

        self.layout
            .set_node_context(node_id, Some(id))
            .map_err(WidgetError::TaffyLayoutError)?;

        Ok(id)
    }

    pub const fn core_data(&self) -> &InterfaceCommonRowTable {
        &self.commons
    }

    pub const fn core_data_mut(&mut self) -> &mut InterfaceCommonRowTable {
        &mut self.commons
    }

    pub const fn panel_data(&self) -> &InterfacePanelRowTable {
        &self.panels
    }

    pub const fn panel_data_mut(&mut self) -> &mut InterfacePanelRowTable {
        &mut self.panels
    }

    pub const fn image_data(&self) -> &InterfaceImageRowTable {
        &self.images
    }

    pub const fn image_data_mut(&mut self) -> &mut InterfaceImageRowTable {
        &mut self.images
    }

    pub const fn text_data(&self) -> &InterfaceTextRowTable {
        &self.texts
    }

    pub const fn text_data_mut(&mut self) -> &mut InterfaceTextRowTable {
        &mut self.texts
    }

    pub const fn button_data(&self) -> &InterfaceButtonRowTable {
        &self.buttons
    }

    pub const fn button_data_mut(&mut self) -> &mut InterfaceButtonRowTable {
        &mut self.buttons
    }

    pub const fn slider_data(&self) -> &InterfaceSliderRowTable {
        &self.sliders
    }

    pub const fn slider_data_mut(&mut self) -> &mut InterfaceSliderRowTable {
        &mut self.sliders
    }

    pub fn create_element(
        &mut self,
        parameters: ElementParams,
    ) -> Result<(WidgetId, IndirectIndex), WidgetError> {
        let root_id = {
            let core = parameters.core();
            self.add_new(
                core.parent,
                core.children,
                core.look,
                core.layout.clone(),
                core.layer,
            )?
        };

        let special_id = match parameters {
            ElementParams::Panel(_, panel_params) => {
                self.make_panel(root_id, panel_params.hidden, panel_params.floating)
            }
            ElementParams::Button(core, button_params) => {
                let text = &button_params.text;

                let (text_root_id, text_id) = {
                    const BUTTON_LABEL_LAYOUT: LayoutOptions = LayoutOptions::new();

                    let root_text_id = self.add_new(
                        Some(root_id),
                        None,
                        core.look,
                        BUTTON_LABEL_LAYOUT,
                        core.layer,
                    )?;
                    let special_text_id = self.make_text(
                        root_text_id,
                        text.contents,
                        text.font_name,
                        FontMetrics {
                            font_size: text.font_size,
                            line_height: text.line_height,
                        },
                        text.never_invalidate,
                    )?;
                    (root_text_id, special_text_id)
                };

                self.make_button(root_id, text_root_id, text_id, button_params.callback)
            }
            ElementParams::Text(_, text_params) => self.make_text(
                root_id,
                text_params.contents,
                text_params.font_name,
                FontMetrics {
                    font_size: text_params.font_size,
                    line_height: text_params.line_height,
                },
                text_params.never_invalidate,
            ),
            ElementParams::Image(_, image_params) => self.make_image(
                root_id,
                image_params.tint,
                image_params.opacity,
                image_params.texture,
            ),
            ElementParams::Slider(core, slider_params) => {
                let mut text_ids = None;
                if let Some(text) = &slider_params.text {
                    const SLIDER_LABEL_LAYOUT: LayoutOptions = LayoutOptions::new();

                    let root_text_id = self.add_new(
                        Some(root_id),
                        None,
                        core.look,
                        SLIDER_LABEL_LAYOUT,
                        core.layer,
                    )?;
                    let special_text_id = self.make_text(
                        root_text_id,
                        text.contents,
                        text.font_name,
                        FontMetrics {
                            font_size: text.font_size,
                            line_height: text.line_height,
                        },
                        text.never_invalidate,
                    )?;
                    text_ids = Some((root_text_id, special_text_id));
                };

                self.make_slider(
                    root_id,
                    text_ids,
                    slider_params.scroll_step,
                    slider_params.value_init,
                    slider_params.value_sync_handle,
                    slider_params.callback,
                    slider_params.vertical,
                )
            }
        }?;

        Ok((root_id, special_id))
    }
}

static FALLBACK_TEXTURE: RwLock<Option<TextureId>> = RwLock::new(None);

pub fn set_fallback_texture(fallback: TextureId) {
    *FALLBACK_TEXTURE.write().unwrap() = Some(fallback);
}

pub fn get_fallback_texture() -> Option<TextureId> {
    *FALLBACK_TEXTURE.read().unwrap()
}

pub fn expect_fallback_texture() -> TextureId {
    get_fallback_texture().expect("global fallback texture is not set")
}
