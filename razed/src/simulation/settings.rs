use gui::DropdownOptions;
use gui::UiEnv;

#[derive(Clone, Debug, Default)]
pub struct Settings {
    pub profiler: ProfilerSettings,
    pub display: DisplaySettings,
    pub render: RenderSettings,
    pub sim: SimSettings,
}

#[repr(u32)]
#[derive(DropdownOptions, Clone, Copy, Debug, Default)]
pub enum ShadingMode {
    BarycentricWeights = 0,
    TangentFrame = 1,
    UvScreenDerivatives = 2,
    VisibilityBuffer = 3,
    #[default]
    StandardPbr = 4,
}

gui::wrap_property!({
    const DBG_SHADING_MODE: ShadingMode = ShadingMode::StandardPbr;

    #[map(range = ..GfxGamma::MAX)]
    const GFX_GAMMA: f32 = GfxGamma::to_normalized(2.20);

    const DBG_SHADING_BIAS_ROUGH: f32 = 0.0;
    const DBG_SHADING_BIAS_METAL: f32 = 0.0;

    const SIM_RUNNING: bool = true;
    const SIM_SPEED: f32 = 1.0;

    const DISPLAY_VSYNC: bool = true;

    const DBG_DRAW_LATTICE: bool = false;
    const DBG_DRAW_CAGE: bool = false;
    const DBG_DRAW_GROUND: bool = true;

    #[map(range = ..DbgGroundScale::MAX)]
    const DBG_GROUND_SCALE: f32 = DbgGroundScale::to_normalized(64.0);
    #[map(range = ..DbgGroundUvScale::MAX)]
    const DBG_GROUND_UV_SCALE: f32 = DbgGroundUvScale::to_normalized(4.0);

    const PROFILER_RUNNING: bool = false;
    const PROFILER_COMMAND: i32 = 0;
});

#[derive(Clone, Debug, Default)]
pub struct DisplaySettings {
    pub vsync: DisplayVsync,
}
impl DisplaySettings {
    pub fn sync_fetch(&mut self, env: &UiEnv) {
        self.vsync.sync_fetch(env);
    }

    pub fn sync_push(&self, env: &mut UiEnv) {
        self.vsync.sync_push(env);
    }
}

#[derive(Clone, Debug, Default)]
pub struct SimSettings {
    pub running: SimRunning,
    pub speed: SimSpeed,
}
impl SimSettings {
    pub fn sync_fetch(&mut self, env: &UiEnv) {
        let mut a = self.running;
        let b = &mut self.running;
        a.0.set(false);
        b.set(true);
        self.running.sync_fetch(env);
        self.speed.sync_fetch(env);
    }

    pub fn sync_push(&self, env: &mut UiEnv) {
        self.running.sync_push(env);
        self.speed.sync_push(env);
    }
}

#[derive(Clone, Debug, Default)]
pub struct ProfilerSettings {
    pub running: ProfilerRunning,
    pub command: ProfilerCommand,
}
impl ProfilerSettings {
    pub fn sync_fetch(&mut self, env: &UiEnv) {
        self.running.sync_fetch(env);
        self.command.sync_fetch(env);
    }

    pub fn sync_push(&self, env: &mut UiEnv) {
        self.running.sync_push(env);
        self.command.sync_push(env);
    }
}

#[derive(Clone, Debug, Default)]
pub struct RenderSettings {
    pub shading_mode: DbgShadingMode,

    pub bias_roughness: DbgShadingBiasRough,
    pub bias_metallic: DbgShadingBiasMetal,

    pub gamma: GfxGamma,

    pub draw_lattice: DbgDrawLattice,
    pub draw_cage: DbgDrawCage,
    pub draw_ground: DbgDrawGround,

    pub ground_scale: DbgGroundScale,
    pub ground_uvscale: DbgGroundUvScale,
}
impl RenderSettings {
    pub fn sync_fetch(&mut self, env: &UiEnv) {
        self.shading_mode.sync_fetch(env);
        self.bias_roughness.sync_fetch(env);
        self.bias_metallic.sync_fetch(env);
        self.gamma.sync_fetch(env);
        self.draw_lattice.sync_fetch(env);
        self.draw_cage.sync_fetch(env);
        self.draw_ground.sync_fetch(env);
        self.ground_scale.sync_fetch(env);
        self.ground_uvscale.sync_fetch(env);
    }

    pub fn sync_push(&self, env: &mut UiEnv) {
        self.shading_mode.sync_push(env);
        self.bias_roughness.sync_push(env);
        self.bias_metallic.sync_push(env);
        self.gamma.sync_push(env);
        self.draw_lattice.sync_push(env);
        self.draw_cage.sync_push(env);
        self.draw_ground.sync_push(env);
        self.ground_scale.sync_push(env);
        self.ground_uvscale.sync_push(env);
    }
}

impl GfxGamma {
    pub const DEFAULT: f32 = 2.2;
    pub const MAX: f32 = 3.4;
    pub const INV_MAX: f32 = 1.0 / Self::MAX;
    pub const DEFAULT_NORMALIZED: f32 = Self::DEFAULT * Self::INV_MAX;

    /// Maps `value` from normalized [0; 1] range to the effective gamma range.
    pub const fn map(value: &mut f32) {
        *value = Self::from_normalized(*value);
    }

    pub fn to_normalized(value: f32) -> f32 {
        Self::normalize(Self::new_clamped(value))
    }

    pub const fn from_normalized(normalized: f32) -> f32 {
        if normalized < 0f32 {
            Self::DEFAULT
        } else {
            normalized * Self::MAX
        }
    }

    pub fn new_clamped(gamma: f32) -> f32 {
        gamma.clamp(0f32, Self::MAX)
    }

    pub const fn as_f32(&self) -> f32 {
        *self.0.get()
    }

    pub const fn normalize(v: f32) -> f32 {
        v * Self::INV_MAX
    }
}

impl DbgGroundScale {
    pub const MAX: f32 = 1024f32;
    pub const DEFAULT: f32 = 64f32;
    pub const DEFAULT_NORMALIZED: f32 = Self::DEFAULT / Self::MAX;
    const INV_MAX: f32 = 1f32 / Self::MAX;

    /// Maps `value` from normalized [0; 1] range to the effective ground scale range.
    pub const fn map(value: &mut f32) {
        *value = Self::from_normalized(*value);
    }

    pub fn to_normalized(value: f32) -> f32 {
        Self::normalize(value)
    }

    pub const fn from_normalized(normalized: f32) -> f32 {
        normalized * Self::MAX
    }
    pub const fn normalize(scale: f32) -> f32 {
        scale * Self::INV_MAX
    }
}

impl DbgGroundUvScale {
    pub const MAX: f32 = 64f32;
    pub const DEFAULT: f32 = 4f32;
    pub const DEFAULT_NORMALIZED: f32 = Self::DEFAULT / Self::MAX;
    const INV_MAX: f32 = 1f32 / Self::MAX;

    /// Maps `value` from normalized [0; 1] range to the effective ground uv-scale range.
    pub const fn map(value: &mut f32) {
        *value = Self::from_normalized(*value);
    }

    pub fn to_normalized(value: f32) -> f32 {
        Self::normalize(value)
    }

    pub const fn from_normalized(normalized: f32) -> f32 {
        normalized * Self::MAX
    }
    pub const fn normalize(uvscale: f32) -> f32 {
        uvscale * Self::INV_MAX
    }
}
