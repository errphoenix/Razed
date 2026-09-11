use ethel::{
    data::{Column, IndirectIndex},
    render::Resolution,
};
use gui::{
    ButtonParams, ContainerLayout, ContentAlignment, CoreElementParams, ElementParams,
    InteractableCallback, InteractionTime, InterfaceButtonRowTable, InterfaceSystem, ItemAlignment,
    LayoutOptions, LayoutPosition, PanelParams, Point, Rectangle, SliderParams, TextContents,
    TextNode, TextParams, Value, WidgetId, Wrap,
    env::{EnvValue, UiEnv},
    style::FlexDirection,
};
use janus::{StringHash, StringMap};

use env_names::*;

use crate::render::graphics::Gamma;

#[allow(unused_imports)]
use crate::{
    render::graphics,
    ui::{env_names::*, widget_names::*},
};

pub mod ctlpanel;
pub mod infopanel;

pub const COLORTINT_HOVER_INVARIANT: glam::Vec4 = glam::vec4(0f32, 0f32, 0f32, 1f32);

pub fn initialize_default(
    resolution: Resolution,
) -> (InterfaceSystem, StringMap<(WidgetId, IndirectIndex)>) {
    let mut system = InterfaceSystem::new(resolution);
    let mut map = StringMap::default();
    let root = system
        .create_element(ElementParams::Panel(
            CoreElementParams {
                parent: None,
                children: None,
                layer: 5,
                layout_options: LayoutOptions {
                    container: ContainerLayout::Flexbox {
                        direction: FlexDirection::Column,
                        wrap: Wrap::Wrap,
                        justify_content: ContentAlignment::Auto,
                        align_content: ContentAlignment::Auto,
                        align_items: ItemAlignment::Auto,
                    },
                    align_self: ItemAlignment::Stretch,
                    justify_self: ItemAlignment::Stretch,
                    layout_position: LayoutPosition::Relative,
                    size: Some(Point::new(
                        Value::Absolute(2560f32),
                        Value::Absolute(1440f32),
                    )),
                    ..Default::default()
                },
            },
            PanelParams {
                hover_tint: glam::Vec4::ZERO,
                opacity: 0f32,
                ..Default::default()
            },
        ))
        .unwrap()
        .0;

    infopanel::root(&mut system, root);
    ctlpanel::root(&mut system, root, &mut map);

    (system, map)
}

pub(crate) fn button_color_state(
    env_id_map: &[(StringHash, StringHash)],
    ui_map: &StringMap<(WidgetId, IndirectIndex)>,
    buttons: &mut InterfaceButtonRowTable,
    env: &UiEnv,
) {
    env_id_map.iter().for_each(|(var, id)| {
        if let Some(var) = env.get(var).and_then(|v| v.as_boolean()) {
            if let Some((_, id)) = ui_map.get(id).copied() {
                let did = buttons.solve_indirect(id).unwrap();
                buttons.base_color[did.as_index()] =
                    if var { glam::Vec3::Y } else { glam::Vec3::X };
            }
        }
    });
}

pub mod widget_names {
    use janus::StringHash;

    pub const DEBUG_CTL_VSYNC_BUTTON: StringHash =
        janus::hash_string("__debug.control.display.vsync:button");
}

pub mod env_names {
    use janus::StringHash;

    pub const SIM_CTL_STATE: StringHash = janus::hash_string("sim.control.state");
    pub const SIM_CTL_SPEED: StringHash = janus::hash_string("sim.control.speed");

    pub const DEBUG_PERF_LAST_SIMUL_FRAME_TIME_MILLIS: StringHash =
        janus::hash_string("__debug.perf.last_simul_frame_time.millis");
    pub const DEBUG_PERF_LAST_RENDER_FRAME_TIME_MILLIS: StringHash =
        janus::hash_string("__debug.perf.last_render_frame_time.millis");
    pub const DEBUG_PERF_FPS_AVG: StringHash = janus::hash_string("__debug.perf.fps.avg");
    pub const DEBUG_PERF_TPS_TOTAL: StringHash = janus::hash_string("__debug.perf.tps.total");

    pub const DEBUG_COUNTER_LATTICE_NODES: StringHash =
        janus::hash_string("__debug.counter.lattice.nodes");
    pub const DEBUG_COUNTER_LATTICE_CONSTRAINTS: StringHash =
        janus::hash_string("__debug.counter.lattice.constraints");
    pub const DEBUG_COUNTER_FRAGMENTS: StringHash = janus::hash_string("__debug.counter.fragments");
    pub const DEBUG_COUNTER_CAGES: StringHash = janus::hash_string("__debug.counter.cages");
    pub const DEBUG_COUNTER_DEBRIS: StringHash = janus::hash_string("__debug.counter.debris");

    pub const DEBUG_RENDER_GBANK_VUSE_PERC: StringHash =
        janus::hash_string("__debug.render.gbank.vuse_perc");
    pub const DEBUG_RENDER_GBANK_TUSE_PERC: StringHash =
        janus::hash_string("__debug.render.gbank.tuse_perc");
    pub const DEBUG_RENDER_GBANK_TRIS_COUNT: StringHash =
        janus::hash_string("__debug.render.gbank.tris_count");
    pub const DEBUG_RENDER_GBANK_MEMPRINT: StringHash =
        janus::hash_string("__debug.render.gbank.mem_footprint");

    pub const DEBUG_CTL_DISPLAY_VSYNC: StringHash =
        janus::hash_string("__debug.control.display.vsync");
    pub const DEBUG_CTL_GRAPHICS_GAMMA: StringHash =
        janus::hash_string("__debug.control.graphics.gamma");

    pub const DEBUG_CTL_SHADE_MODE: StringHash = janus::hash_string("__debug.control.shading.mode");
}
