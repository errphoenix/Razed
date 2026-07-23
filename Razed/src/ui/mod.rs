use ethel::render::Resolution;
use gui::{
    ButtonParams, ContainerLayout, ContentAlignment, CoreElementParams, ElementParams,
    InterfaceSystem, ItemAlignment, LayoutOptions, LayoutPosition, PanelParams, Point, Rectangle,
    TextContents, TextNode, TextParams, Value, Wrap, style::FlexDirection,
};

pub fn initialize_default(resolution: Resolution) -> InterfaceSystem {
    let mut system = InterfaceSystem::new(resolution);

    let debug_panel = system
        .create_element(ElementParams::Panel(
            CoreElementParams {
                parent: None,
                children: None,
                layout_options: LayoutOptions {
                    container: ContainerLayout::Flexbox {
                        direction: FlexDirection::Column,
                        wrap: Wrap::Wrap,
                        justify_content: ContentAlignment::Stretch,
                        align_content: ContentAlignment::Stretch,
                        align_items: ItemAlignment::Stretch,
                    },
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
        TextNode::Static("Cage points = "),
        TextNode::Variable(env_names::DEBUG_COUNTER_CAGE_POINTS),
    ]));
    debug_text(TextContents::from_nodes(&[
        TextNode::Static("Debris = "),
        TextNode::Variable(env_names::DEBUG_COUNTER_DEBRIS),
    ]));

    system
}

pub mod env_names {
    use janus::StringHash;

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
    pub const DEBUG_COUNTER_CAGE_POINTS: StringHash =
        janus::hash_string("__debug.counter.cage_points");
    pub const DEBUG_COUNTER_DEBRIS: StringHash = janus::hash_string("__debug.counter.debris");
}
