use ethel::Resolution;
use gui::{
    FloatGrabArea, InterfaceSystem, LayoutOptions, TextOptions, WidgetId, button_callback,
    declare::UiPanel, env::EnvValue, slider_callback, style_layout, text_contents,
};

use crate::{
    render::graphics::Groundplane,
    ui::env_names::{DEBUG_CTL_GROUND_DRAW, DEBUG_CTL_GROUND_SCALE, DEBUG_CTL_GROUND_UVSCALE},
};

pub fn root(system: &mut InterfaceSystem, root: WidgetId, resolution: Resolution) -> WidgetId {
    UiPanel::new(resolution)
        .with_parent(root)
        .with_padding(crate::ui::DEV_PANEL_PADDING)
        .with_offset((8.0, 900.0))
        .with_size((510.0, 180.0))
        .builder(|mut stack, props| {
            stack.button(|button| {
                button.builder(|_, props| {
                    props
                        .with_label_literal("Toggle Ground")
                        .with_callback(button_callback!(toggle DEBUG_CTL_GROUND_DRAW))
                })
            });
            stack.slider(|slider| {
                slider
                    .with_layout(LayoutOptions {
                        padding: Some(style_layout!(horiz 0.0, vert 8.0)),
                        ..Default::default()
                    })
                    .builder(|_, props| {
                        props
                            .with_text(TextOptions::from_contents(text_contents!(
                                "geom.ground.scale= ",
                                |DEBUG_CTL_GROUND_SCALE: v_norm| {
                                    let norm = v_norm.as_float().unwrap_or_default();
                                    EnvValue::Float(Groundplane::scale_from_normalized(norm))
                                }
                            )))
                            .with_scroll_step(0.01)
                            .with_starting_value(Groundplane::DEFAULT_SCALE_NORMALIZED)
                            .with_sync_handle(DEBUG_CTL_GROUND_SCALE)
                            .with_callback(slider_callback!(copy DEBUG_CTL_GROUND_SCALE))
                    })
            });
            stack.slider(|slider| {
                slider
                    .with_layout(LayoutOptions {
                        padding: Some(style_layout!(horiz 0.0, vert 8.0)),
                        ..Default::default()
                    })
                    .builder(|_, props| {
                        props
                            .with_text(TextOptions::from_contents(text_contents!(
                                "geom.ground.uv-scale= ",
                                |DEBUG_CTL_GROUND_UVSCALE: v_norm| {
                                    let norm = v_norm.as_float().unwrap_or_default();
                                    EnvValue::Float(Groundplane::uvscale_from_normalized(norm))
                                }
                            )))
                            .with_scroll_step(0.01)
                            .with_starting_value(Groundplane::DEFAULT_UVSCALE_NORMALIZED)
                            .with_sync_handle(DEBUG_CTL_GROUND_UVSCALE)
                            .with_callback(slider_callback!(copy DEBUG_CTL_GROUND_UVSCALE))
                    })
            });
            props.with_floating(FloatGrabArea::top(20.0))
        })
        .apply(system)
}
