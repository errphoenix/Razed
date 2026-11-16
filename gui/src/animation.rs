use ethel::{Column, DirectIndex, IndirectIndex};
use janus::DeltaTime;

#[derive(Clone, Copy, Debug, Default)]
pub enum TransitionCurve {
    #[default]
    Linear,

    QuadraticIn,
    QuadraticOut,
    QuadraticInOut,

    CubicIn,
    CubicOut,
    CubicInOut,

    Custom(fn(f64) -> f64),
}
impl TransitionCurve {
    pub const fn custom(curve: fn(f64) -> f64) -> Self {
        Self::Custom(curve)
    }

    pub fn apply(&self, value: &mut f64) {
        match self {
            TransitionCurve::Linear => { /* no op */ }
            TransitionCurve::QuadraticIn => *value *= *value,
            TransitionCurve::QuadraticOut => {
                let v = 1.0 - *value;
                let s = v * v;
                *value = 1.0 - s;
            }
            TransitionCurve::QuadraticInOut => {
                if *value < 0.5 {
                    *value = 2.0 * *value * *value;
                } else {
                    let v = -2.0 * *value + 2.0;
                    *value = 1.0 - (v * v) / 2.0;
                }
            }
            TransitionCurve::CubicIn => *value *= *value * *value,
            TransitionCurve::CubicOut => {
                let v = 1.0 - *value;
                let s = v * v * v;
                *value = 1.0 - s;
            }
            TransitionCurve::CubicInOut => {
                if *value < 0.5 {
                    *value = 4.0 * *value * *value * *value;
                } else {
                    let v = -2.0 * *value + 2.0;
                    *value = 1.0 - (v * v * v) / 2.0;
                }
            }
            TransitionCurve::Custom(f) => *value = f(*value),
        }
    }
}

