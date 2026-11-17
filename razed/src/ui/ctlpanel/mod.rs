use gui::{ButtonCallback, DEFAULT_GENERIC_COLOR, FloatGrabArea, FloatParams};

pub mod groundctl;
pub mod profile;

pub use super::*;

pub fn shade_mode_selector(system: &mut InterfaceSystem, root: WidgetId) -> WidgetId {
    let panel = system
        .create_element(ElementOptions::Panel(
            CommonWidgetOptions {
                parent: Some(root),
                layer: 5,
                layout: LayoutOptions {
                    container: ContainerLayout::Flexbox {
                        direction: style_layout!(flex - column),
                        wrap: style_layout!(flex - wrap),
                        justify_content: style_layout!(align - content - stretch),
                        align_content: style_layout!(align - content - stretch),
                        align_items: style_layout!(align - item - stretch),
                    },
                    margin: Some(style_layout!(top 16.0)),
                    ..Default::default()
                },
                ..Default::default()
            },
            PanelOptions::hidden(),
        ))
        .unwrap()
        .0;

    system
        .create_element(ElementOptions::Text(
            CommonWidgetOptions {
                parent: Some(panel),
                layer: 5,
                ..Default::default()
            },
            TextOptions {
                contents: TextContents::from_node(TextNode::Static("shading-mode:")),
                ..Default::default()
            },
        ))
        .unwrap()
        .0;

    system
        .create_element(ElementOptions::Button(
            CommonWidgetOptions {
                parent: Some(panel),
                layer: 5,
                layout: LayoutOptions {
                    size: Some(Point::new(Value::Absolute(256f32), Value::Absolute(22f32))),
                    ..Default::default()
                },
                ..Default::default()
            },
            ButtonOptions {
                text: TextOptions {
                    contents: TextContents::from_node(TextNode::VariableAnd {
                        env_id: DEBUG_CTL_SHADE_MODE,
                        operation: |var| {
                            let id = var.as_integer().unwrap_or_default() as u32;
                            EnvValue::from_str(match id {
                                0 => "Standard/PBR",
                                1 => "Attr./Barycentric Weights",
                                2 => "Attr./Tangent Frame",
                                3 => "Attr./Screen Derivatives",
                                4 => "Geom./Visibility Buffer",
                                _ => "??? ??? ???",
                            })
                        },
                    }),
                    ..Default::default()
                },
                callback: InteractableCallback::Once(|env, _time| {
                    const OPTIONS_COUNT: u32 = 5;
                    env.modify_or_default(DEBUG_CTL_SHADE_MODE, |id| {
                        let iid = id.as_integer().unwrap_or_default();
                        if let Some(id) = id.as_integer_mut() {
                            *id = (iid + 1) % OPTIONS_COUNT as i32;
                        }
                    });
                }),
            },
        ))
        .unwrap()
        .0;

    panel
}

