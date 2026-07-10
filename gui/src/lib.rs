use std::sync::RwLock;

use cosmic_text::FontSystem;
use ethel::{
    assets::{AssetMetadataRegistry, CachedStringHash, TextureId, TextureMetadata},
    render::Resolution,
    state::data::{Column, DirectIndex, IndirectIndex},
};

use janus::{context::DeltaTime, input::KeyEvent};

pub mod draw;
pub mod env;
pub mod shaders;
pub mod style;
pub mod text;

pub use style::*;

use taffy::prelude::*;

use crate::{
    draw::{Batch, BatchingLayerCompositor, InterfaceAggregator, InterfaceObject},
    text::{FontMetrics, GlyphAtlas, TextComposer, TextMeasurement, font::FontLibrary},
};

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

ethel::table_spec! {
    struct InterfaceCommon {
        archetype: ComponentKind;
        parent: WidgetId;
        children: Vec<WidgetId>;
        taffy_id: TaffyNodeId;
        layout_options: LayoutOptions;

        layer: u32;

        // feedback values from taffy tree after evaluation
        feedback_anchor: glam::Vec2; // top left corner
        feedback_bounds: Box2d;

        hovered: bool;
        pressed: bool;
        hover_time: InteractionTime;
        press_time: InteractionTime;
    }
}

ethel::table_spec! {
    struct InterfacePanel {
        background_color: glam::Vec3;
        hover_tint: glam::Vec4;
        opacity: f32;
    }
}

