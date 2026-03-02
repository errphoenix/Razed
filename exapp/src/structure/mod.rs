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

    const MASS: f32 = 50.0;

    const VERY_STIFF_COMPL: f32 = 0.1e-6;
    const STIFF_COMPL: f32 = 0.1e-5;
    const SOFT_COMPL: f32 = 0.25e-3;

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
    // 0---1---2
    //         |
    // 7       3
    // |       |
    // 6---5---4
    let mut last_top = [
        bottom_l_b, bottom_b, bottom_r_b, bottom_r, bottom_r_f, bottom_f, bottom_l_f, bottom_l,
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
            lattice.link_nodes(center_back, last_top[1], STRONG_LINK);
            lattice.link_nodes(back_right, last_top[2], STRONG_LINK);
            lattice.link_nodes(center_right, last_top[3], STRONG_LINK);
            lattice.link_nodes(front_right, last_top[4], STRONG_LINK);
            lattice.link_nodes(center_front, last_top[5], STRONG_LINK);
            lattice.link_nodes(front_left, last_top[6], STRONG_LINK);
            lattice.link_nodes(center_left, last_top[7], STRONG_LINK);
        }

        // side diagonals
        {
            lattice.link_nodes(back_left, last_top[1], WEAK_LINK);
            lattice.link_nodes(last_top[1], back_right, WEAK_LINK);
            lattice.link_nodes(back_right, last_top[3], WEAK_LINK);
            lattice.link_nodes(last_top[3], front_right, WEAK_LINK);
            lattice.link_nodes(front_right, last_top[5], WEAK_LINK);
            lattice.link_nodes(last_top[5], front_left, WEAK_LINK);
            lattice.link_nodes(front_left, last_top[7], WEAK_LINK);
            lattice.link_nodes(last_top[7], back_left, WEAK_LINK);

            // lattice.link_nodes(center_back, last_top[0], STRONG_LINK);
            // lattice.link_nodes(last_top[2], center_back, STRONG_LINK);
            // lattice.link_nodes(center_right, last_top[2], STRONG_LINK);
            // lattice.link_nodes(last_top[4], center_right, STRONG_LINK);
            // lattice.link_nodes(center_front, last_top[4], STRONG_LINK);
            // lattice.link_nodes(last_top[6], center_front, STRONG_LINK);
            // lattice.link_nodes(center_left, last_top[6], STRONG_LINK);
            // lattice.link_nodes(last_top[0], center_left, STRONG_LINK);
        }

        // side cross
        // {
        //     lattice.link_nodes(c_left, back_left, MID_LINK);
        //     lattice.link_nodes(c_left, front_left, MID_LINK);
        //     lattice.link_nodes(c_left, last_top[0], MID_LINK);
        //     lattice.link_nodes(c_left, last_top[3], MID_LINK);

        //     lattice.link_nodes(c_right, back_right, MID_LINK);
        //     lattice.link_nodes(c_right, front_right, MID_LINK);
        //     lattice.link_nodes(c_right, last_top[1], MID_LINK);
        //     lattice.link_nodes(c_right, last_top[2], MID_LINK);

        //     lattice.link_nodes(c_front, front_left, MID_LINK);
        //     lattice.link_nodes(c_front, front_right, MID_LINK);
        //     lattice.link_nodes(c_front, last_top[2], MID_LINK);
        //     lattice.link_nodes(c_front, last_top[3], MID_LINK);

        //     lattice.link_nodes(c_back, back_right, MID_LINK);
        //     lattice.link_nodes(c_back, back_left, MID_LINK);
        //     lattice.link_nodes(c_back, last_top[0], MID_LINK);
        //     lattice.link_nodes(c_back, last_top[1], MID_LINK);
        // }

        // floor cross
        {
            lattice.link_nodes(center_left, center_right, WEAK_LINK);
            lattice.link_nodes(center_back, center_front, WEAK_LINK);
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
        ];
    }

    lattice
}
