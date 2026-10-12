use ethel::data::IndirectIndex;
use gui::{
    ContainerLayout, ContentAlignment, CoreElementParams, ElementParams, FloatGrabArea,
    FloatParams, InteractableCallback, InterfaceSystem, ItemAlignment, LayoutOptions,
    LayoutPosition, PanelParams, Point, Rectangle, SliderParams, TextContents, TextNode,
    TextParams, Value, WidgetId, Wrap, env::EnvValue, style::FlexDirection,
};
use janus::StringMap;

use crate::{
    data::Groundplane,
    ui::{
        env_names::{self, DEBUG_CTL_GROUND_SCALE, DEBUG_CTL_GROUND_UVSCALE},
        widget_names::DEBUG_CTL_GROUND_DRAW_BUTTON,
    },
};

pub fn root(
    system: &mut InterfaceSystem,
    root: WidgetId,
    map: &mut StringMap<(WidgetId, IndirectIndex)>,
) -> WidgetId {
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
                    layout_position: LayoutPosition::Absolute {
                        x: Some(Value::Percentage(0.5f32)),
                        y: Some(Value::Percentage(0.4f32)),
                    },
                    size: Some(Point {
                        x: Value::Absolute(510f32),
                        y: Value::Absolute(240f32),
                    }),
                    padding: Some(Rectangle {
                        top: Value::Absolute(26f32),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                layer: 5,
            },
            PanelParams {
                float_params: Some(FloatParams {
                    base_pos: None,
                    grab_area: FloatGrabArea::SectionHoriz { height: 20f32 },
                }),
                ..Default::default()
            },
        ))
        .unwrap()
        .0;

    let dbg_button_drawground = super::dbg_toggle_button::<
        { env_names::DEBUG_CTL_GROUND_DRAW.as_u64() },
    >(system, root, "Toggle Ground");
    map.insert(DEBUG_CTL_GROUND_DRAW_BUTTON, dbg_button_drawground);

    // ground scale
    {
        system
            .create_element(ElementParams::Slider(
                CoreElementParams {
                    parent: Some(root),
                    children: None,
                    layout_options: LayoutOptions {
                        size: Some(Point::new(Value::Absolute(360.0), Value::Absolute(16.0))),
                        padding: Some(Rectangle::new(
                            Value::Absolute(0f32),
                            Value::Absolute(8f32),
                            Value::Absolute(0f32),
                            Value::Absolute(8f32),
                        )),
                        ..Default::default()
                    },
                    layer: 5,
                },
                SliderParams {
                    text: Some(TextParams {
                        contents: TextContents::from_nodes(&[
                            TextNode::Static("geom.ground.scale = "),
                            TextNode::VariableAnd {
                                env_id: DEBUG_CTL_GROUND_SCALE,
                                operation: |value| {
                                    let norm = value.as_float().unwrap_or_default();
                                    let v = Groundplane::scale_from_normalized(norm);
                                    EnvValue::Float(v)
                                },
                            },
                        ]),
                        ..Default::default()
                    }),

                    scroll_step: 0.01,
                    value_init: Groundplane::DEFAULT_SCALE_NORMALIZED,
                    value_sync_handle: Some(DEBUG_CTL_GROUND_SCALE),
                    callback: InteractableCallback::Repeating(|env, v| {
                        env.insert(DEBUG_CTL_GROUND_SCALE, *v);
                    }),

                    ..Default::default()
                },
            ))
            .unwrap();
    }
    // ground UV scale
    {
        system
            .create_element(ElementParams::Slider(
                CoreElementParams {
                    parent: Some(root),
                    children: None,
                    layout_options: LayoutOptions {
                        size: Some(Point::new(Value::Absolute(360.0), Value::Absolute(16.0))),
                        margin: Some(Rectangle {
                            top: Value::Absolute(12f32),
                            ..Default::default()
                        }),
                        padding: Some(Rectangle::new(
                            Value::Absolute(0f32),
                            Value::Absolute(8f32),
                            Value::Absolute(0f32),
                            Value::Absolute(8f32),
                        )),
                        ..Default::default()
                    },
                    layer: 5,
                },
                SliderParams {
                    text: Some(TextParams {
                        contents: TextContents::from_nodes(&[
                            TextNode::Static("geom.ground.uv-scale = "),
                            TextNode::VariableAnd {
                                env_id: DEBUG_CTL_GROUND_UVSCALE,
                                operation: |value| {
                                    let norm = value.as_float().unwrap_or_default();
                                    let v = Groundplane::uvscale_from_normalized(norm);
                                    EnvValue::Float(norm * v)
                                },
                            },
                        ]),
                        ..Default::default()
                    }),

                    scroll_step: 0.01,
                    value_init: Groundplane::DEFAULT_UVSCALE_NORMALIZED,
                    value_sync_handle: Some(DEBUG_CTL_GROUND_UVSCALE),
                    callback: InteractableCallback::Repeating(|env, v| {
                        env.insert(DEBUG_CTL_GROUND_UVSCALE, *v);
                    }),

                    ..Default::default()
                },
            ))
            .unwrap();
    }

    root
}
