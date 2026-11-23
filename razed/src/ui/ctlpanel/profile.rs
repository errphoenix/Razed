use ethel::Resolution;
use gui::{
    FloatGrabArea, InterfaceSystem, WidgetId, button_callback, declare::UiPanel, text_contents,
};

use crate::ui::env_names::{PROFILER_ACTION, PROFILER_FRAME_COUNT, PROFILER_RUNNING};

pub fn root(system: &mut InterfaceSystem, root: WidgetId, resolution: Resolution) -> WidgetId {
    UiPanel::new(resolution)
        .with_parent(root)
        .with_padding(crate::ui::DEV_PANEL_PADDING)
        .with_offset((8.0,720.0))
        .with_size((510.0,180.0))
        .builder(|mut stack, props| {
            stack.text_from(text_contents!("Profiler::running = ", PROFILER_RUNNING));
            stack.text_from(text_contents!("Profiler::frames = ", PROFILER_FRAME_COUNT));
            stack.button(|button| {
                button.builder(|_, props| {
                    props
                        .with_label_literal("Start/Pause")
                        .with_callback(button_callback! { select PROFILER_ACTION =>
                            PROFILER_RUNNING; ? crate::PROFILER_ACTION_PAUSE; : crate::PROFILER_ACTION_START
                        })
                })
            });
            stack.button(|button| {
                button.builder(|_, props| {
                    props.with_label_literal("Dump").with_callback(
                        button_callback!(set PROFILER_ACTION crate::PROFILER_ACTION_DUMP),
                    )
                })
            });
            stack.checkbox_value(true);
            stack.checkbox_value(false);
            props.with_floating(FloatGrabArea::top(20.0))
        })
        .apply(system)
}
