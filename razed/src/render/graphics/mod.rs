pub mod envmap;
pub mod materials;

pub use envmap::*;
pub use materials::*;

use janus::sync::TriCell;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Gamma(f32);
impl Default for Gamma {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}
impl Gamma {
    pub const DEFAULT: f32 = 2.2;
    pub const MAX: f32 = 3.4;
    pub const INV_MAX: f32 = 1.0 / Self::MAX;
    pub const DEFAULT_NORMALIZED: f32 = Self::DEFAULT * Self::INV_MAX;

    pub const fn new_clamped(gamma: f32) -> Self {
        Self(gamma.clamp(0f32, Self::MAX))
    }

    pub const fn from_normalized(normalized: f32) -> Self {
        if normalized < 0f32 {
            Self(Self::DEFAULT)
        } else {
            Self::new_clamped(normalized * Self::MAX)
        }
    }

    pub const fn as_f32(&self) -> f32 {
        self.0
    }

    pub const fn normalize(&self) -> f32 {
        self.0 * Self::INV_MAX
    }
}

#[derive(Debug, Default)]
pub struct RenderParams {
    pub gamma: TriCell<Gamma>,
    pub exposure: TriCell<f32>, //todo
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RenderStats {
    pub gbank_vuse_perc: f32,
    pub gbank_tuse_perc: f32,
    pub gbank_mprint: usize,
    pub tris_count: u32,
    pub instance_count: u32,
    pub instancelist_count: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct DebugRenderOptions {
    pub draw_viz_lattice: bool,
    pub draw_viz_cage: bool,
    pub bias_roughness: f32,
    pub bias_metallic: f32,
}
impl Default for DebugRenderOptions {
    fn default() -> Self {
        Self {
            draw_viz_lattice: true,
            draw_viz_cage: true,
            bias_roughness: 0f32,
            bias_metallic: 0f32,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Groundplane {
    pub scale: f32,
    pub uvscale: f32,
    pub draw: bool,
}
impl Default for Groundplane {
    fn default() -> Self {
        Self {
            scale: Self::DEFAULT_SCALE,
            uvscale: Default::default(),
            draw: true,
        }
    }
}
impl Groundplane {
    pub const MAX_SCALE: f32 = 1024f32;
    pub const DEFAULT_SCALE: f32 = 64f32;
    pub const DEFAULT_SCALE_NORMALIZED: f32 = Self::DEFAULT_SCALE / Self::MAX_SCALE;
    const INV_MAX_SCALE: f32 = 1f32 / Self::MAX_SCALE;

    pub const MAX_UVSCALE: f32 = 64f32;
    pub const DEFAULT_UVSCALE: f32 = 4f32;
    pub const DEFAULT_UVSCALE_NORMALIZED: f32 = Self::DEFAULT_UVSCALE / Self::MAX_UVSCALE;
    const INV_MAX_UVSCALE: f32 = 1f32 / Self::MAX_UVSCALE;

    pub const fn scale_from_normalized(normalized: f32) -> f32 {
        normalized * Self::MAX_SCALE
    }
    pub const fn normalize_scale(scale: f32) -> f32 {
        scale * Self::INV_MAX_SCALE
    }

    pub const fn uvscale_from_normalized(normalized: f32) -> f32 {
        normalized * Self::MAX_UVSCALE
    }
    pub const fn normalize_uvscale(uvscale: f32) -> f32 {
        uvscale * Self::INV_MAX_UVSCALE
    }
}
