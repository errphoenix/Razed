pub mod envmap;
pub mod materials;

#[allow(unused_imports)]
pub use envmap::*;
use janus::sync::TriCell;
#[allow(unused_imports)]
pub use materials::*;

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
