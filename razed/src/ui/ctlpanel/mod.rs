use gui::{button_callback, declare::UiPanel, slider_callback, text_contents};

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

pub fn root(system: &mut InterfaceSystem, root: WidgetId, resolution: Resolution) -> WidgetId {
    let this = UiPanel::new(resolution)
        .with_parent(root)
        .with_offset((8.0, 500.0))
        .with_size((420.0, 240.0))
        .with_padding((0.0, 20.0, 0.0, 0.0))
        .builder(|mut stack, props| {
            stack.button(|button| {
                button.builder(|_, props| {
                    props
                        .with_label_literal("Toggle V-Sync")
                        .with_callback(button_callback!(toggle DEBUG_CTL_DISPLAY_VSYNC))
                })
            });
            stack.button(|button| {
                button.builder(|_, props| {
                    props
                        .with_label_literal("Toggle Lattice Viz")
                        .with_callback(button_callback!(toggle DEBUG_CTL_RENDER_LATTICE))
                })
            });
            stack.button(|button| {
                button.builder(|_, props| {
                    props
                        .with_label_literal("Toggle Cage Viz")
                        .with_callback(button_callback!(toggle DEBUG_CTL_RENDER_CAGE))
                })
            });

            stack.slider(|slider| {
                slider.with_padding((0.0, 8.0)).builder(|mut stack, props| {
                    stack.text_from(text_contents!(
                        "graphics.gamma = ",
                        |DEBUG_CTL_GRAPHICS_GAMMA: v_gamma| {
                            let norm = v_gamma.as_float().unwrap_or_default();
                            EnvValue::Float(Gamma::from_normalized(norm))
                        }
                    ));
                    props
                        .with_starting_value(graphics::Gamma::DEFAULT_NORMALIZED)
                        .with_sync_handle(DEBUG_CTL_GRAPHICS_GAMMA)
                        .with_callback(slider_callback!(copy DEBUG_CTL_GRAPHICS_GAMMA))
                })
            });

            stack.slider(|slider| {
                slider.with_padding((0.0, 8.0)).builder(|mut stack, props| {
                    stack.text_from(text_contents!(
                        "shading.bias.roughness = ",
                        DEBUG_CTL_SHADE_BIAS_ROUGHNESS
                    ));
                    props
                        .with_starting_value(0.0)
                        .with_scroll_step(0.01)
                        .with_sync_handle(DEBUG_CTL_SHADE_BIAS_ROUGHNESS)
                        .with_callback(slider_callback!(copy DEBUG_CTL_SHADE_BIAS_ROUGHNESS))
                })
            });
            stack.slider(|slider| {
                slider.with_padding((0.0, 8.0)).builder(|mut stack, props| {
                    stack.text_from(text_contents!(
                        "shading.bias.metallic = ",
                        DEBUG_CTL_SHADE_BIAS_METALLIC
                    ));
                    props
                        .with_starting_value(0.0)
                        .with_scroll_step(0.01)
                        .with_sync_handle(DEBUG_CTL_SHADE_BIAS_METALLIC)
                        .with_callback(slider_callback!(copy DEBUG_CTL_SHADE_BIAS_METALLIC))
                })
            });

            props.with_floating(FloatGrabArea::top(20.0))
        })
        .apply(system);
    shade_mode_selector(system, this);
    this
}
