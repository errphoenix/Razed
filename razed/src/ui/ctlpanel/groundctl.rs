use ethel::IndirectIndex;
use gui::{
    CommonWidgetOptions, ElementOptions, InteractableCallback, InterfaceSystem, LayoutOptions,
    Point, Rectangle, SliderOptions, TextContents, TextNode, TextOptions, Value, WidgetId,
    env::EnvValue, style_layout,
};
use janus::StringMap;

use crate::{
    render::graphics::Groundplane,
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
        .create_element(crate::ui::params_panel_dev_options(
            Some(root),
            style_layout!(position-absolute 8.0, 900.0),
            Some(style_layout!(510.0, 180.0)),
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
            .create_element(ElementOptions::Slider(
                CommonWidgetOptions {
                    parent: Some(root),
                    layer: 5,
                    layout: LayoutOptions {
                        size: Some(Point::new(Value::Absolute(360.0), Value::Absolute(16.0))),
                        padding: Some(Rectangle::new(
                            Value::Absolute(0f32),
                            Value::Absolute(8f32),
                            Value::Absolute(0f32),
                            Value::Absolute(8f32),
                        )),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                SliderOptions {
                    text: Some(TextOptions {
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
            .create_element(ElementOptions::Slider(
                CommonWidgetOptions {
                    parent: Some(root),
                    layer: 5,
                    layout: LayoutOptions {
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
                    ..Default::default()
                },
                SliderOptions {
                    text: Some(TextOptions {
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
