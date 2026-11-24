use janus::{StringHash, StringMap};

use crate::rgb;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LookId(StringHash);
impl LookId {
    pub const DEFAULT: Self = Self(janus::hash_string("gui::__look.default"));

    pub const fn inner(&self) -> StringHash {
        self.0
    }
}
impl Default for LookId {
    fn default() -> Self {
        Self::DEFAULT
    }
}
impl<T> From<T> for LookId
where
    T: Into<StringHash>,
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

#[derive(Debug)]
pub struct LooksManager {
    looks: StringMap<Look>,
}
impl Default for LooksManager {
    fn default() -> Self {
        let mut looks = StringMap::default();
        looks.insert(LookId::DEFAULT.0, Look::standard());
        Self { looks }
    }
}
impl LooksManager {
    pub fn create(&mut self, id: impl Into<StringHash>, look: Look) -> LookId {
        let id = id.into();
        self.looks.insert(id, look);
        LookId(id)
    }

    pub fn get(&self, id: LookId) -> &Look {
        self.looks.get(&id.0).expect("invalid look id")
    }

    pub fn get_mut(&mut self, id: LookId) -> &mut Look {
        self.looks.get_mut(&id.0).expect("invalid look id")
    }
}

#[derive(Debug)]
pub struct Look {
    pub panel_background: Rgb,
    pub panel_background_hover: Rgb,
    pub panel_opacity: f32,
    pub panel_opacity_hover: f32,
    pub panel_border_width: f32,
    pub panel_border: Rgb,
    pub panel_border_hover: Rgb,
    pub panel_border_press: Rgb,

    pub button_background: Rgb,
    pub button_background_hover: Rgb,
    pub button_background_press: Rgb,
    pub button_opacity: f32,
    pub button_opacity_hover: f32,
    pub button_opacity_press: f32,
    pub button_border_width: f32,
    pub button_border: Rgb,
    pub button_border_hover: Rgb,
    pub button_border_press: Rgb,

    pub text_color: Rgb,
    pub text_scaling: f32,

    pub checkbox_border: Rgb,
    pub checkbox_border_width: f32,
    pub checkbox_background: Rgb,
    pub checkbox_background_hover: Rgb,
    pub checkbox_check: Rgb,
    pub checkbox_scaling: f32,

    pub slider_knob: Rgb,
    pub slider_knob_hover: Rgb,
    pub slider_knob_press: Rgb,
    pub slider_knob_border_width: f32,
    pub slider_knob_border_hover: Rgb,
    pub slider_knob_border_press: Rgb,
    pub slider_track: Rgb,

