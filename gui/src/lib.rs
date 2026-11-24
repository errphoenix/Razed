pub mod declare;
pub mod draw;
pub mod env;
pub mod look;
pub mod render;
pub mod style;
pub mod text;
pub mod widget;

#[cfg(feature = "animation")]
pub mod animation;

use std::{fmt::Debug, sync::RwLock};

use cosmic_text::FontSystem;
use ethel::{
    assets::{AssetMetadataRegistry, TextureId, TextureMetadata},
    render::Resolution,
    state::{
        InputEvent,
        data::{Column, DirectIndex, IndirectIndex},
    },
};

use janus::{context::DeltaTime, input::KeyEvent};

use rendrs::batch::Batch;
pub use style::*;

use taffy::prelude::*;

use crate::{
    draw::{BatchingLayerCompositor, InterfaceAggregator, InterfaceObject, Quad},
    look::{LookId, LooksManager},
    text::{FontMetrics, GlyphAtlas, TextComposer, font::FontLibrary},
};

pub use crate::env::*;
pub use crate::widget::*;

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
    pub checkboxes: InterfaceCheckboxRowTable,
    pub dropdowns: InterfaceDropdownRowTable,

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
            checkboxes: InterfaceCheckboxRowTable::new(),
            dropdowns: InterfaceDropdownRowTable::new(),
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
            checkboxes: InterfaceCheckboxRowTableView::from(&self.checkboxes),
            dropdowns: InterfaceDropdownRowTableView::from(&self.dropdowns),
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

    /// Poll environment changes made by external systems directly on widgets.
    ///
    /// This affects sliders and checkboxes.
    ///
    /// Must run before interface input updates to ensure correct
    /// behaviour.
    pub fn poll_environment_changes(&mut self) {
        let env = &self.environment;

        // sliders
        {
            let sliders = &mut self.sliders;
            let caches = &mut sliders.value_cache[1..];
            let handles = &sliders.value_sync[1..];
            for (id, cache) in handles.iter().zip(caches).filter(|(id, _)| id.is_some()) {
                let id = id.as_ref().unwrap();
                let env_value = env.get(id).and_then(|v| v.as_float());
                *cache = env_value.unwrap_or_default();
            }
        }
        // checkboxes
        {
            let checkboxes = &mut self.checkboxes;
            let caches = &mut checkboxes.value[1..];
            let handles = &checkboxes.sync_handle[1..];
            for (id, cache) in handles.iter().zip(caches).filter(|(id, _)| id.is_some()) {
                let id = id.as_ref().unwrap();
                let env_value = env.get(id).and_then(|v| v.as_boolean());
                *cache = env_value.unwrap_or_default();
            }
        }
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
                ComponentKind::Slider(handle) => {
                    let direct = self.sliders.solve_indirect(handle).unwrap();
                    let callback = slider_callbacks[direct.as_index()];
                    let value = &mut self.sliders.value_cache[direct.as_index()];

                    match callback {
                        InteractableCallback::None => {}
                        InteractableCallback::Once(cb) => {
                            // only call once slider is released
                            // note: this fails if slider has a sync_handle
                            if !interact.pressed && interact.press_time.frames != 0 {
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
                ComponentKind::Checkbox(handle) => {
                    let direct = self.checkboxes.solve_indirect(handle).unwrap();
                    let value = &mut self.checkboxes.value[direct.as_index()];
                    if interact.press_time.frames == 1 {
                        value.toggle();
                    }
                    if let Some(sync) = self.checkboxes.sync_handle[direct.as_index()] {
                        self.environment.insert(sync, *value);
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
        let visible = &self.commons.visible;
        let bounds = &self.commons.feedback_bounds;
        let interacts = &mut self.commons.interaction_data;

        for i in (1..count).rev() {
            let bounds = bounds[i];
            let interact = &mut interacts[i];
            if visible[i] && bounds.contains(x, y) {
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
        if !self.commons.visible[table_index] {
            return;
        }

        let interact = &mut self.commons.interaction_data[table_index];
        if !interact.hovered {
            return;
        };

        if let ComponentKind::Slider(handle) = self.commons.archetype[table_index] {
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
        if let ComponentKind::Slider(slider_id) = self.commons.archetype[did.as_index()] {
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

    pub fn set_visible(&mut self, id: WidgetId, visible: bool) {
        if let Some(did) = self.commons.solve_indirect(id.0) {
            self.commons.visible[did.as_index()] = visible;
            self.set_visible_impl(did, visible);
        }
    }

    pub fn toggle_visible(&mut self, id: WidgetId) {
        if let Some(did) = self.commons.solve_indirect(id.0) {
            let visible = {
                let visible = &mut self.commons.visible[did.as_index()];
                visible.toggle();
                *visible
            };
            self.set_visible_impl(did, visible);
        }
    }

    fn set_visible_impl(&mut self, root_id: DirectIndex, visible: bool) {
        let taffy = self.commons.taffy_id[root_id.as_index()];
        let mut inner_style = self.layout.style(taffy.0).unwrap().clone();
        if visible {
            let visible_display_mode = self.commons.restore_display[root_id.as_index()];
            inner_style.display = visible_display_mode.into();
        } else {
            inner_style.display = taffy::Display::None;
        }
        let _ = self.layout.set_style(taffy.0, inner_style);
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

    /// Create a new empty or core element.
    ///
    /// This returns a new [`WidgetId`] which can be used to create
    /// specific element types through [`Self::make_panel`],
    /// [`Self::make_text`], [`Self::make_image`], or [`Self::make_button`].
    pub fn add_new(
        &mut self,
        parent: Option<WidgetId>,
        look_id: Option<impl Into<LookId>>,
        layout_options: LayoutOptions,
        layer: u32,
        visible: bool,
    ) -> Result<WidgetId, WidgetError> {
        let mut taffy_style = layout_options.into_taffy_style();

        // if invisible, set display to `none`
        // in any case, store real display mode for eventual
        // visibility toggles
        let visible_display_mode = taffy_style.display.into();
        if !visible {
            taffy_style.display = taffy::Display::None;
        }

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

        let id = WidgetId(self.commons.insert((
            ComponentKind::Null,
            look_id.map(Into::into).unwrap_or_default(),
            parent,
            TaffyNodeId(node_id),
            layout_options,
            layer.min(LAYERS as u32),
            visible,
            visible_display_mode,
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
        parameters: ElementOptions,
    ) -> Result<(WidgetId, IndirectIndex), WidgetError> {
        let root_id = {
            let core = parameters.common_options();
            self.add_new(
                core.parent,
                core.look,
                core.layout.clone(),
                core.layer,
                core.visible,
            )?
        };

        let special_id = match parameters {
            ElementOptions::Panel(_, panel_params) => {
                self.make_panel(root_id, panel_params.hidden, panel_params.floating)
            }
            ElementOptions::Button(core, button_params) => {
                let text = &button_params.text;

                let (text_root_id, text_id) = {
                    const BUTTON_LABEL_LAYOUT: LayoutOptions = LayoutOptions::new();

                    let root_text_id = self.add_new(
                        Some(root_id),
                        core.look,
                        BUTTON_LABEL_LAYOUT,
                        core.layer,
                        core.visible,
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
            ElementOptions::Text(_, text_params) => self.make_text(
                root_id,
                text_params.contents,
                text_params.font_name,
                FontMetrics {
                    font_size: text_params.font_size,
                    line_height: text_params.line_height,
                },
                text_params.never_invalidate,
            ),
            ElementOptions::Image(_, image_params) => self.make_image(
                root_id,
                image_params.tint,
                image_params.tint_opacity,
                image_params.texture,
            ),
            ElementOptions::Slider(_, slider_params) => self.make_slider(
                root_id,
                slider_params.scroll_step,
                slider_params.value_init,
                slider_params.value_sync_handle,
                slider_params.callback,
                slider_params.vertical,
            ),
            ElementOptions::Checkbox(_, checkbox_params) => self.make_checkbox(
                root_id,
                checkbox_params.starting_value,
                checkbox_params.sync_handle,
            ),
            ElementOptions::Dropdown(core, dropdown_params) => {
                let entries = dropdown_params.entries;
                let selected = dropdown_params.starting_selection.unwrap_or(0);
                let selected_entry = entries[selected as usize].name;
                let text_font_name = dropdown_params.font_name;
                let text_size = dropdown_params.font_size;
                let sync_handle = dropdown_params.sync_handle;

                let text_root_id = {
                    const DROPDOWN_LABELS_LAYOUT: LayoutOptions = LayoutOptions::new();

                    let root_text_id =
                        self.add_new(Some(root_id), core.look, DROPDOWN_LABELS_LAYOUT, core.layer)?;
                    let _ = self.make_text(
                        root_text_id,
                        TextContents::literal(selected_entry),
                        text_font_name,
                        FontMetrics {
                            font_size: text_size,
                            line_height: text_size + 2.0,
                        },
                        true,
                    )?;
                    root_text_id
                };

                let entries = entries.map(|e| e.name);
                self.make_dropdown(root_id, text_root_id, selected, entries, sync_handle)
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
