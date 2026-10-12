use ethel::data::hash::Cell;
use physics::xpbd::{RawXpbdLattice, XpbdLatticeBuilder, XpbdLinkOptions, XpbdNodeOptions as Node};

use crate::procedural::voxel::VoxelFn;

#[derive(Clone, Copy, PartialEq, PartialOrd, Default, Debug)]
pub struct CuboidVoxelFn {
    pub width: f32,
    pub height: f32,
    pub depth: f32,
    pub levels: u32,
}
impl VoxelFn for CuboidVoxelFn {
    fn pass(&self, cell: Cell) -> bool {
        let half_cell = Cell {
            x: (self.width * 0.5) as i32,
            y: (self.levels as f32 * self.height * 0.5) as i32,
            z: (self.depth * 0.5) as i32,
        };
        let cell = cell + half_cell;

        let inner_space = (self.depth.min(self.width) * 0.25) as i32;
        cell.x < inner_space
            || cell.x > (self.width as i32) - inner_space - 1
            || cell.z < inner_space
            || cell.z > (self.depth as i32) - inner_space - 1
    }
}

pub fn lattice(
    origin: glam::Vec3,
    width: f32,
    height: f32,
    depth: f32,
    floors: u32,
) -> RawXpbdLattice {
    debug_assert!(floors > 0, "cannot create a structure with 0 floors");

    const FLOOR_NODE_COUNT: usize = 8;
    // include 4 anchor nodes of the building
    let total_node_count = FLOOR_NODE_COUNT * floors as usize + 4;

    const MASS: f32 = 100.0;

    const VERY_STIFF_COMPL: f32 = 0.1e-8;
    const STIFF_COMPL: f32 = 0.15e-7;
    const SOFT_COMPL: f32 = 0.2e-6;

    const STRONG_LINK: XpbdLinkOptions = XpbdLinkOptions::new(VERY_STIFF_COMPL);
    const MID_LINK: XpbdLinkOptions = XpbdLinkOptions::new(STIFF_COMPL);
    const WEAK_LINK: XpbdLinkOptions = XpbdLinkOptions::new(SOFT_COMPL);

    let mut lattice = XpbdLatticeBuilder::with_capacity(total_node_count);
    let w = width / 2.0;
    let d = depth / 2.0;
    let o = origin;

    // anchor nodes
    let bottom_l_b = lattice.node(Node::new(o + glam::vec3(-w, 0.0, -d), MASS).with_fixed(true));
    let bottom_r_b = lattice.node(Node::new(o + glam::vec3(w, 0.0, -d), MASS).with_fixed(true));
    let bottom_r_f = lattice.node(Node::new(o + glam::vec3(w, 0.0, d), MASS).with_fixed(true));
    let bottom_l_f = lattice.node(Node::new(o + glam::vec3(-w, 0.0, d), MASS).with_fixed(true));

    // base loop
    {
        lattice.link_nodes(bottom_l_b, bottom_r_b, STRONG_LINK);
        lattice.link_nodes(bottom_r_b, bottom_r_f, STRONG_LINK);
        lattice.link_nodes(bottom_r_f, bottom_l_f, STRONG_LINK);
        lattice.link_nodes(bottom_l_f, bottom_l_b, STRONG_LINK);
    }

    // back_left, back_right, front_right, front_left
    // 0-------1
    // |       |
    // |       |
    // 3-------2
    let mut last_top = [bottom_l_b, bottom_r_b, bottom_r_f, bottom_l_f];

    for i in 0..floors {
        let ceiling_y = height * (i + 1) as f32;

        let back_left = lattice.node(Node::new(o + glam::vec3(-w, ceiling_y, -d), MASS));
        let back_right = lattice.node(Node::new(o + glam::vec3(w, ceiling_y, -d), MASS));
        let front_right = lattice.node(Node::new(o + glam::vec3(w, ceiling_y, d), MASS));
        let front_left = lattice.node(Node::new(o + glam::vec3(-w, ceiling_y, d), MASS));

        let half_y = ceiling_y - (height * 0.5);
        let middle_back = lattice.node(Node::new(o + glam::vec3(0.0, half_y, -d), MASS));
        let middle_front = lattice.node(Node::new(o + glam::vec3(0.0, half_y, d), MASS));
        let middle_right = lattice.node(Node::new(o + glam::vec3(w, half_y, 0.0), MASS));
        let middle_left = lattice.node(Node::new(o + glam::vec3(-w, half_y, 0.0), MASS));

        // top loop
        {
            lattice.link_nodes(back_left, back_right, MID_LINK);
            lattice.link_nodes(back_right, front_right, MID_LINK);
            lattice.link_nodes(front_right, front_left, MID_LINK);
            lattice.link_nodes(front_left, back_left, MID_LINK);
        }
        // pillars
        {
            let prev_back_left = 0;
            let prev_back_right = 1;
            let prev_front_right = 2;
            let prev_front_left = 3;

            lattice.link_nodes(back_left, last_top[prev_back_left], STRONG_LINK);
            lattice.link_nodes(back_right, last_top[prev_back_right], STRONG_LINK);
            lattice.link_nodes(front_right, last_top[prev_front_right], STRONG_LINK);
            lattice.link_nodes(front_left, last_top[prev_front_left], STRONG_LINK);
        }
        // side diagonals
        {
            let prev_back_left = 0;
            let prev_back_right = 1;
            let prev_front_right = 2;
            let prev_front_left = 3;

            lattice.link_nodes(back_left, middle_back, MID_LINK);
            lattice.link_nodes(back_right, middle_back, MID_LINK);
            lattice.link_nodes(middle_back, last_top[prev_back_right], MID_LINK);
            lattice.link_nodes(middle_back, last_top[prev_back_left], MID_LINK);

            lattice.link_nodes(front_right, middle_right, MID_LINK);
            lattice.link_nodes(back_right, middle_right, MID_LINK);
            lattice.link_nodes(middle_right, last_top[prev_back_right], MID_LINK);
            lattice.link_nodes(middle_right, last_top[prev_front_right], MID_LINK);

            lattice.link_nodes(front_left, middle_front, MID_LINK);
            lattice.link_nodes(front_right, middle_front, MID_LINK);
            lattice.link_nodes(middle_front, last_top[prev_front_right], MID_LINK);
            lattice.link_nodes(middle_front, last_top[prev_front_left], MID_LINK);

            lattice.link_nodes(back_left, middle_left, MID_LINK);
            lattice.link_nodes(front_left, middle_left, MID_LINK);
            lattice.link_nodes(middle_left, last_top[prev_front_left], MID_LINK);
            lattice.link_nodes(middle_left, last_top[prev_back_left], MID_LINK);
        }
        // floor diagonal
        {
            lattice.link_nodes(back_left, front_right, WEAK_LINK);
            lattice.link_nodes(front_left, back_right, WEAK_LINK);
        }

        last_top = [back_left, back_right, front_right, front_left];
    }

    lattice.build()
}
