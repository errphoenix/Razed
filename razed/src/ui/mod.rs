use ethel::Resolution;
use gui::{
    FloatGrabArea, InterfaceSystem, Rectangle, TextContents, WidgetId, declare::UiPanel,
    env::EnvValue, style_layout,
};

use env_names::*;

#[allow(unused_imports)]
use crate::{render::graphics, ui::env_names::*};

pub mod ctlpanel;
pub mod infopanel;

pub const DEV_PANEL_PADDING: Rectangle = style_layout!(6.0, 26.0, 6.0, 6.0);

pub const COLORTINT_HOVER_INVARIANT: glam::Vec4 = glam::vec4(0f32, 0f32, 0f32, 1f32);

pub fn initialize_default(resolution: Resolution) -> InterfaceSystem {
    let mut system = InterfaceSystem::new(resolution);
    let root = UiPanel::new(resolution).with_size(0.0).apply(&mut system);

    infopanel::root(&mut system, root, resolution);
    ctlpanel::root(&mut system, root, resolution);
    ctlpanel::groundctl::root(&mut system, root, resolution);
    ctlpanel::profile::root(&mut system, root, resolution);

    system
}

pub mod env_names {
    use janus::StringHash;

    pub const DEBUG_PERF_LAST_SIMUL_FRAME_TIME_MILLIS: StringHash =
        janus::hash_string("__debug.perf.last_simul_frame_time.millis");
    pub const DEBUG_PERF_LAST_RENDER_FRAME_TIME_MILLIS: StringHash =
        janus::hash_string("__debug.perf.last_render_frame_time.millis");
    pub const DEBUG_PERF_FPS_AVG: StringHash = janus::hash_string("__debug.perf.fps.avg");
    pub const DEBUG_PERF_TPS_TOTAL: StringHash = janus::hash_string("__debug.perf.tps.total");

    pub const DEBUG_COUNTER_LATTICE_NODES: StringHash =
        janus::hash_string("__debug.counter.lattice.nodes");
    pub const DEBUG_COUNTER_LATTICE_CONSTRAINTS: StringHash =
        janus::hash_string("__debug.counter.lattice.constraints");
    pub const DEBUG_COUNTER_FRAGMENTS: StringHash = janus::hash_string("__debug.counter.fragments");
    pub const DEBUG_COUNTER_CAGES: StringHash = janus::hash_string("__debug.counter.cages");
    pub const DEBUG_COUNTER_DEBRIS: StringHash = janus::hash_string("__debug.counter.debris");

    pub const DEBUG_RENDER_GBANK_VUSE_PERC: StringHash =
        janus::hash_string("__debug.render.gbank.vuse_perc");
    pub const DEBUG_RENDER_GBANK_TUSE_PERC: StringHash =
        janus::hash_string("__debug.render.gbank.tuse_perc");
    pub const DEBUG_RENDER_GBANK_TRIS_COUNT: StringHash =
        janus::hash_string("__debug.render.gbank.tris_count");
    pub const DEBUG_RENDER_GBANK_MEMPRINT: StringHash =
        janus::hash_string("__debug.render.gbank.mem_footprint");
    pub const DEBUG_RENDER_GBANK_INSTANCE_COUNT: StringHash =
        janus::hash_string("__debug.render.gbank.instance_count");
    pub const DEBUG_RENDER_GBANK_INSTANCELIST_COUNT: StringHash =
        janus::hash_string("__debug.render.gbank.instancelist_count");

    pub const PROFILER_FRAME_COUNT: StringHash = janus::hash_string("profiler.frame_count");
}
