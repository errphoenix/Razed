use ethel::render::Resolution;
use gui::{
    ButtonCallback, ButtonParams, ContainerLayout, ContentAlignment, CoreElementParams,
    ElementParams, InterfaceSystem, ItemAlignment, LayoutOptions, LayoutPosition, PanelParams,
    Point, Rectangle, TextParams, Value, Wrap, draw::Batch, style::FlexDirection,
};
use janus::texture::TextureKey;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Hash)]
pub struct UiRenderCommandBasic {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub instance_offset: u32,
    pub texture_units: [Option<TextureKey>; Batch::UNITS],
}
impl UiRenderCommandBasic {
    pub fn bind_texture_units(&self) {
        janus::assert_gl!();

        self.texture_units
            .iter()
            .enumerate()
            .filter_map(|(i, key)| key.and_then(|key| Some((i, key))))
            .for_each(|(index, key)| {
                use janus::texture::TextureTarget;
                janus::texture::bind_without_meta(TextureTarget::Flat, key, index as u32);
            });
    }
}

pub const GENERIC_PANEL_PARAMS: PanelParams = PanelParams {
    color: glam::Vec3::ZERO,
    hover_tint: glam::Vec4::ZERO,
    opacity: 0.6,
};

pub fn initialize_default(resolution: Resolution) -> InterfaceSystem {
    let mut system = InterfaceSystem::new(resolution);

    let root = system
        .create_element(ElementParams::Panel(
            CoreElementParams {
                parent: None,
                children: None,
                layout_options: LayoutOptions {
                    container: ContainerLayout::Flexbox {
                        direction: FlexDirection::Column,
                        wrap: Wrap::DontWrap,
                        justify_content: ContentAlignment::Center,
                        align_content: ContentAlignment::Center,
                        align_items: ItemAlignment::Center,
                    },
                    layout_position: LayoutPosition::Relative,
                    //margin: Some(Rectangle::splat(Value::Absolute(64.0))),
                    margin: Some(Rectangle {
                        left: Value::Absolute(64.0),
                        ..Default::default()
                    }),
                    size: Some(Point {
                        x: Value::Absolute(1024.0),
                        y: Value::Absolute(512.0),
                    }),
                    ..Default::default()
                },
                layer: 5,
            },
            GENERIC_PANEL_PARAMS,
        ))
        .unwrap()
        .0;

    let subpanel = system
        .create_element(ElementParams::Panel(
            CoreElementParams {
                parent: Some(root),
                children: None,
                layout_options: LayoutOptions {
                    container: ContainerLayout::Block,
                    layout_position: LayoutPosition::Relative,
                    size: Some(Point {
                        x: Value::Absolute(256.0),
                        y: Value::Absolute(128.0),
                    }),
                    align_self: ItemAlignment::Center,
                    justify_self: ItemAlignment::Center,
                    margin: Some(Rectangle::ZERO),
                    ..Default::default()
                },
                layer: 6,
            },
            GENERIC_PANEL_PARAMS,
        ))
        .unwrap()
        .0;

    system
        .create_element(ElementParams::Text(
            CoreElementParams {
                parent: Some(subpanel),
                children: None,
                layout_options: LayoutOptions {
                    container: ContainerLayout::Block,
                    size: Some(Point {
                        x: Value::Absolute(160.0),
                        y: Value::Absolute(72.0),
                    }),
                    ..Default::default()
                },
                layer: 7,
            },
            TextParams {
                string: *ethel::lazy_hash_str!("Quitj"),
                font_name: *ethel::lazy_hash_str!("Arial"),
                color: glam::Vec4::ONE,
                font_size: 11f32,
                line_height: 12f32,
            },
        ))
        .unwrap();

    system
}