pub trait Lerp: Clone + Copy {
    fn lerp(&self, other: Self, t: f64) -> Self;
}
impl Lerp for f32 {
    fn lerp(&self, other: Self, t: f64) -> Self {
        *self + ((other - *self) * t as f32)
    }
}
impl Lerp for f64 {
    fn lerp(&self, other: Self, t: f64) -> Self {
        *self + ((other - *self) * t)
    }
}
impl Lerp for glam::Vec2 {
    fn lerp(&self, other: Self, t: f64) -> Self {
        glam::vec2(self.x.lerp(other.x, t), self.y.lerp(other.y, t))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AnimationPropTargets(u8);
impl AnimationPropTargets {
    const SHIFT_OFFSET: u8 = 0;
    const SHIFT_ROTATION: u8 = 1;
    const SHIFT_SCALE: u8 = 2;

    const MASK_OFFSET: u8 = 1 << Self::SHIFT_OFFSET;
    const MASK_ROTATION: u8 = 1 << Self::SHIFT_ROTATION;
    const MASK_SCALE: u8 = 1 << Self::SHIFT_SCALE;

    pub const fn new(offset: bool, rotation: bool, scale: bool) -> Self {
        Self(
            ((offset as u8) << Self::SHIFT_OFFSET)
                | ((rotation as u8) << Self::SHIFT_ROTATION)
                | ((scale as u8) << Self::SHIFT_SCALE),
        )
    }

    pub const fn flip_offset(&mut self) {
        self.0 ^= Self::MASK_OFFSET
    }
    pub const fn flip_rotation(&mut self) {
        self.0 ^= Self::MASK_ROTATION
    }
    pub const fn flip_scale(&mut self) {
        self.0 ^= Self::MASK_SCALE
    }

    pub const fn has_offset(&self) -> bool {
        (self.0 & Self::MASK_OFFSET) != 0
    }
    pub const fn has_rotation(&self) -> bool {
        (self.0 & Self::MASK_ROTATION) != 0
    }
    pub const fn has_scale(&self) -> bool {
        (self.0 & Self::MASK_SCALE) != 0
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AnimationProp<T: Lerp> {
    current: T,
    target: T,
    data: Option<Tween>,
}
impl<T: Lerp> AnimationProp<T> {
    pub const fn new(current: T) -> Self {
        Self {
            current,
            target: current,
            data: None,
        }
    }

    pub const fn with_transition(start: T, end: T, data: Tween) -> Self {
        Self {
            current: start,
            target: end,
            data: Some(data),
        }
    }

    pub const fn current_value(&self) -> T {
        self.current
    }

    pub const fn target_value(&self) -> T {
        self.target
    }

    pub const fn data(&self) -> Option<&Tween> {
        self.data.as_ref()
    }

    pub const fn active(&self) -> bool {
        self.data.is_some()
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Tween {
    /// in seconds
    elapsed: f64,
    /// in seconds
    duration: f64,
    curve: TransitionCurve,
}
impl Tween {
    pub const fn new(duration_seconds: f64, curve: TransitionCurve) -> Self {
        Self {
            curve,
            elapsed: 0.0,
            duration: duration_seconds,
        }
    }

    pub fn evaluate<T: Lerp>(&self, a: T, b: T) -> T {
        let mut t = self.duration / self.elapsed;
        self.curve.apply(&mut t);
        a.lerp(b, t)
    }

    pub const fn update(&mut self, delta: DeltaTime) {
        let dt = delta.as_f64();
        self.elapsed = (self.elapsed + dt).min(self.duration);
    }

    pub const fn finished(&self) -> bool {
        self.elapsed >= self.duration
    }

    pub const fn elapsed(&self) -> f64 {
        self.elapsed
    }

    pub const fn duration(&self) -> f64 {
        self.elapsed
    }
}

pub type Offset = glam::Vec2;
pub type Rotation = f32;
pub type Scale = f32;

ethel::table_spec! {
    struct AnimationData {
        offsets   : AnimationProp<Offset>;
        rotations : AnimationProp<Rotation>;
        scales    : AnimationProp<Scale>;
    }
}

#[derive(Debug)]
pub struct AnimationManager {
    data: AnimationDataRowTable,
}
impl AnimationManager {
    pub fn delete_handle(&mut self, handle: IndirectIndex) {
        self.data.free(handle);
    }

    pub fn create_handle(&mut self) -> IndirectIndex {
        let dummy = (Default::default(), Default::default(), Default::default());
        self.data.insert(dummy)
    }

    pub fn set_offset(
        &mut self,
        handle: IndirectIndex,
        offset: Offset,
        duration_seconds: f64,
        curve: TransitionCurve,
    ) {
        let index = self
            .data
            .solve_indirect(handle)
            .expect("invalid animation handle")
            .as_index();
        let prop = &mut self.data.offsets[index];
        prop.target = offset + prop.current;
        prop.data = Some(Tween::new(duration_seconds, curve));
    }

    pub fn set_rotation(
        &mut self,
        handle: IndirectIndex,
        rotation: Rotation,
        duration_seconds: f64,
        curve: TransitionCurve,
    ) {
        let index = self
            .data
            .solve_indirect(handle)
            .expect("invalid animation handle")
            .as_index();
        let prop = &mut self.data.rotations[index];
        prop.target = rotation + prop.current;
        prop.data = Some(Tween::new(duration_seconds, curve));
    }

    pub fn set_scale(
        &mut self,
        handle: IndirectIndex,
        scale: Scale,
        duration_seconds: f64,
        curve: TransitionCurve,
    ) {
        let index = self
            .data
            .solve_indirect(handle)
            .expect("invalid animation handle")
            .as_index();
        let prop = &mut self.data.scales[index];
        prop.target = scale + prop.current;
        prop.data = Some(Tween::new(duration_seconds, curve));
    }

    fn update_array<T: Lerp>(array: &mut [AnimationProp<T>], delta: DeltaTime) {
        for prop in array {
            let mut finish = false;
            if let Some(tween) = &mut prop.data {
                tween.update(delta);
                let (a, b) = (prop.current, prop.target);
                prop.current = tween.evaluate(a, b);
                finish = tween.finished();
            }
            if finish {
                prop.data = None;
            }
        }
    }

    pub fn update(&mut self, delta: DeltaTime) {
        Self::update_array(&mut self.data.offsets[1..], delta);
        Self::update_array(&mut self.data.rotations[1..], delta);
        Self::update_array(&mut self.data.scales[1..], delta);
    }

    pub fn solve_indirect(&self, id: IndirectIndex) -> Option<DirectIndex> {
        self.data.solve_indirect(id)
    }

    pub fn offset(&self, did: DirectIndex) -> Option<Offset> {
        self.data
            .offsets
            .get(did.as_index())
            .map(AnimationProp::current_value)
    }

    pub fn rotation(&self, did: DirectIndex) -> Option<Rotation> {
        self.data
            .rotations
            .get(did.as_index())
            .map(AnimationProp::current_value)
    }

    pub fn scale(&self, did: DirectIndex) -> Option<Scale> {
        self.data
            .scales
            .get(did.as_index())
            .map(AnimationProp::current_value)
    }

    pub fn data(&self) -> &AnimationDataRowTable {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut AnimationDataRowTable {
        &mut self.data
    }
}
