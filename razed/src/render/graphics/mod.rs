pub mod envmap;
pub mod materials;

pub use envmap::*;
pub use materials::*;

use crate::simulation::settings::{
    DbgGroundScale, DbgGroundUvScale, GfxGamma, RenderSettings, ShadingMode,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct RenderStats {
    pub gbank_vuse_perc: f32,
    pub gbank_tuse_perc: f32,
    pub gbank_mprint: usize,
    pub tris_count: u32,
    pub instance_count: u32,
    pub instancelist_count: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct RenderOptions {
    pub shading_mode: ShadingMode,

    pub bias_roughness: f32,
    pub bias_metallic: f32,

    pub gamma: f32,

    pub draw_lattice: bool,
    pub draw_cage: bool,
    pub draw_ground: bool,

    pub ground_scale: f32,
    pub ground_uvscale: f32,
}
impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            shading_mode: ShadingMode::StandardPbr,
            bias_roughness: 0.0,
            bias_metallic: 0.0,
            gamma: GfxGamma::DEFAULT,
            draw_lattice: false,
            draw_cage: false,
            draw_ground: true,
            ground_scale: DbgGroundScale::DEFAULT,
            ground_uvscale: DbgGroundUvScale::DEFAULT,
        }
    }
}
impl From<&RenderSettings> for RenderOptions {
    fn from(value: &RenderSettings) -> Self {
        Self {
            shading_mode: *value.shading_mode.get(),
            bias_roughness: *value.bias_roughness.get(),
            bias_metallic: *value.bias_metallic.get(),
            gamma: *value.gamma.get(),
            draw_lattice: *value.draw_lattice.get(),
            draw_cage: *value.draw_cage.get(),
            draw_ground: *value.draw_ground.get(),
            ground_scale: *value.ground_scale.get(),
            ground_uvscale: *value.ground_uvscale.get(),
        }
    }
}
