use ethel::{
    data::{Column, IndirectIndex},
    render::Resolution,
};
use gui::{
    ButtonCallback, ButtonParams, ContainerLayout, ContentAlignment, CoreElementParams,
    ElementParams, InterfaceButtonRowTable, InterfaceSystem, ItemAlignment, LayoutOptions,
    LayoutPosition, PanelParams, Point, Rectangle, TextContents, TextNode, TextParams, Value,
    WidgetId, Wrap, env::UiEnv, style::FlexDirection,
};
use janus::{StringHash, StringMap};

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

    debug_infopanel(&mut system, root);
    debug_ctlpanel(&mut system, root, &mut map);

    (system, map)
}

pub const DEBUG_CTL_VSYNC_BUTTON: StringHash = janus::hash_string("__debug.ctl.vsync:button");

fn debug_ctlpanel(
    system: &mut InterfaceSystem,
    root: WidgetId,
    map: &mut StringMap<(WidgetId, IndirectIndex)>,
) {
    let debug_panel = system
        .create_element(ElementParams::Panel(
            CoreElementParams {
                parent: Some(root),
                children: None,
                layout_options: LayoutOptions {
                    container: ContainerLayout::Flexbox {
                        direction: FlexDirection::Column,
                        wrap: Wrap::Wrap,
                        justify_content: ContentAlignment::Stretch,
                        align_content: ContentAlignment::Stretch,
                        align_items: ItemAlignment::Stretch,
                    },
                    justify_self: ItemAlignment::Center,
                    align_self: ItemAlignment::End,
                    layout_position: LayoutPosition::Relative,
                    size: Some(Point {
                        x: Value::Absolute(400f32),
                        y: Value::Absolute(340f32),
                    }),
                    margin: Some(Rectangle::splat(Value::Absolute(8f32))),
                    ..Default::default()
                },
                layer: 5,
            },
            Default::default(),
        ))
        .unwrap()
        .0;

    let params_dbg_button = |text: &'static str, cb: ButtonCallback| ButtonParams {
        text: TextParams {
            contents: TextContents::from_node(TextNode::Static(text)),
            never_invalidate: true,
            ..Default::default()
        },
        bg_color: glam::Vec3::ZERO,
        bg_hover_tint: COLORTINT_HOVER_INVARIANT,
        bg_press_tint: COLORTINT_HOVER_INVARIANT,
        callback: cb,
    };

    let dbg_button_vsync = system
        .create_element(ElementParams::Button(
            CoreElementParams {
                parent: Some(debug_panel),
                children: None,
                layout_options: LayoutOptions {
                    align_self: ItemAlignment::Start,
                    justify_self: ItemAlignment::Start,
                    ..Default::default()
                },
                layer: 5,
            },
            params_dbg_button(
                "V-SYNC",
                ButtonCallback::Once(|env| {
                    if let Some(vsync) = env.get_mut(&env_names::DEBUG_CTL_VSYNC) {
                        let v = vsync.as_boolean_mut().unwrap();
                        *v = !*v;
                    }
                }),
            ),
        ))
        .unwrap();

    map.insert(DEBUG_CTL_VSYNC_BUTTON, dbg_button_vsync);
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

fn debug_infopanel(system: &mut InterfaceSystem, root: WidgetId) {
    let debug_panel = system
        .create_element(ElementParams::Panel(
            CoreElementParams {
                parent: Some(root),
                children: None,
                layout_options: LayoutOptions {
                    container: ContainerLayout::Flexbox {
                        direction: FlexDirection::Column,
                        wrap: Wrap::Wrap,
                        justify_content: ContentAlignment::Stretch,
                        align_content: ContentAlignment::Stretch,
                        align_items: ItemAlignment::Stretch,
                    },
                    justify_self: ItemAlignment::Start,
                    align_self: ItemAlignment::Start,
                    layout_position: LayoutPosition::Absolute {
                        x: Some(Value::Absolute(8f32)),
                        y: Some(Value::Absolute(8f32)),
                    },
                    size: Some(Point {
                        x: Value::Absolute(840f32),
                        y: Value::Absolute(420f32),
                    }),
                    padding: Some(Rectangle::splat(Value::Absolute(10f32))),
                    ..Default::default()
                },
                layer: 5,
            },
            Default::default(),
        ))
        .unwrap()
        .0;

    let mut debug_text = |contents: TextContents| {
        system
            .create_element(ElementParams::Text(
                CoreElementParams {
                    parent: Some(debug_panel),
                    children: None,
                    layout_options: LayoutOptions {
                        align_self: ItemAlignment::Stretch,
                        ..Default::default()
                    },
                    layer: 5,
                },
                TextParams {
                    contents,
                    font_size: 16f32,
                    line_height: 18f32,
                    ..Default::default()
                },
            ))
            .unwrap();
    };

    debug_text(TextContents::from_nodes(&[
        TextNode::Static("FPS = "),
        TextNode::Variable(env_names::DEBUG_PERF_FPS_AVG),
    ]));
    debug_text(TextContents::from_nodes(&[
        TextNode::Static("TPS = "),
        TextNode::Variable(env_names::DEBUG_PERF_TPS_TOTAL),
    ]));
    debug_text(TextContents::from_nodes(&[
        TextNode::Static("Last SIMUL. frame duration = "),
        TextNode::Variable(env_names::DEBUG_PERF_LAST_SIMUL_FRAME_TIME_MILLIS),
        TextNode::Static("ms"),
    ]));
    debug_text(TextContents::from_nodes(&[
        TextNode::Static("Last RENDER frame duration = "),
        TextNode::Variable(env_names::DEBUG_PERF_LAST_RENDER_FRAME_TIME_MILLIS),
        TextNode::Static("ms"),
    ]));
    debug_text(TextContents::from_nodes(&[
        TextNode::Static("Lattice nodes = "),
        TextNode::Variable(env_names::DEBUG_COUNTER_LATTICE_NODES),
    ]));
    debug_text(TextContents::from_nodes(&[
        TextNode::Static("Lattice constraints = "),
        TextNode::Variable(env_names::DEBUG_COUNTER_LATTICE_CONSTRAINTS),
    ]));
    debug_text(TextContents::from_nodes(&[
        TextNode::Static("Fragments = "),
        TextNode::Variable(env_names::DEBUG_COUNTER_FRAGMENTS),
    ]));
    debug_text(TextContents::from_nodes(&[
        TextNode::Static("Deform. Cages = "),
        TextNode::Variable(env_names::DEBUG_COUNTER_CAGES),
    ]));
    debug_text(TextContents::from_nodes(&[
        TextNode::Static("Debris = "),
        TextNode::Variable(env_names::DEBUG_COUNTER_DEBRIS),
    ]));
    debug_text(TextContents::from_nodes(&[
        TextNode::Static("Sim::state = "),
        TextNode::Variable(env_names::SIM_CTL_STATE),
    ]));
    debug_text(TextContents::from_nodes(&[
        TextNode::Static("Sim::speed = "),
        TextNode::Variable(env_names::SIM_CTL_SPEED),
    ]));
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

    pub const DEBUG_CTL_VSYNC: StringHash = janus::hash_string("__debug.control.vsync");
}
