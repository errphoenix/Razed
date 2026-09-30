pub mod debris;
pub mod fragments;

pub use debris::*;
pub use fragments::*;

rendrs::geometry_buffers! {
    vertices = 1_048_560;
    triangles = 1_048_560;
}
