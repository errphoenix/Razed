use gui::DEFAULT_GENERIC_COLOR;

pub use super::*;

pub fn shade_mode_selector(system: &mut InterfaceSystem, root: WidgetId) -> WidgetId {
    let panel = system
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
                    align_self: ItemAlignment::Center,
                    justify_self: ItemAlignment::Center,
                    //layout_position: (),
                    //size: (),
                    ..Default::default()
                },
                layer: 5,
            },
            Default::default(),
        ))
        .unwrap()
        .0;

    system
        .create_element(ElementParams::Text(
            CoreElementParams {
                parent: Some(panel),
                children: None,
                layout_options: LayoutOptions::default(),
                layer: 5,
            },
            TextParams {
                contents: TextContents::from_node(TextNode::Static("shading-mode:")),
                ..Default::default()
            },
        ))
        .unwrap()
        .0;

    system
        .create_element(ElementParams::Button(
            CoreElementParams {
                parent: Some(panel),
                children: None,
                layout_options: LayoutOptions {
                    align_self: ItemAlignment::Start,
                    justify_self: ItemAlignment::Start,
                    size: Some(Point::new(Value::Absolute(134f32), Value::Absolute(18f32))),
                    ..Default::default()
                },
                layer: 5,
            },
            ButtonParams {
                text: TextParams {
                    contents: TextContents::from_node(TextNode::VariableAnd {
                        env_id: DEBUG_CTL_SHADE_MODE,
                        operation: |var| {
                            let id = var.as_integer().unwrap_or_default() as u32;
                            EnvValue::from_str(match id {
                                0 => "Standard/PBR|dbg",
                                1 => "Attr./Barycentric Weights",
                                2 => "Attr./Perspective Normals",
                                3 => "Attr./Screen Derivatives",
                                4 => "Geom./Visibility Buffer",
                                _ => "??? ??? ???",
                            })
                        },
                    }),
                    ..Default::default()
                },
                bg_color: glam::vec3(0.6, 0.3, 0.0),
                bg_hover_tint: glam::vec4(0.65, 0.3, 0.0, 0.95),
                bg_press_tint: glam::vec4(0.3, 0.15, 0.0, 0.95),
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

    let dbg_button_vsync = system
        .create_element(ElementParams::Button(
            CoreElementParams {
                parent: Some(root),
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
                InteractableCallback::Once(|env, _time| {
                    if let Some(vsync) = env.get_mut(&env_names::DEBUG_CTL_DISPLAY_VSYNC) {
                        let v = vsync.as_boolean_mut().unwrap();
                        *v = !*v;
                    }
                }),
            ),
        ))
        .unwrap();
    map.insert(DEBUG_CTL_VSYNC_BUTTON, dbg_button_vsync);

    system
        .create_element(ElementParams::Slider(
            CoreElementParams {
                parent: Some(root),
                children: None,
                layout_options: LayoutOptions {
                    size: Some(Point::new(Value::Absolute(256.0), Value::Absolute(16.0))),
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

    root
}

const fn params_dbg_button(
    text: &'static str,
    cb: InteractableCallback<InteractionTime>,
) -> ButtonParams {
    params_dbg_button_with_text(TextNode::Static(text), cb)
}

const fn params_dbg_button_with_text(
    text: TextNode,
    cb: InteractableCallback<InteractionTime>,
) -> ButtonParams {
    ButtonParams {
        text: TextParams {
            contents: TextContents::from_node(text),
            never_invalidate: true,
            font_name: TextParams::DEFAULT_FONT,
            color: TextParams::DEFAULT_COLOR,
            font_size: TextParams::DEFAULT_FONT_SIZE,
            line_height: TextParams::DEFAULT_LINE_HEIGHT,
        },
        bg_color: DEFAULT_GENERIC_COLOR,
        bg_hover_tint: COLORTINT_HOVER_INVARIANT,
        bg_press_tint: COLORTINT_HOVER_INVARIANT,
        callback: cb,
    }
}
