pub mod debris;
pub mod fragments;
pub mod plane;

pub use debris::*;
pub use fragments::*;
pub use plane::*;

rendrs::geometry_buffers! {
    vertices = 4_194_240;
    triangles = 4_194_240;
}

pub const GBANK_VERTEX_CAP: u32 = 4_194_240;
pub const GBANK_TRIANGLE_CAP: u32 = 4_194_240;
