pub use super::*;

pub fn root(system: &mut InterfaceSystem, root: WidgetId) -> WidgetId {
    let root = system
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
                    parent: Some(root),
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
        TextNode::Variable(DEBUG_PERF_FPS_AVG),
    ]));
    debug_text(TextContents::from_nodes(&[
        TextNode::Static("TPS = "),
        TextNode::Variable(DEBUG_PERF_TPS_TOTAL),
    ]));
    debug_text(TextContents::from_nodes(&[
        TextNode::Static("Frame::Duration.Simulation = "),
        TextNode::Variable(DEBUG_PERF_LAST_SIMUL_FRAME_TIME_MILLIS),
        TextNode::Static("ms"),
    ]));
    debug_text(TextContents::from_nodes(&[
        TextNode::Static("Frame::Duration.Render = "),
        TextNode::Variable(DEBUG_PERF_LAST_RENDER_FRAME_TIME_MILLIS),
        TextNode::Static("ms"),
    ]));
    debug_text(TextContents::from_nodes(&[
        TextNode::Static("World::Lattice.Nodes = "),
        TextNode::Variable(DEBUG_COUNTER_LATTICE_NODES),
    ]));
    debug_text(TextContents::from_nodes(&[
        TextNode::Static("World::Lattice.Constraints = "),
        TextNode::Variable(DEBUG_COUNTER_LATTICE_CONSTRAINTS),
    ]));
    debug_text(TextContents::from_nodes(&[
        TextNode::Static("World::Fragments = "),
        TextNode::Variable(DEBUG_COUNTER_FRAGMENTS),
    ]));
    debug_text(TextContents::from_nodes(&[
        TextNode::Static("World::Cages = "),
        TextNode::Variable(DEBUG_COUNTER_CAGES),
    ]));
    debug_text(TextContents::from_nodes(&[
        TextNode::Static("World::Debris "),
        TextNode::Variable(DEBUG_COUNTER_DEBRIS),
    ]));
    debug_text(TextContents::from_nodes(&[
        TextNode::Static("Sim::State = "),
        TextNode::Variable(SIM_CTL_STATE),
    ]));
    debug_text(TextContents::from_nodes(&[
        TextNode::Static("Sim::Speed = "),
        TextNode::Variable(SIM_CTL_SPEED),
    ]));

    debug_text(TextContents::from_nodes(&[
        TextNode::Static("GeomBank::VertexUse = "),
        TextNode::Variable(DEBUG_RENDER_GBANK_VUSE_PERC),
    ]));
    debug_text(TextContents::from_nodes(&[
        TextNode::Static("GeomBank::TrisUse = "),
        TextNode::Variable(DEBUG_RENDER_GBANK_TUSE_PERC),
    ]));
    debug_text(TextContents::from_nodes(&[
        TextNode::Static("GeomBank::TrisCount/w-cull = "),
        TextNode::Variable(DEBUG_RENDER_GBANK_TRIS_COUNT),
    ]));
    debug_text(TextContents::from_nodes(&[
        TextNode::Static("GeomBank::MemoryUse = "),
        TextNode::VariableAnd {
            env_id: DEBUG_RENDER_GBANK_MEMPRINT,
            operation: |ev| {
                let b = ev.as_integer().unwrap_or_default();
                EnvValue::Float(b as f32 / 1024f32) // convert to kb
            },
        },
        TextNode::Static(" KB"),
    ]));
    root
}
