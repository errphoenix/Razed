pub mod deforms;
pub mod fragment;

use ethel::state::data::{Column, SparseSlot};
use physics::xpbd::{NodesRowTable, XpbdLatticeBuilder, XpbdLinkOptions, XpbdNodeOptions as Node};

#[allow(unused_imports)]
pub use fragment::{FragmentState, FragmentSystem};

#[derive(Debug, Clone, Copy)]
pub struct LatticeView<'nodes> {
    pub sparse_ids: &'nodes [u32],
    pub positions: &'nodes [glam::Vec3],
    pub handles: &'nodes [u32],

    /// The indexing offset between sparse index values and contiguous slices.
    slice_offset: usize,
}

impl<'nodes> LatticeView<'nodes> {
    pub fn from(nodes: &'nodes NodesRowTable) -> Self {
        Self {
            sparse_ids: nodes.slots_map(),
            positions: nodes.current_pos_slice(),
            handles: nodes.handles(),
            slice_offset: 0,
        }
    }

    pub fn from_range(nodes: &'nodes NodesRowTable, offset: usize, length: usize) -> Self {
        debug_assert!(
            offset < nodes.len(),
            "cannot construct LatticeView: offset {offset} goes beyond table length of {}",
            nodes.len()
        );
        debug_assert!(
            (offset + length) < nodes.len(),
            "cannot construct LatticeView: attempted to create view over range {offset}..{} for table of length {}",
            offset + length,
            nodes.len()
        );

        Self {
            sparse_ids: nodes.slots_map(),
            positions: &nodes.current_pos_slice()[offset..(offset + length)],
            handles: &nodes.handles()[offset..(offset + length)],
            slice_offset: offset,
        }
    }

    /// Returns the node position and handle (reverse indirect ID) of the given
    /// node.
    ///
    /// # Panics
    /// Will panic if `indirect_id` is out-of-bounds to the sparse IDs map.
    pub fn query(&self, indirect_id: u32) -> (glam::Vec3, u32) {
        let direct_idx = self.sparse_ids[indirect_id as usize] as usize;
        let position = self.positions[direct_idx - self.slice_offset];
        let handle = self.handles[direct_idx - self.slice_offset];
        (position, handle)
    }

    /// Returns the node position and handle (reverse indirect ID) of the given
    /// node.
    ///
    /// # Panics
    /// Will panic if `indirect_id` is out-of-bounds to the sparse IDs map.
    ///
    /// # Safety
    /// This operation is safe as long as `indirect_id` is guaranteed to be
    /// a valid indirect index.
    pub unsafe fn query_unchecked(&self, indirect_id: u32) -> (glam::Vec3, u32) {
        let direct_idx = *unsafe { self.sparse_ids.get_unchecked(indirect_id as usize) } as usize;
        let position = *unsafe { self.positions.get_unchecked(direct_idx - self.slice_offset) };
        let handle = *unsafe { self.handles.get_unchecked(direct_idx - self.slice_offset) };
        (position, handle)
    }

    /// Returns the node position of the given node.
    ///
    /// # Panics
    /// Will panic if `indirect_id` is out-of-bounds to the sparse IDs map.
    pub fn position(&self, indirect_id: u32) -> glam::Vec3 {
        let direct_idx = self.sparse_ids[indirect_id as usize] as usize;
        self.positions[direct_idx - self.slice_offset]
    }

    /// Returns the node position of the given node.
    ///
    /// # Panics
    /// Will panic if `indirect_id` is out-of-bounds to the sparse IDs map.
    ///
    /// # Safety
    /// This operation is safe as long as `indirect_id` is guaranteed to be
    /// a valid indirect index.
    pub unsafe fn position_unchecked(&self, indirect_id: u32) -> glam::Vec3 {
        let direct_idx = *unsafe { self.sparse_ids.get_unchecked(indirect_id as usize) } as usize;
        *unsafe { self.positions.get_unchecked(direct_idx - self.slice_offset) }
    }