    pub menu_background: Rgb,
    pub menu_background_opacity: f32,
    pub menu_option_hover: Rgb,
    pub menu_option_press: Rgb,
    pub menu_option_select: Rgb,
}
impl Default for Look {
    fn default() -> Self {
        Self::standard()
    }
}
impl Look {
    pub const fn standard() -> Self {
        Self {
            panel_background: rgb!(0x0u32),
            panel_background_hover: rgb!(uniform 76),
            panel_opacity: 0.4,
            panel_opacity_hover: 0.6,
            panel_border_width: 0.0,
            panel_border: rgb!(0x0u32),
            panel_border_hover: rgb!(0x0u32),
            panel_border_press: rgb!(0x0u32),

            button_background: rgb!(0x0u32),
            button_background_hover: rgb!(uniform 76),
            button_background_press: rgb!(uniform 114),
            button_opacity: 0.4,
            button_opacity_hover: 0.6,
            button_opacity_press: 0.6,
            button_border_width: 0.0,
            button_border: rgb!(0x0u32),
            button_border_hover: rgb!(0x0u32),
            button_border_press: rgb!(0x0u32),

            text_color: rgb!(0xffffff),
            text_scaling: 1.0,

            checkbox_border: rgb!(0x0),
            checkbox_border_width: 2.0,
            checkbox_background: rgb!(0xaaaaaa),
            checkbox_background_hover: rgb!(0x80808a),
            checkbox_check: rgb!(0x32dc24),
            checkbox_scaling: 1.0,

            slider_knob: rgb!(0x0u32),
            slider_knob_hover: rgb!(uniform 76),
            slider_knob_press: rgb!(uniform 114),
            slider_knob_border_width: 0.0,
            slider_knob_border_hover: rgb!(0x0u32),
            slider_knob_border_press: rgb!(0x0u32),
            slider_track: rgb!(uniform 165),

            menu_background: rgb!(0x676767),
            menu_background_opacity: 0.8,
            menu_option_hover: rgb!(0x8a8a8a),
            menu_option_press: rgb!(0x242424),
            menu_option_select: rgb!(0x82abff),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColorComponent<T: Clone + Copy>(T);
impl<T: Clone + Copy> ColorComponent<T> {
    pub const fn new(value: T) -> Self {
        Self(value)
    }

    pub const fn set(&mut self, value: T) {
        self.0 = value;
    }

    pub const fn get(&self) -> T {
        self.0
    }
}
impl ColorComponent<u8> {
    pub const fn from_f32(value: f32) -> Self {
        Self((value * 255f32) as u8)
    }

    pub const fn from_u32(value: u32) -> Self {
        Self((value * 255u32) as u8)
    }

    pub const fn from_i32(value: i32) -> Self {
        Self((value * 255i32) as u8)
    }
}
impl<T: Clone + Copy> std::ops::Deref for ColorComponent<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T: Clone + Copy> std::ops::DerefMut for ColorComponent<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl From<ColorComponent<f32>> for ColorComponent<u8> {
    fn from(value: ColorComponent<f32>) -> Self {
        Self::from_f32(value.get())
    }
}
impl From<ColorComponent<u32>> for ColorComponent<u8> {
    fn from(value: ColorComponent<u32>) -> Self {
        Self::from_u32(value.get())
    }
}
impl From<ColorComponent<i32>> for ColorComponent<u8> {
    fn from(value: ColorComponent<i32>) -> Self {
        Self::from_i32(value.get())
    }
}
impl From<ColorComponent<u8>> for ColorComponent<f32> {
    fn from(value: ColorComponent<u8>) -> Self {
        Self(value.get() as f32 / 255f32)
    }
}
impl From<ColorComponent<u8>> for ColorComponent<u32> {
    fn from(value: ColorComponent<u8>) -> Self {
        Self(value.get() as u32 / 255u32)
    }
}
impl From<ColorComponent<u8>> for ColorComponent<i32> {
    fn from(value: ColorComponent<u8>) -> Self {
        Self(value.get() as i32 / 255i32)
    }
}
impl<T: Clone + Copy> From<u8> for ColorComponent<T>
where
    ColorComponent<u8>: Into<ColorComponent<T>>,
{
    fn from(value: u8) -> Self {
        ColorComponent(value).into()
    }
}
impl<T: Clone + Copy> From<f32> for ColorComponent<T>
where
    ColorComponent<f32>: Into<ColorComponent<T>>,
{
    fn from(value: f32) -> Self {
        ColorComponent(value).into()
    }
}
impl<T: Clone + Copy> From<u32> for ColorComponent<T>
where
    ColorComponent<u32>: Into<ColorComponent<T>>,
{
    fn from(value: u32) -> Self {
        ColorComponent(value).into()
    }
}
impl<T: Clone + Copy> From<i32> for ColorComponent<T>
where
    ColorComponent<i32>: Into<ColorComponent<T>>,
{
    fn from(value: i32) -> Self {
        ColorComponent(value).into()
    }
}
impl<T: Clone + Copy> Into<f32> for ColorComponent<T>
where
    ColorComponent<T>: Into<ColorComponent<f32>>,
{
    fn into(self) -> f32 {
        self.into().get()
    }
}
impl<T: Clone + Copy> Into<u32> for ColorComponent<T>
where
    ColorComponent<T>: Into<ColorComponent<u32>>,
{
    fn into(self) -> u32 {
        self.into().get()
    }
}
impl<T: Clone + Copy> Into<i32> for ColorComponent<T>
where
    ColorComponent<T>: Into<ColorComponent<i32>>,
{
    fn into(self) -> i32 {
        self.into().get()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rgb {
    r: ColorComponent<u8>,
    g: ColorComponent<u8>,
    b: ColorComponent<u8>,
}
impl Rgb {
    pub fn new(
        r: impl Into<ColorComponent<u8>>,
        g: impl Into<ColorComponent<u8>>,
        b: impl Into<ColorComponent<u8>>,
    ) -> Self {
        Self {
            r: r.into(),
            g: g.into(),
            b: b.into(),
        }
    }

    pub const fn new_const(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: ColorComponent(r),
            g: ColorComponent(g),
            b: ColorComponent(b),
        }
    }

    pub fn component<T: Clone + Copy>(&self, index: usize) -> impl Into<T>
    where
        T: From<ColorComponent<u8>>,
    {
        *self.component_ref(index)
    }

    pub fn red<T: Clone + Copy>(&self) -> impl Into<T>
    where
        T: From<ColorComponent<u8>>,
    {
        *self.red_ref()
    }

    pub fn green<T: Clone + Copy>(&self) -> impl Into<T>
    where
        T: From<ColorComponent<u8>>,
    {
        *self.green_ref()
    }

    pub fn blue<T: Clone + Copy>(&self) -> impl Into<T>
    where
        T: From<ColorComponent<u8>>,
    {
        *self.blue_ref()
    }

    pub const fn component_ref(&self, index: usize) -> &ColorComponent<u8> {
        match index % 3 {
            0 => self.red_ref(),
            1 => self.green_ref(),
            2 => self.blue_ref(),
            _ => unreachable!(),
        }
    }

    pub const fn red_ref(&self) -> &ColorComponent<u8> {
        &self.r
    }

    pub const fn green_ref(&self) -> &ColorComponent<u8> {
        &self.g
    }

    pub const fn blue_ref(&self) -> &ColorComponent<u8> {
        &self.b
    }

    pub const fn component_mut(&mut self, index: usize) -> &mut ColorComponent<u8> {
        match index % 3 {
            0 => self.red_mut(),
            1 => self.green_mut(),
            2 => self.blue_mut(),
            _ => unreachable!(),
        }
    }

    pub const fn red_mut(&mut self) -> &mut ColorComponent<u8> {
        &mut self.r
    }

    pub const fn green_mut(&mut self) -> &mut ColorComponent<u8> {
        &mut self.g
    }

    pub const fn blue_mut(&mut self) -> &mut ColorComponent<u8> {
        &mut self.b
    }
}
impl std::ops::Index<usize> for Rgb {
    type Output = ColorComponent<u8>;

    fn index(&self, index: usize) -> &Self::Output {
        self.component_ref(index)
    }
}
impl std::ops::IndexMut<usize> for Rgb {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.component_mut(index)
    }
}
impl<T: Clone + Copy> From<[T; 3]> for Rgb
where
    T: Into<ColorComponent<u8>>,
{
    fn from(value: [T; 3]) -> Self {
        Self::new(value[0], value[1], value[2])
    }
}
impl From<Rgb> for glam::Vec3 {
    fn from(rgb: Rgb) -> Self {
        Self::new(
            *rgb.red::<ColorComponent<f32>>().into(),
            *rgb.green::<ColorComponent<f32>>().into(),
            *rgb.blue::<ColorComponent<f32>>().into(),
        )
    }
}
impl From<Rgb> for glam::Vec4 {
    fn from(rgb: Rgb) -> Self {
        Self::new(
            *rgb.red::<ColorComponent<f32>>().into(),
            *rgb.green::<ColorComponent<f32>>().into(),
            *rgb.blue::<ColorComponent<f32>>().into(),
            1.0,
        )
    }
}

#[macro_export]
macro_rules! rgb {
    ($r:literal $g:literal $b:literal) => {
        $crate::look::Rgb::new_const($r, $g, $b)
    };
    ($r:expr, $g:expr, $b:expr) => {
        $crate::look::Rgb::new($r, $g, $b)
    };
    (uniform $u:literal) => { $crate::rgb!($u $u $u) };
    (uniform $u:expr) => { $crate::rgb!($u, $u, $u) };
    ($u:literal) => {
        $crate::look::Rgb::new_const(($u & 0xff) as u8, (($u >> 8) & 0xff) as u8, (($u >> 16) & 0xff) as u8)
    };
    ($u:expr) => {
        $crate::look::Rgb::new(($u & 0xff ) as u8, (($u >> 8) & 0xff) as u8, (($u >> 16) & 0xff) as u8)
    };
}
