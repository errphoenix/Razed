pub mod fragment;

use physics::xpbd::{XpbdLatticeBuilder, XpbdLinkOptions, XpbdNodeOptions as Node};

#[allow(unused_imports)]
pub use fragment::{FragmentState, FragmentSystem};

// height is per floor, not total building; todo: docs
pub fn create_structure_lattice(
    origin: glam::Vec3,
    width: f32,
    height: f32,
    depth: f32,
    floors: u32,
) -> XpbdLatticeBuilder {
    debug_assert!(floors > 0, "cannot create a structure with 0 floors");

    const FLOOR_NODE_COUNT: usize = 8;
    // include 4 anchor nodes of the building
    let total_node_count = FLOOR_NODE_COUNT * floors as usize + 4;

    const MASS: f32 = 150.0;

    const VERY_STIFF_COMPL: f32 = 0.15e-7;
    const STIFF_COMPL: f32 = 0.5e-5;
    const SOFT_COMPL: f32 = 0.35e-4;

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
    let bottom_l = lattice.node(Node::new(o + glam::vec3(-w, 0.0, 0.0), MASS).with_fixed(true));
    let bottom_r = lattice.node(Node::new(o + glam::vec3(w, 0.0, 0.0), MASS).with_fixed(true));
    let bottom_f = lattice.node(Node::new(o + glam::vec3(0.0, 0.0, d), MASS).with_fixed(true));
    let bottom_b = lattice.node(Node::new(o + glam::vec3(0.0, 0.0, -d), MASS).with_fixed(true));
    let bottom_center = lattice.node(Node::new(o, MASS).with_fixed(true));
    {
        lattice.link_nodes(bottom_l_b, bottom_b, STRONG_LINK);
        lattice.link_nodes(bottom_b, bottom_r_b, STRONG_LINK);

        lattice.link_nodes(bottom_r_b, bottom_r, STRONG_LINK);
        lattice.link_nodes(bottom_r, bottom_r_f, STRONG_LINK);

        lattice.link_nodes(bottom_r_f, bottom_f, STRONG_LINK);
        lattice.link_nodes(bottom_f, bottom_l_f, STRONG_LINK);

        lattice.link_nodes(bottom_l_f, bottom_l, STRONG_LINK);
        lattice.link_nodes(bottom_l, bottom_l_b, STRONG_LINK);

        lattice.link_nodes(bottom_b, bottom_center, STRONG_LINK);
        lattice.link_nodes(bottom_r, bottom_center, STRONG_LINK);
        lattice.link_nodes(bottom_f, bottom_center, STRONG_LINK);
        lattice.link_nodes(bottom_l, bottom_center, STRONG_LINK);
    }

    // back_left, center_back, back_right, center_right,
    // front_right, center_front, front_left, center_left,
    // origin
    // 0---1---2
    //         |
    // 7---8   3
    // |       |
    // 6---5---4
    let mut last_top = [
        bottom_l_b,
        bottom_b,
        bottom_r_b,
        bottom_r,
        bottom_r_f,
        bottom_f,
        bottom_l_f,
        bottom_l,
        bottom_center,
    ];

    for i in 0..floors {
        let ceiling_y = height * (i + 1) as f32;

        let back_left = lattice.node(Node::new(o + glam::vec3(-w, ceiling_y, -d), MASS));
        let back_right = lattice.node(Node::new(o + glam::vec3(w, ceiling_y, -d), MASS));
        let front_right = lattice.node(Node::new(o + glam::vec3(w, ceiling_y, d), MASS));
        let front_left = lattice.node(Node::new(o + glam::vec3(-w, ceiling_y, d), MASS));
        let center_left = lattice.node(Node::new(o + glam::vec3(-w, ceiling_y, 0.0), MASS));
        let center_right = lattice.node(Node::new(o + glam::vec3(w, ceiling_y, 0.0), MASS));
        let center_front = lattice.node(Node::new(o + glam::vec3(0.0, ceiling_y, d), MASS));
        let center_back = lattice.node(Node::new(o + glam::vec3(0.0, ceiling_y, -d), MASS));
        let origin = lattice.node(Node::new(o + glam::vec3(0.0, ceiling_y, 0.0), MASS));

        // top loop
        {
            lattice.link_nodes(back_left, center_back, STRONG_LINK);
            lattice.link_nodes(center_back, back_right, STRONG_LINK);

            lattice.link_nodes(back_right, center_right, STRONG_LINK);
            lattice.link_nodes(center_right, front_right, STRONG_LINK);

            lattice.link_nodes(front_right, center_front, STRONG_LINK);
            lattice.link_nodes(center_front, front_left, STRONG_LINK);

            lattice.link_nodes(front_left, center_left, STRONG_LINK);
            lattice.link_nodes(center_left, back_left, STRONG_LINK);
        }
        // pillars
        {
            lattice.link_nodes(back_left, last_top[0], STRONG_LINK);
            lattice.link_nodes(back_right, last_top[2], STRONG_LINK);
            lattice.link_nodes(front_right, last_top[4], STRONG_LINK);
            lattice.link_nodes(front_left, last_top[6], STRONG_LINK);

            lattice.link_nodes(center_back, last_top[1], MID_LINK);
            lattice.link_nodes(center_right, last_top[3], MID_LINK);
            lattice.link_nodes(center_front, last_top[5], MID_LINK);
            lattice.link_nodes(center_left, last_top[7], MID_LINK);

            // central spline
            lattice.link_nodes(origin, last_top[8], WEAK_LINK);
        }

        // side diagonals
        {
            lattice.link_nodes(back_left, last_top[2], MID_LINK);
            lattice.link_nodes(back_right, last_top[0], MID_LINK);

            lattice.link_nodes(front_right, last_top[2], MID_LINK);
            lattice.link_nodes(back_right, last_top[4], MID_LINK);

            lattice.link_nodes(front_left, last_top[4], MID_LINK);
            lattice.link_nodes(front_right, last_top[6], MID_LINK);

            lattice.link_nodes(back_left, last_top[6], MID_LINK);
            lattice.link_nodes(front_left, last_top[0], MID_LINK);
        }

        // floor diagonal and cross with intermediate
        {
            // lattice.link_nodes(back_left, front_right, WEAK_LINK);
            // lattice.link_nodes(front_left, back_right, WEAK_LINK);

            // lattice.link_nodes(back_left, origin, WEAK_LINK);
            // lattice.link_nodes(front_right, origin, WEAK_LINK);
            // lattice.link_nodes(back_right, origin, WEAK_LINK);
            // lattice.link_nodes(front_left, origin, WEAK_LINK);

            lattice.link_nodes(center_left, origin, WEAK_LINK);
            lattice.link_nodes(center_right, origin, WEAK_LINK);
            lattice.link_nodes(center_front, origin, WEAK_LINK);
            lattice.link_nodes(center_back, origin, WEAK_LINK);
        }

        last_top = [
            back_left,
            center_back,
            back_right,
            center_right,
            front_right,
            center_front,
            front_left,
            center_left,
            origin,
        ];
    }

    lattice
}