    /// Returns the handle (reverse indirect ID) of the given node.
    ///
    /// # Panics
    /// Will panic if `indirect_id` is out-of-bounds to the sparse IDs map.
    pub fn handle(&self, indirect_id: u32) -> u32 {
        let direct_idx = self.sparse_ids[indirect_id as usize] as usize;
        self.handles[direct_idx - self.slice_offset]
    }

    /// Returns the handle (reverse indirect ID) of the given node.
    ///
    /// # Panics
    /// Will panic if `indirect_id` is out-of-bounds to the sparse IDs map.
    ///
    /// # Safety
    /// This operation is safe as long as `indirect_id` is guaranteed to be
    /// a valid indirect index.
    pub unsafe fn handle_unchecked(&self, indirect_id: u32) -> u32 {
        let direct_idx = *unsafe { self.sparse_ids.get_unchecked(indirect_id as usize) } as usize;
        *unsafe { self.handles.get_unchecked(direct_idx - self.slice_offset) }
    }
}

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

    const VERY_STIFF_COMPL: f32 = 0.10e-7;
    const STIFF_COMPL: f32 = 0.75e-5;
    const SOFT_COMPL: f32 = 0.1e-4;

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

        let half_y = ceiling_y - (height * 0.5);
        let middle_back = lattice.node(Node::new(o + glam::vec3(0.0, half_y, -d), MASS));
        let middle_front = lattice.node(Node::new(o + glam::vec3(0.0, half_y, d), MASS));
        let middle_right = lattice.node(Node::new(o + glam::vec3(w, half_y, 0.0), MASS));
        let middle_left = lattice.node(Node::new(o + glam::vec3(-w, half_y, 0.0), MASS));

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

            lattice.link_nodes(center_back, last_top[1], STRONG_LINK);
            lattice.link_nodes(center_right, last_top[3], STRONG_LINK);
            lattice.link_nodes(center_front, last_top[5], STRONG_LINK);
            lattice.link_nodes(center_left, last_top[7], STRONG_LINK);

            // central spline
            lattice.link_nodes(origin, last_top[8], STRONG_LINK);
        }

        // side diagonals
        {
            lattice.link_nodes(back_left, middle_back, MID_LINK);
            lattice.link_nodes(back_right, middle_back, MID_LINK);
            lattice.link_nodes(middle_back, last_top[2], MID_LINK);
            lattice.link_nodes(middle_back, last_top[0], MID_LINK);

            lattice.link_nodes(front_right, middle_right, MID_LINK);
            lattice.link_nodes(back_right, middle_right, MID_LINK);
            lattice.link_nodes(middle_right, last_top[2], MID_LINK);
            lattice.link_nodes(middle_right, last_top[4], MID_LINK);

            lattice.link_nodes(front_left, middle_front, MID_LINK);
            lattice.link_nodes(front_right, middle_front, MID_LINK);
            lattice.link_nodes(middle_front, last_top[4], MID_LINK);
            lattice.link_nodes(middle_front, last_top[6], MID_LINK);

            lattice.link_nodes(back_left, middle_left, MID_LINK);
            lattice.link_nodes(front_left, middle_left, MID_LINK);
            lattice.link_nodes(middle_left, last_top[6], MID_LINK);
            lattice.link_nodes(middle_left, last_top[0], MID_LINK);
        }

        // floor diagonal and cross with intermediate
        {
            // lattice.link_nodes(back_left, front_right, WEAK_LINK);
            // lattice.link_nodes(front_left, back_right, WEAK_LINK);

            lattice.link_nodes(back_left, origin, WEAK_LINK);
            lattice.link_nodes(front_right, origin, WEAK_LINK);
            lattice.link_nodes(back_right, origin, WEAK_LINK);
            lattice.link_nodes(front_left, origin, WEAK_LINK);

            lattice.link_nodes(center_left, origin, STRONG_LINK);
            lattice.link_nodes(center_right, origin, STRONG_LINK);
            lattice.link_nodes(center_front, origin, STRONG_LINK);
            lattice.link_nodes(center_back, origin, STRONG_LINK);
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