ethel::table_spec! {
    struct InterfaceText {
        string: CachedStringHash;
        font_name: CachedStringHash;
        color: glam::Vec4;
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

        base_color: glam::Vec3;
        hover_tint: glam::Vec4;
        press_tint: glam::Vec4;

        callback: ButtonCallback;
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub enum ButtonCallback {
    #[default]
    None,
    Once(fn()),
    Repeating(fn()),
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
}
impl std::fmt::Display for ComponentKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComponentKind::Null => write!(f, "null"),
            ComponentKind::Panel(_) => write!(f, "panel"),
            ComponentKind::Text(_) => write!(f, "text"),
            ComponentKind::Image(_) => write!(f, "image"),
            ComponentKind::Button { .. } => write!(f, "button"),
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

    commons: InterfaceCommonRowTable,
    panels: InterfacePanelRowTable,
    texts: InterfaceTextRowTable,
    images: InterfaceImageRowTable,
    buttons: InterfaceButtonRowTable,

    intermediate_buffer: Vec<InterfaceObject>,
    compositor: BatchingLayerCompositor<LAYERS>,

    text_composer: TextComposer,
    font_library: FontLibrary,
}
/// Safety:
/// TaffyTree is !Send due to internal implementation details related to raw
/// const* function pointers. This implementation is required for ethel's
/// threads initialization.
unsafe impl<const LAYERS: usize> Send for InterfaceSystem<LAYERS> {}
/// Safety:
/// TaffyTree is !Sync due to internal implementation details related to raw
/// const* function pointers. This implementation is required for ethel's
/// threads initialization.
unsafe impl<const LAYERS: usize> Sync for InterfaceSystem<LAYERS> {}
impl<const LAYERS: usize> InterfaceSystem<LAYERS> {
    pub fn new(resolution: Resolution) -> Self {
        let mut layout = TaffyTree::with_capacity(1);
        const ROOT_ID: WidgetId = WidgetId(IndirectIndex::null(0));

        let tree_id = layout
            .new_leaf_with_context(Style::default(), ROOT_ID)
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
            texts: InterfaceTextRowTable::new(),
            images: InterfaceImageRowTable::new(),
            buttons: InterfaceButtonRowTable::new(),
            intermediate_buffer: Vec::new(),
            compositor: BatchingLayerCompositor::new(),
            text_composer: TextComposer::new(),
            font_library: FontLibrary::new(),
        }
    }

    pub fn with_capacity(resolution: Resolution, capacity: usize) -> Self {
        let mut layout = TaffyTree::with_capacity(capacity + 1);
        const ROOT_ID: WidgetId = WidgetId(IndirectIndex::null(0));

        let tree_id = layout
            .new_leaf_with_context(Style::default(), ROOT_ID)
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
            panels: InterfacePanelRowTable::with_capacity(capacity),
            texts: InterfaceTextRowTable::with_capacity(capacity),
            images: InterfaceImageRowTable::with_capacity(capacity),
            buttons: InterfaceButtonRowTable::with_capacity(capacity),
            intermediate_buffer: Vec::with_capacity(capacity),
            compositor: BatchingLayerCompositor::new(),
            text_composer: TextComposer::new(),
            font_library: FontLibrary::new(),
        }
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
        let aggregator = InterfaceAggregator {
            commons: InterfaceCommonRowTableView::from(&self.commons),
            panels: InterfacePanelRowTableView::from(&self.panels),
            texts: InterfaceTextRowTableView::from(&self.texts),
            images: InterfaceImageRowTableView::from(&self.images),
            buttons: InterfaceButtonRowTableView::from(&self.buttons),
        };
        aggregator.gather_quad_elements(
            &mut self.text_composer,
            glyph_atlas,
            &mut self.intermediate_buffer,
        );
    }

    pub fn composite_layers(&mut self, registry: &AssetMetadataRegistry<TextureMetadata>) {
        self.intermediate_buffer
            .drain(..)
            .for_each(|object| self.compositor.insert(object, registry));
    }

    pub fn finalize_batches(&mut self) {
        self.compositor.pull_batches();
    }

    pub fn batches(&self) -> &[Batch] {
        self.compositor.batches()
    }

    pub fn clear_batches(&mut self) {
        self.compositor.clear_batches();
    }

    pub fn compositor(&self) -> &BatchingLayerCompositor<LAYERS> {
        &self.compositor
    }

    pub fn compositor_mut(&mut self) -> &mut BatchingLayerCompositor<LAYERS> {
        &mut self.compositor
    }

    pub fn process_widget_states(&mut self, delta: DeltaTime) {
        let delta = delta.as_f32();
        let count = self.commons.len();

        let archetypes = &self.commons.archetype;
        let hovered = &self.commons.hovered;
        let hover_time = &mut self.commons.hover_time;
        let pressed = &mut self.commons.pressed;
        let press_time = &mut self.commons.press_time;

        let button_callbacks = &self.buttons.callback;

        for i in 1..count {
            let archetype = archetypes[i];

            match archetype {
                ComponentKind::Button { handle, .. } => {
                    let pressed = pressed[i];
                    if pressed {
                        let direct = self.buttons.solve_indirect(handle).unwrap();
                        let callback = button_callbacks[direct.as_index()];
                        match callback {
                            ButtonCallback::None => {}
                            ButtonCallback::Once(cb) => {
                                if press_time[i].frames == 1 {
                                    cb()
                                }
                            }
                            ButtonCallback::Repeating(cb) => cb(),
                        }
                    }
                }

                _ => {}
            }

            // advance hover/press durations
            let ht = &mut hover_time[i];
            let hovered = hovered[i];
            ht.frames = (ht.frames + 1) * hovered as u32;
            ht.seconds = (ht.seconds + delta) * hovered as u32 as f32;

            pressed[i] &= hovered;
            let pt = &mut press_time[i];
            let pf = pressed[i] as u32;
            pt.frames = (pt.frames + 1) * pf;
            pt.seconds = (pt.seconds + delta) * pf as f32;
        }
    }

    pub fn process_hover_events(&mut self, x: f32, y: f32, delta: DeltaTime) {
        let delta = delta.as_f32();

        let count = self.commons.len();
        let bounds = &self.commons.feedback_bounds;
        let hovered = &mut self.commons.hovered;
        let hover_time = &mut self.commons.hover_time;

        for i in 1..count {
            let bounds = bounds[i];
            if bounds.contains(x, y) {
                hovered[i] = true;
                let ht = &mut hover_time[i];
                ht.seconds += delta;
                ht.frames += 1;
            } else {
                hovered[i] = false;
                let ht = &mut hover_time[i];
                ht.seconds = 0.0;
                ht.frames = 0;
            }
        }
    }

    fn process_key_event(&mut self, table_index: usize, event: KeyEvent, delta: DeltaTime) {
        let delta = delta.as_f32();
        let hovered = &self.commons.hovered;
        let pressed = &mut self.commons.pressed;
        let press_time = &mut self.commons.press_time;

        // mouse press, hold, release
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
            let press = hovered[table_index] & click;
            pressed[table_index] = press;
            let pt = &mut press_time[table_index];
            pt.seconds = (pt.seconds + delta) * press as u32 as f32;
            pt.frames = (pt.frames + 1) * press as u32;
        }

        // keyboard (todo)
        {}
    }

    pub fn feed_key_events(&mut self, events: &[KeyEvent], delta: DeltaTime) {
        let count = self.commons.len();
        for i in 1..count {
            for event in events {
                self.process_key_event(i, *event, delta);
            }
        }
    }

    pub fn is_root(&self, id: WidgetId) -> bool {
        self.root_id().table_id == id
    }

    pub fn synchronise_layout(&mut self) {
        let count = self.commons.len();
        let taffy_ids = &self.commons.taffy_id;
        let parents = &self.commons.parent;

        for i in 1..count {
            let taffy_id = taffy_ids[i];
            if taffy_id.is_null() {
                continue;
            }

            let parent = parents[i];
            let (offset_x, offset_y) = if !self.is_root(parent) {
                let direct = unsafe { self.commons.solve_indirect_unchecked(parent.0) };
                let position = self.commons.feedback_anchor[direct.as_index()];
                (position.x, position.y)
            } else {
                (0.0, 0.0)
            };

            let fb_anchor = &mut self.commons.feedback_anchor[i];
            let fb_bounds = &mut self.commons.feedback_bounds[i];
            let taffy_id = taffy_id.0;

            let node = self.layout.get_final_layout(taffy_id);
            let position = node.location;
            let size = node.size;
            let position_x = position.x + offset_x;
            let position_y = position.y + offset_y;
            let min = glam::vec2(position_x, position_y);
            let max = glam::vec2(position_x + size.width, position_y + size.height);

            *fb_anchor = glam::vec2(position_x, position_y);
            *fb_bounds = Box2d { min, max };
        }
    }

    pub fn evaluate_layout(&mut self) {
        let available = Size {
            width: AvailableSpace::Definite(self.resolution.width),
            height: AvailableSpace::Definite(self.resolution.height),
        };

        self.layout
            .compute_layout_with_measure(
                self.root_node.tree_id,
                available,
                |known_size, available, _id, ctx, _style| {
                    let id = ctx.expect("node has no associated widget with it");
                    let did = unsafe { self.commons.solve_indirect_unchecked(id.0) };
                    let archetype = self.commons.archetype[did.as_index()];
                    if let ComponentKind::Text(text_id) = archetype {
                        let width = known_size.width.or(available.width.into_option());
                        let height = known_size.height.or(available.height.into_option());
                        let tdid = self
                            .texts
                            .solve_indirect(text_id)
                            .expect("text id must be valid")
                            .as_index();

                        let text = self.texts.string[tdid];
                        let metrics = self.texts.metrics[tdid];
                        let font = self.texts.font_name[tdid];

                        let text_string = ethel::assets::strings::fetch(text);
                        let font_string = ethel::assets::strings::fetch(font);

                        self.text_composer.set_buffer_size(width, height);
                        self.text_composer.set_font_metrics(metrics);
                        self.text_composer.set_text(text_string);
                        self.text_composer.set_font(font_string);
                        let measurement = self.text_composer.measure();

                        self.texts.measure[tdid] = measurement;

                        return Size {
                            width: measurement.width,
                            height: measurement.height,
                        };
                    }
                    Size::zero()
                },
            )
            .expect("failed to evaluate taffy layout");
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
        color: glam::Vec3,
        hover_tint: glam::Vec4,
        opacity: f32,
    ) -> Result<IndirectIndex, WidgetError> {
        if let Some(commons_id) = self.commons.solve_indirect(id.0) {
            self.assert_null_archetype(commons_id)?;
            let panel_element = (color, hover_tint, opacity);
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
        string: CachedStringHash,
        font_name: CachedStringHash,
        color: glam::Vec4,
        metrics: FontMetrics,
    ) -> Result<IndirectIndex, WidgetError> {
        if let Some(commons_id) = self.commons.solve_indirect(id.0) {
            self.assert_null_archetype(commons_id)?;
            let text_element = (
                string,
                font_name,
                color,
                metrics,
                TextMeasurement::default(),
            );
            let text_id = self.texts.insert(text_element);
            self.commons.archetype[commons_id.as_index()] = ComponentKind::Text(text_id);
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
        base_color: glam::Vec3,
        hover_tint: glam::Vec4,
        press_tint: glam::Vec4,
        callback: ButtonCallback,
    ) -> Result<IndirectIndex, WidgetError> {
        if let Some(commons_id) = self.commons.solve_indirect(root_id.0) {
            self.assert_null_archetype(commons_id)?;
            let button_element = (text_id, base_color, hover_tint, press_tint, callback);
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

    /// Create a new empty or core element.
    ///
    /// This returns a new [`WidgetId`] which can be used to create
    /// specific element types through [`Self::make_panel`],
    /// [`Self::make_text`], [`Self::make_image`], or [`Self::make_button`].
    pub fn add_new(
        &mut self,
        parent: Option<WidgetId>,
        children: Option<&[WidgetId]>,
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
            parent,
            children,
            TaffyNodeId(node_id),
            layout_options,
            layer.min(LAYERS as u32),
            // init default feedback values
            glam::Vec2::ZERO,
            Box2d::NULL,
            false,
            false,
            InteractionTime::default(),
            InteractionTime::default(),
        )));

        self.layout
            .set_node_context(node_id, Some(id))
            .map_err(WidgetError::TaffyLayoutError)?;

        Ok(id)
    }

    pub fn core_data(&self) -> &InterfaceCommonRowTable {
        &self.commons
    }

    pub fn core_data_mut(&mut self) -> &mut InterfaceCommonRowTable {
        &mut self.commons
    }

    pub fn panel_data(&self) -> &InterfacePanelRowTable {
        &self.panels
    }

    pub fn panel_data_mut(&mut self) -> &mut InterfacePanelRowTable {
        &mut self.panels
    }

    pub fn image_data(&self) -> &InterfaceImageRowTable {
        &self.images
    }

    pub fn image_data_mut(&mut self) -> &mut InterfaceImageRowTable {
        &mut self.images
    }

    pub fn text_data(&self) -> &InterfaceTextRowTable {
        &self.texts
    }

    pub fn text_data_mut(&mut self) -> &mut InterfaceTextRowTable {
        &mut self.texts
    }

    pub fn button_data(&self) -> &InterfaceButtonRowTable {
        &self.buttons
    }

    pub fn button_data_mut(&mut self) -> &mut InterfaceButtonRowTable {
        &mut self.buttons
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
                core.layout_options.clone(),
                core.layer,
            )?
        };

        let special_id = match parameters {
            ElementParams::Panel(_, panel_params) => self.make_panel(
                root_id,
                panel_params.color,
                panel_params.hover_tint,
                panel_params.opacity,
            ),
            ElementParams::Button(core, button_params) => {
                let text = &button_params.text;

                let (text_root_id, text_id) = {
                    const BUTTON_LABEL_LAYOUT: LayoutOptions = LayoutOptions::new();

                    let root_text_id =
                        self.add_new(Some(root_id), None, BUTTON_LABEL_LAYOUT, core.layer)?;
                    let special_text_id = self.make_text(
                        root_text_id,
                        text.string,
                        text.font_name,
                        text.color,
                        FontMetrics {
                            font_size: text.font_size,
                            line_height: text.line_height,
                        },
                    )?;
                    (root_text_id, special_text_id)
                };

                self.make_button(
                    root_id,
                    text_root_id,
                    text_id,
                    button_params.bg_color,
                    button_params.bg_hover_tint,
                    button_params.bg_press_tint,
                    button_params.callback,
                )
            }
            ElementParams::Text(_, text_params) => self.make_text(
                root_id,
                text_params.string,
                text_params.font_name,
                text_params.color,
                FontMetrics {
                    font_size: text_params.font_size,
                    line_height: text_params.line_height,
                },
            ),
            ElementParams::Image(_, image_params) => self.make_image(
                root_id,
                image_params.tint,
                image_params.opacity,
                image_params.texture,
            ),
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

#[derive(Clone, Debug, Default)]
pub struct CoreElementParams<'children> {
    pub parent: Option<WidgetId>,
    pub children: Option<&'children [WidgetId]>,
    pub layout_options: LayoutOptions,
    pub layer: u32,
}

