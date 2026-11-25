use ethel::{Column, IndirectIndex, Resolution};
use janus::Cursor;

use crate::{
    Box2d, CommonWidgetOptions, ComponentKind, ElementOptions, HasOptions, InterfaceSystem,
    LayoutPosition, Value, WidgetError, WidgetId,
    declare::{UiNodeStack, UiPanel},
};

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum FloatGrabArea {
    SectionHoriz { height: f32 },
    SectionVert { width: f32 },
    Corner { width: f32, height: f32 },
}
impl FloatGrabArea {
    pub const DEFAULT_HEIGHT: f32 = 22f32;

    pub const fn top(height: f32) -> Self {
        Self::SectionHoriz { height }
    }

    pub const fn side(width: f32) -> Self {
        Self::SectionVert { width }
    }

    pub const fn corner_square(side: f32) -> Self {
        Self::corner(side, side)
    }

    pub const fn corner(width: f32, height: f32) -> Self {
        Self::Corner { width, height }
    }
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

impl<const LAYERS: usize> InterfaceSystem<LAYERS> {
    /// Process floating events and interactions.
    ///
    /// # Returns
    /// Returns the ID of the element that may require layout
    /// re-evaluation (with [`Self::evaluate_layout_node`]), if any.
    pub fn process_floating(&mut self, cursor: &Cursor) -> Option<WidgetId> {
        let panels_count = self.data.panels.len();
        let float_ids = &self.data.panels.floating;

        let (x, y, dx, dy) = {
            let c = cursor.current_f32();
            let d = cursor.delta_f32();
            (c.0, c.1, d.0, d.1)
        };

        for i in (1..panels_count).rev() {
            if let Some(float_id) = float_ids[i]
                && let Some(fdid) = self.data.floating.solve_indirect(float_id)
            {
                let fdid = fdid.as_index();
                let roots = &self.data.panels.root_rvid;
                let float_bounds = &self.data.floating.grab_area;
                let float_offset = &mut self.data.floating.offset;
                let float_state = &mut self.data.floating.state;

                let root = self.data.commons.solve_indirect(roots[i].0).unwrap();
                let layouts = &mut self.data.commons.layout_options;
                let interact = self.data.commons.interaction_data[root.as_index()];

                match float_state[fdid] {
                    FloatState::Idle => {
                        if interact.pressed && self.grabbing.is_none() {
                            let bounds = {
                                let p_bounds = self.data.commons.feedback_bounds[root.as_index()];
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
                        let p_anchor = self.data.commons.feedback_anchor[root.as_index()];

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
        float_params: Option<FloatPanelOptions>,
    ) -> Result<IndirectIndex, WidgetError> {
        if let Some(commons_id) = self.data.commons.solve_indirect(id.0) {
            self.assert_null_archetype(commons_id)?;

            let float_id = float_params.map(|params| {
                if let Some(base_pos) = params.base_pos {
                    let cdid = commons_id.as_index();
                    self.data.commons.feedback_anchor[cdid] = base_pos;
                    let layout = &mut self.data.commons.layout_options[cdid];
                    layout.layout_position = LayoutPosition::Absolute {
                        x: Some(Value::Absolute(base_pos.x)),
                        y: Some(Value::Absolute(base_pos.y)),
                    };
                }

                self.data
                    .floating
                    .insert((params.grab_area, glam::Vec2::ZERO, FloatState::Idle))
            });

            let panel_element = (id, hidden, float_id);
            let panel_id = self.data.panels.insert(panel_element);
            self.data.commons.archetype[commons_id.as_index()] = ComponentKind::Panel(panel_id);
            Ok(panel_id)
        } else {
            Err(WidgetError::InvalidWidgetHandle(id.0.as_int()))
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct FloatPanelOptions {
    pub base_pos: Option<glam::Vec2>,
    pub grab_area: FloatGrabArea,
}
impl FloatPanelOptions {
    pub const DEFAULT_GRAB_SIZE: f32 = 16.0;

    pub const fn new() -> Self {
        Self {
            base_pos: None,
            grab_area: FloatGrabArea::SectionHoriz {
                height: Self::DEFAULT_GRAB_SIZE,
            },
        }
    }

    pub const fn with_base_pos(self, base_pos: glam::Vec2) -> Self {
        Self {
            base_pos: Some(base_pos),
            ..self
        }
    }

    pub const fn with_grab_area(self, grab_area: FloatGrabArea) -> Self {
        Self { grab_area, ..self }
    }

    pub const fn with_grab_area_size(self, grab_area_size: f32) -> Self {
        Self {
            grab_area: match self.grab_area {
                FloatGrabArea::SectionHoriz { .. } => FloatGrabArea::SectionHoriz {
                    height: grab_area_size,
                },
                FloatGrabArea::SectionVert { .. } => FloatGrabArea::SectionVert {
                    width: grab_area_size,
                },
                FloatGrabArea::Corner { .. } => FloatGrabArea::Corner {
                    height: grab_area_size,
                    width: grab_area_size,
                },
            },
            ..self
        }
    }
}

#[derive(Clone, Debug, Default, Copy)]
pub struct PanelOptions {
    pub hidden: bool,
    /// if present, any positional layout (flex, grid) is overriden with
    /// the absolute `base_pos` defined in [`FloatParams`]
    pub floating: Option<FloatPanelOptions>,
}
impl PanelOptions {
    pub const fn new() -> Self {
        Self {
            hidden: false,
            floating: None,
        }
    }

    pub const fn hidden() -> Self {
        Self::new().with_hidden(true)
    }

    pub const fn floating_at(grab: FloatGrabArea, base_offset: Option<glam::Vec2>) -> Self {
        Self::new().with_floating_options(FloatPanelOptions {
            base_pos: base_offset,
            grab_area: grab,
        })
    }

    pub const fn floating(grab: FloatGrabArea) -> Self {
        Self::floating_at(grab, None)
    }

    pub const fn with_hidden(self, hidden: bool) -> Self {
        Self { hidden, ..self }
    }

    pub const fn with_floating(self, grab_area: FloatGrabArea) -> Self {
        self.with_floating_options(FloatPanelOptions {
            base_pos: None,
            grab_area,
        })
    }

    pub const fn with_floating_options(self, floating: FloatPanelOptions) -> Self {
        Self {
            floating: Some(floating),
            ..self
        }
    }
}

#[derive(Debug)]
pub struct Panel;
impl HasOptions for Panel {
    type Options = PanelOptions;

    fn complement_options(
        common: CommonWidgetOptions,
        this: Self::Options,
        _: Resolution,
    ) -> ElementOptions {
        ElementOptions::Panel(common, this)
    }

    fn archetype() -> ComponentKind {
        ComponentKind::Panel(Default::default())
    }
}

impl UiNodeStack<'_> {
    pub fn panel(&mut self, panel_op: impl FnOnce(UiPanel) -> UiPanel) {
        self.push(panel_op(UiPanel::new(self.reference_resolution)));
    }

    pub fn panel_from(&mut self, panel_options: PanelOptions) {
        self.push(UiPanel::from(panel_options, self.reference_resolution));
    }
}
