use gui::{
    ContainerLayout, ContentAlignment, CoreElementParams, ElementParams, FloatGrabArea,
    FloatParams, InterfaceSystem, ItemAlignment, LayoutOptions, LayoutPosition, PanelParams, Point,
    Rectangle, TextContents, TextNode, TextParams, Value, WidgetId, Wrap, env::EnvValue,
    style::FlexDirection,
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
                        direction: FlexDirection::Column,
                        wrap: Wrap::Wrap,
                        justify_content: ContentAlignment::Stretch,
                        align_content: ContentAlignment::Stretch,
                        align_items: ItemAlignment::Stretch,
                    },
                    layout_position: LayoutPosition::Absolute {
                        x: Some(Value::Absolute(8f32)),
                        y: Some(Value::Percentage(0.5f32)),
                    },
                    size: Some(Point {
                        x: Value::Absolute(240f32),
                        y: Value::Absolute(160f32),
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