pub fn root(system: &mut InterfaceSystem, root: WidgetId) -> WidgetId {
    let root = system
        .create_element(crate::ui::params_panel_dev_options(
            Some(root),
            style_layout!(position-absolute 8.0, 500.0),
            Some(style_layout!(420.0, 240.0)),
        ))
        .unwrap()
        .0;

    dbg_toggle_button::<{ env_names::DEBUG_CTL_DISPLAY_VSYNC.as_u64() }>(
        system,
        root,
        "Toggle: V-sync",
    );
    dbg_toggle_button::<{ env_names::DEBUG_CTL_RENDER_LATTICE.as_u64() }>(
        system,
        root,
        "Viz: lattice",
    );
    dbg_toggle_button::<{ env_names::DEBUG_CTL_RENDER_CAGE.as_u64() }>(system, root, "Viz: cage");

    // gamma slider
    system
        .create_element(ElementOptions::Slider(
            CommonWidgetOptions {
                parent: Some(root),
                layer: 5,
                layout: LayoutOptions {
                    size: Some(Point::new(Value::Absolute(256.0), Value::Absolute(16.0))),
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
                        TextNode::Static("graphics.gamma = "),
                        TextNode::VariableAnd {
                            env_id: DEBUG_CTL_GRAPHICS_GAMMA,
                            operation: |value| {
                                let gamma_norm = value.as_float().unwrap_or_default();
                                let gamma_param = Gamma::from_normalized(gamma_norm);
                                EnvValue::Float(gamma_param.as_f32())
                            },
                        },
                    ]),
                    ..Default::default()
                }),
                value_init: graphics::Gamma::DEFAULT_NORMALIZED,
                value_sync_handle: Some(DEBUG_CTL_GRAPHICS_GAMMA),
                callback: InteractableCallback::Repeating(|env, v| {
                    env.insert(DEBUG_CTL_GRAPHICS_GAMMA, *v);
                }),

                ..Default::default()
            },
        ))
        .unwrap();

    shade_mode_selector(system, root);

    // roughness bias
    system
        .create_element(ElementOptions::Slider(
            CommonWidgetOptions {
                parent: Some(root),
                layer: 5,
                layout: LayoutOptions {
                    size: Some(Point::new(Value::Absolute(256.0), Value::Absolute(16.0))),
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
                    contents: text_contents!(
                        "shading.bias.roughness =",
                        DEBUG_CTL_SHADE_BIAS_ROUGHNESS
                    ),
                    ..Default::default()
                }),
                scroll_step: 0.01,
                value_init: 0f32,
                value_sync_handle: Some(DEBUG_CTL_SHADE_BIAS_ROUGHNESS),
                callback: InteractableCallback::Repeating(|env, v| {
                    env.insert(DEBUG_CTL_SHADE_BIAS_ROUGHNESS, *v);
                }),
                ..Default::default()
            },
        ))
        .unwrap();
    // metallic bias
    system
        .create_element(ElementOptions::Slider(
            CommonWidgetOptions {
                parent: Some(root),
                layer: 5,
                layout: LayoutOptions {
                    size: Some(Point::new(Value::Absolute(256.0), Value::Absolute(16.0))),
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
                    contents: text_contents!(
                        "shading.bias.metallic =",
                        DEBUG_CTL_SHADE_BIAS_METALLIC
                    ),
                    ..Default::default()
                }),
                scroll_step: 0.01,
                value_init: 0f32,
                value_sync_handle: Some(DEBUG_CTL_SHADE_BIAS_METALLIC),
                callback: InteractableCallback::Repeating(|env, v| {
                    env.insert(DEBUG_CTL_SHADE_BIAS_METALLIC, *v);
                }),
                ..Default::default()
            },
        ))
        .unwrap();

    root
}

pub const fn params_dbg_button(text: &'static str, cb: ButtonCallback) -> ButtonOptions {
    params_dbg_button_with_text(
        TextOptions {
            contents: TextContents::from_node(TextNode::Static(text)),
            never_invalidate: true,
            font_name: TextOptions::DEFAULT_FONT,
            font_size: TextOptions::DEFAULT_FONT_SIZE,
            line_height: TextOptions::DEFAULT_LINE_HEIGHT,
        },
        cb,
    )
}

pub const fn params_dbg_button_with_text(text: TextOptions, cb: ButtonCallback) -> ButtonOptions {
    ButtonOptions { text, callback: cb }
}

pub fn dbg_toggle_button<const PROP: u64>(
    system: &mut InterfaceSystem,
    root: WidgetId,
    label: &'static str,
) -> (WidgetId, IndirectIndex) {
    dbg_toggle_button_with(
        system,
        root,
        label,
        InteractableCallback::Once(|env, _time| {
            if let Some(prop) = env.get_mut(&janus::StringHash::from_u64(PROP)) {
                prop.invert();
            }
        }),
    )
}

pub fn dbg_toggle_button_with(
    system: &mut InterfaceSystem,
    root: WidgetId,
    label: &'static str,
    cb: ButtonCallback,
) -> (WidgetId, IndirectIndex) {
    system
        .create_element(ElementOptions::Button(
            CommonWidgetOptions {
                parent: Some(root),
                layer: 5,
                layout: LayoutOptions {
                    align_self: ItemAlignment::Start,
                    justify_self: ItemAlignment::Start,
                    ..Default::default()
                },
                ..Default::default()
            },
            params_dbg_button(label, cb),
        ))
        .unwrap()
}
