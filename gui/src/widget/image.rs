use ethel::{Column, IndirectIndex, Resolution, TextureId};

use crate::{
    CommonWidgetOptions, ComponentKind, ElementOptions, HasOptions, InterfaceSystem, WidgetError,
    WidgetId,
    declare::{UiImage, UiNodeStack},
};

ethel::table_spec! {
    struct InterfaceImage {
        tint: glam::Vec3;
        tint_opacity: f32;
        texture: TextureId;
    }
}

impl<const LAYERS: usize> InterfaceSystem<LAYERS> {
    /// Make a core element into an image element.
    ///
    /// This requires the core element's `id`.
    ///
    /// # Returns
    /// Returns the `IndirectIndex` of the image data.
    /// Or returns `None` if the given core `id` is invalid.
    pub fn make_image(
        &mut self,
        id: WidgetId,
        tint: glam::Vec3,
        tint_opacity: f32,
        texture: TextureId,
    ) -> Result<IndirectIndex, WidgetError> {
        if let Some(commons_id) = self.data.commons.solve_indirect(id.0) {
            self.assert_null_archetype(commons_id)?;
            let image_element = (tint, tint_opacity, texture);
            let image_id = self.data.images.insert(image_element);
            self.data.commons.archetype[commons_id.as_index()] = ComponentKind::Image(image_id);
            Ok(image_id)
        } else {
            Err(WidgetError::InvalidWidgetHandle(id.0.as_int()))
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct ImageOptions {
    pub tint: glam::Vec3,
    pub tint_opacity: f32,
    pub texture: TextureId,
}
impl Default for ImageOptions {
    fn default() -> Self {
        Self::new(crate::expect_fallback_texture())
    }
}
impl ImageOptions {
    pub const DEFAULT_TINT: glam::Vec3 = glam::Vec3::ZERO;
    pub const DEFAULT_TINT_OPACITY: f32 = 0.0;

    pub const fn new(texture: TextureId) -> Self {
        Self {
            tint: Self::DEFAULT_TINT,
            tint_opacity: Self::DEFAULT_TINT_OPACITY,
            texture,
        }
    }

    pub const fn tinted(texture: TextureId, tint: glam::Vec3) -> Self {
        Self::new(texture).with_tint(tint)
    }

    pub const fn with_texture(self, texture: TextureId) -> Self {
        Self { texture, ..self }
    }

    pub const fn with_tint(self, tint: glam::Vec3) -> Self {
        Self {
            tint,
            tint_opacity: 0.5,
            ..self
        }
    }

    pub const fn with_tint_opacity(self, tint_opacity: f32) -> Self {
        Self {
            tint_opacity,
            ..self
        }
    }
}

#[derive(Debug)]
pub struct Image;
impl HasOptions for Image {
    type Options = ImageOptions;

    fn complement_options(
        common: CommonWidgetOptions,
        this: Self::Options,
        _: Resolution,
    ) -> ElementOptions {
        ElementOptions::Image(common, this)
    }

    fn archetype() -> ComponentKind {
        ComponentKind::Image(Default::default())
    }
}

impl UiNodeStack<'_> {
    pub fn image(&mut self, image_op: impl FnOnce(UiImage) -> UiImage) {
        self.push(image_op(UiImage::new(self.reference_resolution)));
    }

    pub fn image_from(&mut self, image_options: ImageOptions) {
        self.push(UiImage::from(image_options, self.reference_resolution));
    }
}
