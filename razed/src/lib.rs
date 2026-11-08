pub mod assets;
pub mod data;
pub mod input;
pub mod procedural;
pub mod render;
pub mod simulation;
pub mod structure;
pub mod ui;

pub const PROFILER_ACTION_PAUSE: u8 = 0b00000001;
pub const PROFILER_ACTION_START: u8 = 0b00000010;
pub const PROFILER_ACTION_DUMP: u8 = 0b00000100;
