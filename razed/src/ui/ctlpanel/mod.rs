use gui::{button_callback, declare::UiPanel, slider_callback, text_contents};

pub mod groundctl;
pub mod profile;

use crate::simulation::settings::{
    GfxGamma, PROP_ID_DBG_DRAW_CAGE, PROP_ID_DBG_DRAW_LATTICE, PROP_ID_DBG_SHADING_BIAS_METAL,
    PROP_ID_DBG_SHADING_BIAS_ROUGH, PROP_ID_DBG_SHADING_MODE, PROP_ID_DISPLAY_VSYNC,
    PROP_ID_GFX_GAMMA, ShadingMode,
};

pub use super::*;

pub fn root(system: &mut InterfaceSystem, root: WidgetId, resolution: Resolution) -> WidgetId {
    UiPanel::new(resolution)
        .with_parent(root)
        .with_offset((8.0, 500.0))
        .with_size((420.0, 280.0))
        .with_padding(crate::ui::DEV_PANEL_PADDING)
        .builder(|mut stack, props| {
            stack.button(|button| {
                button.builder(|_, props| {
                    props
                        .with_label_literal("Toggle V-Sync")
                        .with_callback(button_callback!(toggle PROP_ID_DISPLAY_VSYNC))
                })
            });
            stack.button(|button| {
                button.builder(|_, props| {
                    props
                        .with_label_literal("Toggle Lattice Viz")
                        .with_callback(button_callback!(toggle PROP_ID_DBG_DRAW_LATTICE))
                })
            });
            stack.button(|button| {
                button.builder(|_, props| {
                    props
                        .with_label_literal("Toggle Cage Viz")
                        .with_callback(button_callback!(toggle PROP_ID_DBG_DRAW_CAGE))
                })
            });

            stack.slider(|slider| {
                slider.with_padding((0.0, 8.0)).builder(|mut stack, props| {
                    stack.text_from(text_contents!(
                        "graphics.gamma = ",
                        |PROP_ID_GFX_GAMMA: v_gamma| {
                            let norm = v_gamma.as_float().unwrap_or_default();
                            EnvValue::Float(GfxGamma::from_normalized(norm))
                        }
                    ));
                    props
                        .with_starting_value(GfxGamma::DEFAULT_NORMALIZED)
                        .with_sync_handle(PROP_ID_GFX_GAMMA)
                        .with_callback(slider_callback!(copy PROP_ID_GFX_GAMMA))
                })
            });

            stack.slider(|slider| {
                slider.with_padding((0.0, 8.0)).builder(|mut stack, props| {
                    stack.text_from(text_contents!(
                        "shading.bias.roughness = ",
                        PROP_ID_DBG_SHADING_BIAS_ROUGH
                    ));
                    props
                        .with_starting_value(0.0)
                        .with_scroll_step(0.01)
                        .with_sync_handle(PROP_ID_DBG_SHADING_BIAS_ROUGH)
                        .with_callback(slider_callback!(copy PROP_ID_DBG_SHADING_BIAS_ROUGH))
                })
            });
            stack.slider(|slider| {
                slider.with_padding((0.0, 8.0)).builder(|mut stack, props| {
                    stack.text_from(text_contents!(
                        "shading.bias.metallic = ",
                        PROP_ID_DBG_SHADING_BIAS_METAL
                    ));
                    props
                        .with_starting_value(0.0)
                        .with_scroll_step(0.01)
                        .with_sync_handle(PROP_ID_DBG_SHADING_BIAS_METAL)
                        .with_callback(slider_callback!(copy PROP_ID_DBG_SHADING_BIAS_METAL))
                })
            });
            stack.dropdown_enum::<ShadingMode>(PROP_ID_DBG_SHADING_MODE);
            props.with_floating(FloatGrabArea::top(20.0))
        })
        .apply(system)
}
