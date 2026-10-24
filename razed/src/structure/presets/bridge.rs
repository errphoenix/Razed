use ethel::Cell;
use physics::xpbd::{RawXpbdLattice, XpbdLatticeBuilder, XpbdLinkOptions, XpbdNodeOptions as Node};

use crate::procedural::voxel::VoxelFn;

#[derive(Clone, Copy, PartialEq, PartialOrd, Default, Debug)]
pub struct BridgeVoxelFn {
    pub width: f32,
    pub height: f32,
    pub depth: f32,
    pub deck_thickness: u32,
    pub pier_width: u32,
    pub truss_depth: u32,
}

impl VoxelFn for BridgeVoxelFn {
    fn pass(&self, cell: Cell) -> bool {
        let width = self.width as i32;
        let height = self.height as i32;
        let depth = self.depth as i32;

        if width <= 0 || height <= 0 || depth <= 0 {
            return false;
        }

        let x = cell.x + width / 2;
        let y = cell.y + height / 2;
        let z = cell.z + depth / 2;

        if x < 0 || x >= width || y < 0 || y >= height || z < 0 || z >= depth {
            return false;
        }

        let deck_thickness = self.deck_thickness.max(1) as i32;
        let pier_width = self.pier_width.max(1) as i32;
        let truss_depth = self.truss_depth.max(1) as i32;

        let deck_start = height.saturating_sub(deck_thickness);
        let deck = y >= deck_start;

        let pier_a = depth / 4;
        let pier_b = (depth * 3) / 4;
        let half_pier = pier_width / 2;
        let pier_at = |center: i32| (z - center).abs() <= half_pier;
        let pier = y < deck_start && (pier_at(pier_a) || pier_at(pier_b));

        let side_band = x < truss_depth || x >= width.saturating_sub(truss_depth);
        let span = depth.max(1) as f32;
        let rise = (height.saturating_sub(deck_start)).max(1) as f32;
        let relative_z = z as f32 / span;
        let relative_y = (y - deck_start).max(0) as f32 / rise;
        let diagonal_a = (relative_y - relative_z).abs() < 0.12;
        let diagonal_b = (relative_y - (1.0 - relative_z)).abs() < 0.12;
        let truss =
            y >= deck_start.saturating_sub(truss_depth) && side_band && (diagonal_a || diagonal_b);

        deck || pier || truss
    }
}

