use gui::{
    declare::{UiPanel, UiText},
    text_contents,
};

use crate::simulation::settings::{PROP_ID_SIM_RUNNING, PROP_ID_SIM_SPEED};

pub use super::*;

pub fn root(system: &mut InterfaceSystem, root: WidgetId, resolution: Resolution) -> WidgetId {
    let panel = UiPanel::new(resolution)
        .with_parent(root)
        .with_offset((8.0, 8.0))
        .with_size((840.0, 420.0))
        .with_padding(10.0)
        .apply(system);

    let mut debug_text = |contents: TextContents| {
        UiText::new(resolution)
            .with_parent(panel)
            .builder(|_, props| props.with_contents(contents))
            .apply(system)
    };

    debug_text(text_contents!("::FPS = ", DEBUG_PERF_FPS_AVG));
    debug_text(text_contents!("::TPS = ", DEBUG_PERF_TPS_TOTAL));
    debug_text(text_contents!(
        "Frame::Duration.Simulation = ",
        DEBUG_PERF_LAST_SIMUL_FRAME_TIME_MILLIS,
        "ms"
    ));
    debug_text(text_contents!(
        "Frame::Duration.Render = ",
        DEBUG_PERF_LAST_RENDER_FRAME_TIME_MILLIS,
        "ms"
    ));

    debug_text(text_contents!(
        "World::Lattice.Nodes = ",
        DEBUG_COUNTER_LATTICE_NODES,
    ));
    debug_text(text_contents!(
        "World::Lattice.Constraints = ",
        DEBUG_COUNTER_LATTICE_CONSTRAINTS,
    ));
    debug_text(text_contents!(
        "World::Fragments = ",
        DEBUG_COUNTER_FRAGMENTS,
    ));
    debug_text(text_contents!("World::Cages = ", DEBUG_COUNTER_CAGES,));
    debug_text(text_contents!("World::Debris = ", DEBUG_COUNTER_DEBRIS,));

    debug_text(text_contents!("Sim::Active = ", PROP_ID_SIM_RUNNING,));
    debug_text(text_contents!("Sim::Speed = ", PROP_ID_SIM_SPEED,));

    debug_text(text_contents!(
        "GeomBank::VertexUse = ",
        DEBUG_RENDER_GBANK_VUSE_PERC,
    ));
    debug_text(text_contents!(
        "GeomBank::TrisUse = ",
        DEBUG_RENDER_GBANK_TUSE_PERC,
    ));
    debug_text(text_contents!(
        "GeomBank::TrisCount/w-cull = ",
        DEBUG_RENDER_GBANK_TRIS_COUNT,
    ));
    debug_text(text_contents!(
        "GeomBank::MemoryUse = ",
        |DEBUG_RENDER_GBANK_MEMPRINT: var| {
            EnvValue::Float(var.as_integer().unwrap_or_default() as f32 / 1024.0)
        },
        " KB"
    ));
    debug_text(text_contents!(
        "GeomBank::Instances = ",
        DEBUG_RENDER_GBANK_INSTANCE_COUNT
    ));
    debug_text(text_contents!(
        "GeomBank::InstanceLists = ",
        DEBUG_RENDER_GBANK_INSTANCELIST_COUNT
    ));

    panel
}
