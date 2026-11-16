use gui::{
    ContainerLayout, CoreElementParams, ElementParams, FloatGrabArea, FloatParams, InterfaceSystem,
    LayoutOptions, PanelParams, TextContents, TextNode, TextParams, WidgetId, env::EnvValue,
    style_layout,
};

use crate::{
    PROFILER_ACTION_PAUSE, PROFILER_ACTION_START,
    ui::env_names::{PROFILER_ACTION, PROFILER_FRAME_COUNT, PROFILER_RUNNING},
};

pub fn root(system: &mut InterfaceSystem, root: WidgetId) -> WidgetId {
    let root = system
        .create_element(ElementParams::Panel(
            CoreElementParams {
                parent: Some(root),
                children: None,
                layout_options: LayoutOptions {
                    container: ContainerLayout::Flexbox {
                        direction: style_layout!(flex - column),
                        wrap: style_layout!(flex - wrap),
                        justify_content: style_layout!(align - content - stretch),
                        align_content: style_layout!(align - content - stretch),
                        align_items: style_layout!(align - item - stretch),
                    },
                    layout_position: style_layout!(position-absolute 8.0, 0.5),
                    size: Some(style_layout!(240.0, 160.0)),
                    padding: Some(style_layout!(top 26.0)),
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

    system
        .create_element(ElementParams::Text(
            CoreElementParams {
                parent: Some(root),
                children: None,
                layer: 5,
                layout_options: LayoutOptions {
                    ..Default::default()
                },
            },
            TextParams {
                contents: TextContents::from_nodes(&[
                    TextNode::Static("Profiler::running = "),
                    TextNode::Variable(PROFILER_RUNNING),
                ]),
                // font_name: todo!(),
                // color: todo!(),
                // font_size: todo!(),
                // line_height: todo!(),
                never_invalidate: true,
                ..Default::default()
            },
        ))
        .unwrap();
    system
        .create_element(ElementParams::Text(
            CoreElementParams {
                parent: Some(root),
                children: None,
                layer: 5,
                layout_options: LayoutOptions {
                    ..Default::default()
                },
            },
            TextParams {
                contents: TextContents::from_nodes(&[
                    TextNode::Static("Profiler::frames = "),
                    TextNode::Variable(PROFILER_FRAME_COUNT),
                ]),
                // font_name: todo!(),
                // color: todo!(),
                // font_size: todo!(),
                // line_height: todo!(),
                never_invalidate: true,
                ..Default::default()
            },
        ))
        .unwrap();

    super::dbg_toggle_button_with(
        system,
        root,
        "Start/Pause",
        gui::InteractableCallback::Once(|env, _| {
            let state = env.get_boolean(&PROFILER_RUNNING);
            if let Some(prop) = env
                .get_mut(&PROFILER_ACTION)
                .and_then(EnvValue::as_integer_mut)
            {
                let next_action = if state {
                    PROFILER_ACTION_PAUSE
                } else {
                    PROFILER_ACTION_START
                };
                *prop |= next_action as i32;
            }
        }),
    );
    super::dbg_toggle_button_with(
        system,
        root,
        "Dump",
        gui::InteractableCallback::Once(|env, _| {
            if let Some(prop) = env
                .get_mut(&PROFILER_ACTION)
                .and_then(EnvValue::as_integer_mut)
            {
                *prop |= crate::PROFILER_ACTION_DUMP as i32;
            }
        }),
    );

    root
}