pub fn lattice(origin: glam::Vec3, width: f32, height: f32, depth: f32) -> RawXpbdLattice {
    debug_assert!(width > 0.0);
    debug_assert!(height > 0.0);
    debug_assert!(depth >= 4.0);

    const MASS: f32 = 100.0;

    const VERY_STIFF_COMPL: f32 = 0.1e-8;
    const STIFF_COMPL: f32 = 0.15e-7;
    const SOFT_COMPL: f32 = 0.2e-6;

    const STRONG_LINK: XpbdLinkOptions = XpbdLinkOptions::new(VERY_STIFF_COMPL);
    const MID_LINK: XpbdLinkOptions = XpbdLinkOptions::new(STIFF_COMPL);
    const WEAK_LINK: XpbdLinkOptions = XpbdLinkOptions::new(SOFT_COMPL);

    let w = width * 0.5;
    let d = depth;
    let truss_depth = (height * 0.3).clamp(0.5, 2.0);
    let lower_y = (height - truss_depth).max(0.1);

    let bay_count = ((depth / 3.0).round() as usize).max(2);
    let station_count = bay_count + 1;

    let mut lattice = XpbdLatticeBuilder::with_capacity(station_count * 4 + 4);
    let mut top_left = Vec::with_capacity(station_count);
    let mut top_right = Vec::with_capacity(station_count);
    let mut lower_left = Vec::with_capacity(station_count);
    let mut lower_right = Vec::with_capacity(station_count);

    for i in 0..station_count {
        let t = i as f32 / bay_count as f32;
        let z = -d * 0.5 + d * t;

        top_left.push(lattice.node(Node::new(origin + glam::vec3(-w, height, z), MASS)));
        top_right.push(lattice.node(Node::new(origin + glam::vec3(w, height, z), MASS)));

        lower_left.push(lattice.node(Node::new(origin + glam::vec3(-w, lower_y, z), MASS)));
        lower_right.push(lattice.node(Node::new(origin + glam::vec3(w, lower_y, z), MASS)));
    }

    for i in 0..bay_count {
        lattice.link_nodes(top_left[i], top_left[i + 1], STRONG_LINK);
        lattice.link_nodes(top_right[i], top_right[i + 1], STRONG_LINK);

        lattice.link_nodes(top_left[i], top_right[i], MID_LINK);
        lattice.link_nodes(top_left[i], top_right[i + 1], WEAK_LINK);
        lattice.link_nodes(top_right[i], top_left[i + 1], WEAK_LINK);
    }

    lattice.link_nodes(top_left[bay_count], top_right[bay_count], MID_LINK);

    for i in 0..bay_count {
        lattice.link_nodes(lower_left[i], lower_left[i + 1], MID_LINK);
        lattice.link_nodes(lower_right[i], lower_right[i + 1], MID_LINK);

        lattice.link_nodes(top_left[i], lower_left[i], MID_LINK);
        lattice.link_nodes(top_right[i], lower_right[i], MID_LINK);

        if i % 2 == 0 {
            lattice.link_nodes(top_left[i], lower_left[i + 1], WEAK_LINK);
            lattice.link_nodes(lower_right[i], top_right[i + 1], WEAK_LINK);
        } else {
            lattice.link_nodes(lower_left[i], top_left[i + 1], WEAK_LINK);
            lattice.link_nodes(top_right[i], lower_right[i + 1], WEAK_LINK);
        }
    }

    lattice.link_nodes(top_left[bay_count], lower_left[bay_count], MID_LINK);
    lattice.link_nodes(top_right[bay_count], lower_right[bay_count], MID_LINK);

    for support_z in [-d * 0.25, d * 0.25] {
        let i = ((support_z + d * 0.5) / d * bay_count as f32)
            .round()
            .clamp(0.0, bay_count as f32) as usize;

        let z_front = support_z - 0.75;
        let z_back = support_z + 0.75;

        let anchors = [
            lattice.node(Node::new(origin + glam::vec3(-w, 0.0, z_front), MASS).with_fixed(true)),
            lattice.node(Node::new(origin + glam::vec3(w, 0.0, z_front), MASS).with_fixed(true)),
            lattice.node(Node::new(origin + glam::vec3(-w, 0.0, z_back), MASS).with_fixed(true)),
            lattice.node(Node::new(origin + glam::vec3(w, 0.0, z_back), MASS).with_fixed(true)),
        ];

        lattice.link_nodes(anchors[0], anchors[1], STRONG_LINK);
        lattice.link_nodes(anchors[0], anchors[2], STRONG_LINK);
        lattice.link_nodes(anchors[1], anchors[3], STRONG_LINK);
        lattice.link_nodes(anchors[2], anchors[3], STRONG_LINK);

        lattice.link_nodes(anchors[0], top_left[i], STRONG_LINK);
        lattice.link_nodes(anchors[1], top_right[i], STRONG_LINK);
        lattice.link_nodes(anchors[2], lower_left[i], MID_LINK);
        lattice.link_nodes(anchors[3], lower_right[i], MID_LINK);

        lattice.link_nodes(anchors[0], lower_right[i], MID_LINK);
        lattice.link_nodes(anchors[1], lower_left[i], MID_LINK);
        lattice.link_nodes(anchors[2], top_right[i], MID_LINK);
        lattice.link_nodes(anchors[3], top_left[i], MID_LINK);
    }

    lattice.build()
}
