use ethel::render::Resolution;
use gui::{
    ContainerLayout, ContentAlignment, CoreElementParams, ElementParams, InterfaceSystem,
    ItemAlignment, LayoutOptions, LayoutPosition, PanelParams, Point, Rectangle, Value, Wrap,
    draw::Batch, style::FlexDirection,
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
            PanelParams {
                color: glam::Vec3::X,
                hover_tint: glam::Vec4::ONE,
                opacity: 1.0,
            },
        ))
        .unwrap()
        .0;

    system
        .create_element(ElementParams::Panel(
            CoreElementParams {
                parent: Some(root),
                children: None,
                layout_options: LayoutOptions {
                    container: ContainerLayout::Block,
                    layout_position: LayoutPosition::Relative,
                    size: Some(Point {
                        x: Value::Absolute(128.0),
                        y: Value::Absolute(128.0),
                    }),
                    align_self: ItemAlignment::Center,
                    justify_self: ItemAlignment::Center,
                    margin: Some(Rectangle::ZERO),
                    ..Default::default()
                },
                layer: 6,
            },
            PanelParams {
                color: glam::Vec3::Y,
                hover_tint: glam::vec4(0.0, 1.0, 0.0, 1.0),
                opacity: 1.0,
            },
        ))
        .unwrap();

    system
}
