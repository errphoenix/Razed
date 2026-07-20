use ethel::render::Resolution;
use gui::{
    ContainerLayout, ContentAlignment, CoreElementParams, ElementParams, InterfaceSystem,
    ItemAlignment, LayoutOptions, LayoutPosition, Point, Rectangle, TextContents, TextNode,
    TextParams, Value, Wrap, style::FlexDirection,
};
use janus::texture::{Tex, TextureView};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Hash)]
pub struct UiRenderCommandBasic {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub instance_offset: u32,
    pub texture_units: [Option<TextureView>; rendrs::BATCH_UNITS],
}
impl UiRenderCommandBasic {
    pub fn bind_texture_units(&self) {
        janus::assert_gl!();

        self.texture_units
            .iter()
            .enumerate()
            .filter_map(|(i, tex)| tex.and_then(|tex| Some((i, tex))))
            .for_each(|(index, texture)| {
                texture.bind(index as u32);
            });
    }
}

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
                        justify_content: ContentAlignment::SpaceEvenly,
                        align_content: ContentAlignment::Start,
                        align_items: ItemAlignment::Start,
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
                        align_self: ItemAlignment::Start,
                        min_size: Some(Point {
                            x: Value::Absolute(256f32),
                            y: Value::Absolute(0f32),
                        }),
                        ..Default::default()
                    },
                    layer: 5,
                },
                TextParams {
                    contents,
                    font_size: 14f32,
                    line_height: 16f32,
                    ..Default::default()
                },
            ))
            .unwrap();
    };

    debug_text(TextContents::from_nodes(&[
        TextNode::Static("Last frame duration = "),
        TextNode::Variable(env_names::DEBUG_PERF_LAST_FRAME_TIME_MILLIS),
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

    pub const DEBUG_PERF_LAST_FRAME_TIME_MILLIS: StringHash =
        janus::hash_string("__debug.perf.last_frame_time.millis");

    pub const DEBUG_COUNTER_LATTICE_NODES: StringHash =
        janus::hash_string("__debug.counter.lattice.nodes");
    pub const DEBUG_COUNTER_LATTICE_CONSTRAINTS: StringHash =
        janus::hash_string("__debug.counter.lattice.constraints");
    pub const DEBUG_COUNTER_FRAGMENTS: StringHash = janus::hash_string("__debug.counter.fragments");
    pub const DEBUG_COUNTER_CAGE_POINTS: StringHash =
        janus::hash_string("__debug.counter.cage_points");
    pub const DEBUG_COUNTER_DEBRIS: StringHash = janus::hash_string("__debug.counter.debris");
}
