use ethel::render::Resolution;
use gui::{
    CommonLayoutOptions, CoreElementParams, ElementParams, InterfaceSystem, LayoutStyle,
    PanelParams, Point, Value, draw::Batch,
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

    system
        .create_element(ElementParams::Panel(
            CoreElementParams {
                layout_style: LayoutStyle::Block {
                    inset: None,
                    position: Some(Point {
                        x: Value::Absolute(128.0),
                        y: Value::Percentage(0.25),
                    }),
                    common: CommonLayoutOptions {
                        size: Some(Point {
                            x: Value::Absolute(256.0),
                            y: Value::Absolute(256.0),
                        }),
                        ..Default::default()
                    },
                },
                layer: 5,
                ..Default::default()
            },
            PanelParams {
                color: glam::Vec3::X,
                hover_tint: glam::vec4(0.0, 1.0, 0.0, 1.0),
                opacity: 1.0,
            },
        ))
        .unwrap();

    system
}
