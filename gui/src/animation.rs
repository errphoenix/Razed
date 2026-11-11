use std::time::Duration;

use glam::FloatExt;
use janus::DeltaTime;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Translation(pub glam::Vec2);
impl std::ops::DerefMut for Translation {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl std::ops::Deref for Translation {
    type Target = glam::Vec2;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<glam::Vec2> for Translation {
    fn from(value: glam::Vec2) -> Self {
        Self(value)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Rotation(pub f32);
impl std::ops::DerefMut for Rotation {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl std::ops::Deref for Rotation {
    type Target = f32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<f32> for Rotation {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scaling(pub f32);
impl Default for Scaling {
    fn default() -> Self {
        Self(1f32)
    }
}
impl std::ops::DerefMut for Scaling {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl std::ops::Deref for Scaling {
    type Target = f32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<f32> for Scaling {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

/// Transformation-related properties for animation.
///
/// `Translation`, `rotation`, and `scale` are optional field: while
/// interpolating, a `None` property is ignored.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Transform {
    pub translate: Option<Translation>,
    /// Internal rotation in radians
    pub rotate: Option<Rotation>,
    pub scale: Option<Scaling>,
}
impl Transform {
    pub fn translate(self, p: glam::Vec2) -> Self {
        Self {
            translate: Some(Translation(self.translate.unwrap_or_default().0 + p)),
            rotate: self.rotate,
            scale: self.scale,
        }
    }

    pub fn rotate(self, degrees: f32) -> Self {
        Self {
            translate: self.translate,
            rotate: Some(Rotation(
                self.rotate.unwrap_or_default().0 + degrees.to_radians(),
            )),
            scale: self.scale,
        }
    }

    pub fn scale(self, mult: f32) -> Self {
        Self {
            translate: self.translate,
            rotate: self.rotate,
            scale: Some(Scaling(self.scale.unwrap_or_default().0 * mult)),
        }
    }

    pub const fn has_translation(&self) -> bool {
        self.translate.is_some()
    }

    pub const fn has_rotation(&self) -> bool {
        self.rotate.is_some()
    }

    pub const fn has_scaling(&self) -> bool {
        self.scale.is_some()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Progress(pub f64);
impl Progress {
    pub const fn getd(&self) -> f64 {
        self.0
    }

    pub const fn getf(&self) -> f32 {
        self.0 as f32
    }

    pub const fn ended(&self) -> bool {
        self.0 >= 1f64
    }

    const fn tick(&mut self, d: f64) {
        self.0 = (self.0 + d).min(1.0)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub enum TransitionCurve {
    #[default]
    Linear,
    Quadratic,
    Cubic,
    Smoothstep,
    Custom(fn(f32) -> f32),
}
impl TransitionCurve {
    pub const fn custom(curve: fn(f32) -> f32) -> Self {
        Self::Custom(curve)
    }

    pub fn apply(&self, value: &mut f32) {
        match self {
            TransitionCurve::Linear => { /* no op */ }
            TransitionCurve::Quadratic => *value *= *value,
            TransitionCurve::Cubic => *value *= *value * *value,
            TransitionCurve::Smoothstep => todo!("smoothstep transition curve"),
            TransitionCurve::Custom(func) => *value = func(*value),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Verse {
    #[default]
    Forward,
    Reverse,
}
impl Verse {
    pub const fn invert(self) -> Self {
        match self {
            Self::Forward => Self::Reverse,
            Self::Reverse => Self::Forward,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Track {
    transform: Transform,
    progress: Progress,
    duration: Duration,
    curve: TransitionCurve,
    verse: Verse,
}
impl Track {
    pub const fn new(transform: Transform, duration: Duration, curve: TransitionCurve) -> Self {
        Self {
            transform,
            progress: Progress(0.0),
            duration,
            curve,
            verse: Verse::Forward,
        }
    }

    pub const fn new_linear(transform: Transform, duration: Duration) -> Self {
        Self::new(transform, duration, TransitionCurve::Linear)
    }

    pub const fn new_quadratic(transform: Transform, duration: Duration) -> Self {
        Self::new(transform, duration, TransitionCurve::Quadratic)
    }

    pub const fn invert(self) -> Self {
        Self {
            transform: self.transform,
            progress: self.progress,
            duration: self.duration,
            curve: self.curve,
            verse: self.verse.invert(),
        }
    }

    pub const fn tick(&mut self, delta: DeltaTime) {
        let dt = delta.as_f64();
        let seconds = self.duration.as_secs_f64();
        let dp = seconds / dt;
        self.progress.tick(dp);
    }

    pub fn update(&self, origin: &Transform) -> Transform {
        let target = &self.transform;
        let progress = {
            let mut progress_m = self.progress.getf();
            self.curve.apply(&mut progress_m);
            progress_m
        };

        let has_translation = origin.has_translation() || target.has_translation();
        let has_rotation = origin.has_rotation() || target.has_rotation();
        let has_scaling = origin.has_scaling() || target.has_scaling();

        let (origin, target) = match self.verse {
            Verse::Forward => (origin, target),
            Verse::Reverse => (target, origin),
        };

        let translate = if has_translation {
            let origin = origin.translate.unwrap_or_default();
            let target = target.translate.unwrap_or(origin);
            Some(Translation(origin.lerp(*target, progress)))
        } else {
            None
        };
        let rotate = if has_rotation {
            let origin = origin.rotate.unwrap_or_default();
            let target = target.rotate.unwrap_or(origin);
            Some(Rotation(origin.lerp(*target, progress)))
        } else {
            None
        };
        let scale = if has_scaling {
            let origin = origin.scale.unwrap_or_default();
            let target = target.scale.unwrap_or(origin);
            Some(Scaling(origin.lerp(*target, progress)))
        } else {
            None
        };

        Transform {
            translate,
            rotate,
            scale,
        }
    }

    pub const fn ended(&self) -> bool {
        self.progress.ended()
    }

    pub const fn target_transform(&self) -> &Transform {
        &self.transform
    }

    pub const fn progress(&self) -> &Progress {
        &self.progress
    }

    pub const fn duration(&self) -> &Duration {
        &self.duration
    }

    pub const fn curve(&self) -> &TransitionCurve {
        &self.curve
    }
}

ethel::table_spec! {
    struct Animations {
        tfm_transient: Transform;
        tfm_latest: Transform;

        // current active animation track
        // gets overriden by latest animation
        active_track: Track;

    }
}

#[derive(Debug, Default)]
pub struct AnimationManager {
    pub data: AnimationsRowTable,
}
impl AnimationManager {
    pub fn update(&mut self, delta: DeltaTime) {
        todo!()
    }

    pub const fn data(&self) -> &AnimationsRowTable {
        &self.data
    }

    pub const fn data_mut(&mut self) -> &mut AnimationsRowTable {
        &mut self.data
    }
}
