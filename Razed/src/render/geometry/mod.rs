pub mod debris;
pub mod fragments;
pub mod plane;

pub use debris::*;
pub use fragments::*;
pub use plane::*;

rendrs::geometry_buffers! {
    vertices = 1_048_560;
    triangles = 1_048_560;
}