pub const DEFAULT_GENERIC_COLOR: glam::Vec3 = glam::Vec3::ZERO;
pub const DEFAULT_GENERIC_HOVER_TINT: glam::Vec4 = glam::vec4(0.3, 0.3, 0.3, 0.2);
pub const DEFAULT_GENERIC_PRESS_TINT: glam::Vec4 = glam::vec4(0.45, 0.45, 0.45, 0.37);
pub const DEFAULT_GENERIC_OPACITY: f32 = 0.4;

#[derive(Clone, Debug)]
pub struct PanelParams {
    pub color: glam::Vec3,
    pub hover_tint: glam::Vec4,
    pub opacity: f32,
}
impl Default for PanelParams {
    fn default() -> Self {
        Self {
            color: DEFAULT_GENERIC_COLOR,
            hover_tint: DEFAULT_GENERIC_HOVER_TINT,
            opacity: DEFAULT_GENERIC_OPACITY,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ButtonParams {
    pub text: TextParams,
    pub bg_color: glam::Vec3,
    pub bg_hover_tint: glam::Vec4,
    pub bg_press_tint: glam::Vec4,
    pub callback: ButtonCallback,
}
impl Default for ButtonParams {
    fn default() -> Self {
        Self {
            text: TextParams::default(),
            bg_color: DEFAULT_GENERIC_COLOR,
            bg_hover_tint: DEFAULT_GENERIC_HOVER_TINT,
            bg_press_tint: DEFAULT_GENERIC_PRESS_TINT,
            callback: ButtonCallback::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TextParams {
    pub string: CachedStringHash,
    pub font_name: CachedStringHash,
    pub color: glam::Vec4,
    pub font_size: f32,
    pub line_height: f32,
}
impl TextParams {
    ethel::lazy_hash_str! {
        pub DEFAULT_TEXT = "Lorem Ipsum Blah Blah";
        pub DEFAULT_FONT = "Arial";
    }

    pub const DEFAULT_COLOR: glam::Vec4 = glam::Vec4::ONE;
    pub const DEFAULT_FONT_SIZE: f32 = 11.0;
    pub const DEFAULT_LINE_HEIGHT: f32 = 11.0;
}
impl Default for TextParams {
    fn default() -> Self {
        Self {
            string: *Self::DEFAULT_TEXT,
            font_name: *Self::DEFAULT_FONT,
            color: Self::DEFAULT_COLOR,
            font_size: Self::DEFAULT_FONT_SIZE,
            line_height: Self::DEFAULT_LINE_HEIGHT,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ImageParams {
    pub tint: glam::Vec4,
    pub opacity: f32,
    pub texture: TextureId,
}
impl ImageParams {
    pub const DEFAULT_TINT: glam::Vec4 = glam::Vec4::ZERO;
    pub const DEFAULT_OPACITY: f32 = 1.0;
}
impl Default for ImageParams {
    fn default() -> Self {
        Self {
            tint: Self::DEFAULT_TINT,
            opacity: Self::DEFAULT_OPACITY,
            texture: FALLBACK_TEXTURE
                .read()
                .unwrap()
                .expect("fallback texture must be st"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ElementParams<'children> {
    Panel(CoreElementParams<'children>, PanelParams),
    Button(CoreElementParams<'children>, ButtonParams),
    Text(CoreElementParams<'children>, TextParams),
    Image(CoreElementParams<'children>, ImageParams),
}
impl ElementParams<'_> {
    pub fn core(&'_ self) -> &CoreElementParams<'_> {
        match self {
            ElementParams::Panel(core_element_params, _) => core_element_params,
            ElementParams::Button(core_element_params, _) => core_element_params,
            ElementParams::Text(core_element_params, _) => core_element_params,
            ElementParams::Image(core_element_params, _) => core_element_params,
        }
    }
}
