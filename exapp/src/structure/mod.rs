use physics::xpbd::{XpbdLatticeBuilder, XpbdLinkOptions, XpbdNodeOptions as Node};

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

    const MASS: f32 = 100.0;
    const COMPLIANCE: f32 = 0.5e-5;
    const LINK: XpbdLinkOptions = XpbdLinkOptions::new(COMPLIANCE);

    let mut lattice = XpbdLatticeBuilder::with_capacity(total_node_count);
    let w = width / 2.0;
    let d = depth / 2.0;
    let o = origin;

    // anchor nodes
    let bottom_l_b = lattice.node(Node::new(o + glam::vec3(-w, 0.0, -d), MASS).with_fixed(true));
    let bottom_r_b = lattice.node(Node::new(o + glam::vec3(w, 0.0, -d), MASS).with_fixed(true));
    let bottom_r_f = lattice.node(Node::new(o + glam::vec3(w, 0.0, d), MASS).with_fixed(true));
    let bottom_l_f = lattice.node(Node::new(o + glam::vec3(-w, 0.0, d), MASS).with_fixed(true));
    {
        lattice.link_nodes(bottom_l_b, bottom_r_b, LINK);
        lattice.link_nodes(bottom_r_b, bottom_r_f, LINK);
        lattice.link_nodes(bottom_r_f, bottom_l_f, LINK);
        lattice.link_nodes(bottom_l_f, bottom_l_b, LINK);
    }

    // back_left, back_right, front_right, front_left
    // 1---2
    //     |
    // 4---3
    let mut last_top = [bottom_l_b, bottom_r_b, bottom_r_f, bottom_l_f];

    for i in 0..floors {
        let ceiling_y = height * (i + 1) as f32;
        let mid_y = ceiling_y - height * 0.5;

        let back_left = lattice.node(Node::new(o + glam::vec3(-w, ceiling_y, -d), MASS));
        let back_right = lattice.node(Node::new(o + glam::vec3(w, ceiling_y, -d), MASS));
        let front_right = lattice.node(Node::new(o + glam::vec3(w, ceiling_y, d), MASS));
        let front_left = lattice.node(Node::new(o + glam::vec3(-w, ceiling_y, d), MASS));

        // top loop
        {
            lattice.link_nodes(back_left, back_right, LINK);
            lattice.link_nodes(back_right, front_right, LINK);
            lattice.link_nodes(front_right, front_left, LINK);
            lattice.link_nodes(front_left, back_left, LINK);
        }
        // pillars
        {
            lattice.link_nodes(back_left, last_top[0], LINK);
            lattice.link_nodes(back_right, last_top[1], LINK);
            lattice.link_nodes(front_right, last_top[2], LINK);
            lattice.link_nodes(front_left, last_top[3], LINK);
        }

        let c_left = lattice.node(Node::new(o + glam::vec3(-w, mid_y, 0.0), MASS));
        let c_right = lattice.node(Node::new(o + glam::vec3(w, mid_y, 0.0), MASS));
        let c_front = lattice.node(Node::new(o + glam::vec3(0.0, mid_y, d), MASS));
        let c_back = lattice.node(Node::new(o + glam::vec3(0.0, mid_y, -d), MASS));

        // cross
        {
            lattice.link_nodes(c_left, back_left, LINK);
            lattice.link_nodes(c_left, front_left, LINK);
            lattice.link_nodes(c_left, last_top[0], LINK);
            lattice.link_nodes(c_left, last_top[3], LINK);

            lattice.link_nodes(c_right, back_right, LINK);
            lattice.link_nodes(c_right, front_right, LINK);
            lattice.link_nodes(c_right, last_top[1], LINK);
            lattice.link_nodes(c_right, last_top[2], LINK);

            lattice.link_nodes(c_front, front_left, LINK);
            lattice.link_nodes(c_front, front_right, LINK);
            lattice.link_nodes(c_front, last_top[2], LINK);
            lattice.link_nodes(c_front, last_top[3], LINK);

            lattice.link_nodes(c_back, back_right, LINK);
            lattice.link_nodes(c_back, back_left, LINK);
            lattice.link_nodes(c_back, last_top[0], LINK);
            lattice.link_nodes(c_back, last_top[1], LINK);
        }

        last_top = [back_left, back_right, front_right, front_left];
    }

    lattice
}
