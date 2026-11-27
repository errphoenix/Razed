use ethel::Resolution;
use gui::{
    FloatGrabArea, InterfaceSystem, LayoutOptions, WidgetId, button_callback, declare::UiPanel,
    env::EnvValue, slider_callback, style_layout, text_contents,
};

use crate::simulation::settings::{
    DbgGroundScale, DbgGroundUvScale, PROP_ID_DBG_DRAW_GROUND, PROP_ID_DBG_GROUND_SCALE,
    PROP_ID_DBG_GROUND_UV_SCALE,
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
                        .with_callback(button_callback!(toggle PROP_ID_DBG_DRAW_GROUND))
                })
            });
            stack.slider(|slider| {
                slider
                    .with_layout(LayoutOptions {
                        padding: Some(style_layout!(horiz 0.0, vert 8.0)),
                        ..Default::default()
                    })
                    .builder(|mut stack, props| {
                        stack.text_from(text_contents!(
                            "geom.ground.scale= ",
                            |PROP_ID_DBG_GROUND_SCALE: v_norm| {
                                let norm = v_norm.as_float().unwrap_or_default();
                                EnvValue::Float(DbgGroundScale::from_normalized(norm))
                            }
                        ));
                        props
                            // .with_text(TextOptions::from_contents())
                            .with_scroll_step(0.01)
                            .with_starting_value(DbgGroundScale::DEFAULT_NORMALIZED)
                            .with_sync_handle(PROP_ID_DBG_GROUND_SCALE)
                            .with_callback(slider_callback!(copy PROP_ID_DBG_GROUND_SCALE))
                    })
            });
            stack.slider(|slider| {
                slider
                    .with_layout(LayoutOptions {
                        padding: Some(style_layout!(horiz 0.0, vert 8.0)),
                        ..Default::default()
                    })
                    .builder(|mut stack, props| {
                        stack.text_from(text_contents!(
                            "geom.ground.uv-scale= ",
                            |PROP_ID_DBG_GROUND_UV_SCALE: v_norm| {
                                let norm = v_norm.as_float().unwrap_or_default();
                                EnvValue::Float(DbgGroundUvScale::from_normalized(norm))
                            }
                        ));
                        props
                            .with_scroll_step(0.01)
                            .with_starting_value(DbgGroundUvScale::DEFAULT_NORMALIZED)
                            .with_sync_handle(PROP_ID_DBG_GROUND_UV_SCALE)
                            .with_callback(slider_callback!(copy PROP_ID_DBG_GROUND_UV_SCALE))
                    })
            });
            props.with_floating(FloatGrabArea::top(20.0))
        })
        .apply(system)
}
