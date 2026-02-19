use ethel::state::data::Column;
use janus::context::DeltaTime;

#[derive(Debug, Clone, Copy, Default)]
pub struct XpbdNodeOptions {
    pos: glam::Vec3,
    mass: f32,
    fixed: bool,
}

impl XpbdNodeOptions {
    pub const fn new(pos: glam::Vec3, mass: f32) -> Self {
        Self {
            pos,
            mass,
            fixed: false,
        }
    }

    pub const fn with_fixed(self, fixed: bool) -> Self {
        Self {
            pos: self.pos,
            mass: self.mass,
            fixed,
        }
    }
}

impl XpbdLinkOptions {
    pub const fn new(compliance: f32) -> Self {
        Self {
            compliance,
            rest_length: None,
        }
    }

    pub const fn with_rest_length(compliance: f32, rest_length: f32) -> Self {
        Self {
            compliance,
            rest_length: Some(rest_length),
        }
    }

    pub const fn and_rest_length(self, rest_length: f32) -> Self {
        Self {
            compliance: self.compliance,
            rest_length: Some(rest_length),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct XpbdLinkOptions {
    compliance: f32,
    rest_length: Option<f32>,
}

#[derive(Debug, Clone, Copy, Default)]
struct XpbdLink {
    node_a: u32,
    node_b: u32,
    options: XpbdLinkOptions,
}

#[derive(Debug, Clone, Default)]
pub struct XpbdLatticeBuilder {
    nodes: Vec<XpbdNodeOptions>,
    links: Vec<XpbdLink>,
    stack: Vec<u32>,
}

impl XpbdLatticeBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(capacity),
            links: Vec::with_capacity(capacity * 3),
            stack: Vec::with_capacity(capacity / 3),
        }
    }

    /// Push a new node in the hierarchy with the specified `options`.
    ///
    /// Subsequent [`node`] and [`link`] operations will operate on this new
    /// node.
    ///
    /// # Returns
    /// Returns the index of the node in the hierarchy.
    ///
    /// This index can be used to reference a node and create an explicit link,
    /// through [`XpbdLatticeBuilder::link_to`].
    ///
    /// [`node`]: XpbdLatticeBuilder::node
    /// [`link`]: XpbdLatticeBuilder::link
    pub fn node(&mut self, options: XpbdNodeOptions) -> u32 {
        let id = self.nodes.len();
        self.stack.push(id as u32);
        self.nodes.push(options);
        id as u32
    }

    /// Create a contraint between the last 2 nodes in the stack.
    ///
    /// This effectively creates a link between the current node and its
    /// parent.
    ///
    /// After this operation, the last node is popped off the stack, so no
    /// other links can be created to it unless you explicitly reference it
    /// using its ID.
    ///
    /// Afterwards the context returns to the parent node. All subsequent node
    /// and link operations will operate on that node, again.
    ///
    /// Also see [`XpbdLatticeBuilder::link_to`] for explicit constraints
    /// linking.
    ///
    /// # Panics
    /// Will panic if there are less than 2 nodes currently in the stack.
    ///
    /// # Returns
    /// Returns the index of the newly created link.
    pub fn link(&mut self, options: XpbdLinkOptions) -> u32 {
        debug_assert!(
            self.stack.len() >= 2,
            "attempted to create lattice link with less than 2 nodes in stack"
        );

        let id = self.stack.pop().expect("stack must have >=2 nodes");
        let parent = self.stack.last().expect("stack must have >=2 nodes");

        let link_id = self.links.len();
        self.links.push(XpbdLink {
            node_a: *parent,
            node_b: id,
            options,
        });
        link_id as u32
    }

    /// Create a contraint between the current node in the stack and an
    /// arbitrary `node_id`.
    ///
    /// The `node_id` must be an ID returned from the
    /// [`node`](XpbdLatticeBuilder::node) function.
    ///
    /// The intended use is for cross-node relations that cannot be created
    /// as a hierarchical tree through the standard
    /// [`link`](XpbdLatticeBuilder::link) function.
    ///
    /// # Panics
    /// Will panic if there are less than 2 nodes currently in the stack or if
    /// `node_id` does not point to a valid node in the stack.
    /// Will also panic if `node_id` corresponds to the current node in the
    /// stack, as a node cannot be linked to itself.
    ///
    /// # Returns
    /// Returns the index of the newly created link.
    pub fn link_to(&mut self, node_id: u32, options: XpbdLinkOptions) -> u32 {
        debug_assert!(
            self.stack.len() >= 1,
            "attempted to create lattice link with no nodes in the stack"
        );

        let id = *self.stack.last().expect("stack must be populated");
        debug_assert!(id != node_id, "cannot links node {id} to itself");

        let link_id = self.links.len();
        self.links.push(XpbdLink {
            node_a: id,
            node_b: node_id,
            options,
        });
        link_id as u32
    }

    /// Create a link between two nodes `node_a` and `node_b`.
    ///
    /// The node IDs must be nodes provided by the [`node`] function.
    ///
    /// This will create a constraint between two arbitrary nodes with the
    /// given `options` as constrant properties.
    ///
    /// Also see [`XpbdLatticeBuilder::link`] and
    /// [`XpbdLatticeBuilder::link_to`] for alternative ways of constructing
    /// lattice structures.
    ///
    /// [`node`]: XpbdLatticeBuilder::node
    ///
    /// # Panics
    /// Will panic if either `node_a` of `node_b` do not point to a valid node ID.
    ///
    /// # Returns
    /// Returns the index of the newly created link.
    pub fn link_nodes(&mut self, node_a: u32, node_b: u32, options: XpbdLinkOptions) -> u32 {
        #[cfg(debug_assertions)]
        {
            let node_count = self.nodes.len() as u32;
            debug_assert!(
                node_a < node_count,
                "attempted to create a link containing invalid node {node_a}"
            );
            debug_assert!(
                node_b < node_count,
                "attempted to create a link containing invalid node {node_b}"
            );
        }

        let link_id = self.links.len();
        self.links.push(XpbdLink {
            node_a,
            node_b,
            options,
        });
        link_id as u32
    }

    /// Export the current defined lattice structure into the given tables.
    ///
    /// # Returns
    /// A mapping of the [`LatticeIds`] between the indices of the nodes and
    /// links and the indirect indices of the actual nodes and links in their
    /// respective tables.
    pub fn export(mut self, nodes: &mut NodesRowTable, links: &mut LinksRowTable) -> LatticeIds {
        let node_ids = self
            .nodes
            .drain(..)
            .map(|node_opt| {
                let p_pos = node_opt.pos;
                let c_pos = node_opt.pos;
                let mass = node_opt.mass;
                let mut inv_mass = 1.0 / node_opt.mass;
                if !inv_mass.is_normal() || node_opt.fixed {
                    inv_mass = 0.0;
                }
                let forces = glam::Vec3::ZERO;
                let velocity = glam::Vec3::ZERO;

                nodes.put((p_pos, c_pos, mass, inv_mass, forces, velocity))
            })
            .collect::<Vec<_>>();

        let link_ids = self
            .links
            .drain(..)
            .map(|link| {
                let relation = LinkNodes(
                    node_ids[link.node_a as usize],
                    node_ids[link.node_b as usize],
                );

                let lambda = 0f32;
                let compliance = link.options.compliance;
                let rest_length = link.options.rest_length.unwrap_or_else(|| {
                    let ip_a = unsafe { nodes.get_indirect_unchecked(relation.0) };
                    let ip_b = unsafe { nodes.get_indirect_unchecked(relation.1) };

                    let node_positions = nodes.current_pos_slice();
                    let p_a = unsafe { node_positions.get_unchecked(ip_a as usize) };
                    let p_b = unsafe { node_positions.get_unchecked(ip_b as usize) };
                    (p_a - p_b).length()
                });

                links.put((relation, compliance, rest_length, lambda))
            })
            .collect::<Vec<_>>();

        LatticeIds {
            nodes: node_ids,
            links: link_ids,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct LatticeIds {
    pub nodes: Vec<u32>,
    pub links: Vec<u32>,
}

pub const DEFAULT_SOLVE_ITERATIONS: u32 = 8;
pub const DEFAULT_SUB_STEPS: u32 = 4;
pub const DAMPING: f32 = 0.996;

ethel::table_spec! {
    struct Nodes {
        predicted_pos: glam::Vec3;
        current_pos: glam::Vec3;
        mass: f32;
        inv_mass: f32;
        forces: glam::Vec3;
        velocity: glam::Vec3;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct LinkNodes(pub u32, pub u32);

ethel::table_spec! {
    struct Links {
        relation: LinkNodes;
        compliance: f32;
        rest_length: f32;
        lambda: f32;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct XpbdSolver {
    iterations: u32,
    substeps: u32,
    h: f32,
    h2: f32,
    allow_breaking: bool,
    ground_level: Option<f32>,
    broken_links: Vec<u32>,
}

impl Default for XpbdSolver {
    fn default() -> Self {
        Self {
            iterations: DEFAULT_SOLVE_ITERATIONS,
            substeps: DEFAULT_SUB_STEPS,
            h: 0.0,
            h2: 0.0,
            ground_level: None,
            allow_breaking: true,
            broken_links: Vec::with_capacity(32),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct XpbdOptions {
    pub iterations: u32,
    pub substeps: u32,
    pub allow_breaking: bool,
    pub ground_level: Option<f32>,
}

impl XpbdOptions {
    pub const fn new(
        iterations: u32,
        substeps: u32,
        allow_breaking: bool,
        ground_level: Option<f32>,
    ) -> Self {
        Self {
            iterations,
            substeps,
            allow_breaking,
            ground_level,
        }
    }

    pub const fn with_iterations(self, iterations: u32) -> Self {
        Self {
            iterations,
            substeps: self.substeps,
            allow_breaking: self.allow_breaking,
            ground_level: self.ground_level,
        }
    }

    pub const fn with_substeps(self, substeps: u32) -> Self {
        Self {
            substeps,
            iterations: self.iterations,
            allow_breaking: self.allow_breaking,
            ground_level: self.ground_level,
        }
    }

    pub const fn with_breaking(self, breaking: bool) -> Self {
        Self {
            allow_breaking: breaking,
            iterations: self.iterations,
            substeps: self.substeps,
            ground_level: self.ground_level,
        }
    }

    pub const fn with_ground_level(self, ground_level: Option<f32>) -> Self {
        Self {
            ground_level,
            iterations: self.iterations,
            substeps: self.substeps,
            allow_breaking: self.allow_breaking,
        }
    }
}

impl Default for XpbdOptions {
    fn default() -> Self {
        Self {
            iterations: DEFAULT_SOLVE_ITERATIONS,
            substeps: DEFAULT_SUB_STEPS,
            allow_breaking: true,
            ground_level: None,
        }
    }
}

impl XpbdSolver {
    #[inline]
    pub fn new(options: XpbdOptions) -> Self {
        Self {
            h: 0.0,
            h2: 0.0,
            iterations: options.iterations,
            substeps: options.substeps,
            allow_breaking: options.allow_breaking,
            ground_level: options.ground_level,
            broken_links: Vec::with_capacity(32 * options.allow_breaking as usize),
        }
    }

    #[inline]
    pub const fn iterations(&self) -> u32 {
        self.iterations
    }

    #[inline]
    pub const fn substeps(&self) -> u32 {
        self.substeps
    }

    #[inline]
    pub const fn set_iterations(&mut self, iterations: u32) {
        self.iterations = iterations;
    }

    #[inline]
    pub const fn set_substeps(&mut self, substeps: u32) {
        self.substeps = substeps;
    }

    #[inline]
    pub const fn set_step_time(&mut self, delta: DeltaTime) {
        self.h = delta.as_f32() / self.substeps as f32;
        self.h2 = self.h * self.h;
    }

    /// Returns a slice over the constraint IDs that were broken in the last
    /// step.
    ///
    /// This is reset at the beginning of every step. Broken constraint IDs
    /// are accumulated every sub-step.
    ///
    /// # Panics
    /// Will panic if the XPBD's solver `allow_breaking` flag is `false`.
    pub fn broken_links(&self) -> &[u32] {
        assert!(
            self.allow_breaking,
            "cannot query broken links: allow_breaking flag for XPBD is set to false"
        );

        &self.broken_links
    }

    #[inline]
    pub fn step(&mut self, nodes: &mut NodesRowTable, links: &mut LinksRowTable) {
        self.broken_links.clear();
        for _ in 0..self.substeps {
            self.substep(nodes, links);
        }
        for v in nodes.velocity_mut_slice() {
            *v *= DAMPING;
        }
    }

    #[inline]
    fn substep(&mut self, nodes: &mut NodesRowTable, links: &mut LinksRowTable) {
        self.predict_positions(nodes);
        if self.ground_level.is_some() {
            self.apply_ground_constraint(nodes);
        }

        links.lambda_mut_slice().fill(0.0);
        for _ in 0..self.iterations {
            self.solve_constraints(nodes, links);
        }

        if self.allow_breaking {
            const LAMBDA_STRAIN_THRESHOLD: f32 = 45_000.0;
            const LAMBDA_COMPRESSION_THRESHOLD: f32 = -15_000.0;

            for (handle, lambda) in links.handles().iter().zip(links.lambda_slice()) {
                let force_strain = *lambda / self.h2;
                if force_strain >= LAMBDA_STRAIN_THRESHOLD
                    || force_strain <= LAMBDA_COMPRESSION_THRESHOLD
                {
                    self.broken_links.push(*handle);
                }
            }

            self.broken_links.iter().for_each(|&handle| {
                links.free(handle);
            });
        }
        self.finalise_nodes(nodes);
    }

    #[inline]
    fn predict_positions(&self, nodes: &mut NodesRowTable) {
        let node_count = nodes.len();
        let c_pos = &nodes.current_pos;
        let inv_mass = &nodes.inv_mass;
        let velocity = &nodes.velocity;
        let p_pos = &mut nodes.predicted_pos;
        let forces = &mut nodes.forces;

        for i in 0..node_count {
            let x = c_pos[i];
            let f = std::mem::take(&mut forces[i]);
            let v = velocity[i];
            let w = inv_mass[i];

            let p = &mut p_pos[i];

            *p = x + self.h * v + self.h2 * f * w;
        }
    }

    #[inline]
    fn solve_constraints(&self, node_data: &mut NodesRowTable, link_data: &mut LinksRowTable) {
        let (rel, comp, len, lambda) = link_data.split_mut();
        let view = rel.join(comp).join(len).join(lambda);

        for (ab, inv_stiffness, l, y) in view {
            let i_a = unsafe { node_data.get_indirect_unchecked(ab.0) };
            let i_b = unsafe { node_data.get_indirect_unchecked(ab.1) };
            let inv_mass = &node_data.inv_mass;
            let position = &mut node_data.predicted_pos;

            let w_a = inv_mass[i_a as usize];
            let w_b = inv_mass[i_b as usize];

            let p_a = position[i_a as usize];
            let p_b = position[i_b as usize];

            let ab_d = p_a - p_b;
            let dist = ab_d.length();
            if dist < 0.1e-6 {
                continue;
            }

            let compliance = *inv_stiffness / self.h2;

            let w_t = w_a + w_b;
            if w_t < 0.1e-6 {
                continue;
            }

            let constraint = dist - *l;
            let d_y = (-constraint - compliance * *y) / (w_a + w_b + compliance);
            *y += d_y;

            let gradient = ab_d / dist;
            position[i_a as usize] += w_a * d_y * gradient;
            position[i_b as usize] -= w_b * d_y * gradient;
        }
    }

    #[inline]
    fn apply_ground_constraint(&self, node_data: &mut NodesRowTable) {
        const RESTITUTION: f32 = 0.4;
        const FRICTION: f32 = 0.2;

        let ground_level = self.ground_level.unwrap_or_default();
        let (n_pos, c_pos, _, _, _, velocity) = node_data.split_mut();
        for (n_pos, c_pos, vel) in n_pos.join(c_pos).join(velocity) {
            if n_pos.y < ground_level {
                n_pos.y = ground_level;
                c_pos.y = ground_level;

                vel.y *= -RESTITUTION;
                vel.x *= FRICTION;
                vel.z *= FRICTION;
            }
        }
    }

    #[inline]
    fn finalise_nodes(&self, node_data: &mut NodesRowTable) {
        let (p_pos, c_pos, _, _, _, vel) = node_data.split_mut();

        for (p, x, v) in p_pos.join(c_pos).join(vel) {
            *v = (*p - *x) / self.h;
            *x = *p;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xpbd_lattice_builder() {
        let mut builder = XpbdLatticeBuilder::new();

        {
            const MASS: f32 = 5.0;
            const POS: glam::Vec3 = glam::Vec3::ONE;
            const COMPLIANCE: f32 = 1.0;

            const NODE: XpbdNodeOptions = XpbdNodeOptions::new(POS, MASS);
            const LINK: XpbdLinkOptions = XpbdLinkOptions::new(COMPLIANCE);

            builder.node(NODE); // A
            builder.node(NODE); // B
            builder.node(NODE); // C
            builder.link(LINK); // B->C
            builder.link(LINK); // A->B
            builder.node(NODE); // D
            builder.node(NODE); // E
            builder.node(NODE); // F
            builder.link(LINK); // E->F
            builder.link(LINK); // D->E
            builder.node(NODE); // G
            builder.node(NODE); // H
            builder.link(LINK); // G->H
            builder.link(LINK); // D->G
            builder.link(LINK); // A->D
        }

        const A: u32 = 0;
        const B: u32 = 1;
        const C: u32 = 2;
        const D: u32 = 3;
        const E: u32 = 4;
        const F: u32 = 5;
        const G: u32 = 6;
        const H: u32 = 7;

        const BC: u32 = 0;
        const AB: u32 = 1;
        const EF: u32 = 2;
        const DE: u32 = 3;
        const GH: u32 = 4;
        const DG: u32 = 5;
        const AD: u32 = 6;

        let mut nodes = NodesRowTable::new();
        let mut links = LinksRowTable::new();

        let map = builder.export(&mut nodes, &mut links);
        {
            let node_ids = map.nodes;
            let compare = {
                let mut v = vec![A, B, C, D, E, F, G, H];
                v.iter_mut().for_each(|i| *i += 1);
                v
            };
            assert_eq!(node_ids, compare);

            let link_ids = map.links;
            let compare = {
                let mut v = vec![BC, AB, EF, DE, GH, DG, AD];
                v.iter_mut().for_each(|i| *i += 1);
                v
            };
            assert_eq!(link_ids, compare);
        }
    }

    #[test]
    fn xpbd_lattice_builder_cross_link() {
        let mut builder = XpbdLatticeBuilder::new();

        {
            const MASS: f32 = 5.0;
            const POS: glam::Vec3 = glam::Vec3::ONE;
            const COMPLIANCE: f32 = 1.0;

            const NODE: XpbdNodeOptions = XpbdNodeOptions::new(POS, MASS);
            const LINK: XpbdLinkOptions = XpbdLinkOptions::new(COMPLIANCE);

            let root = builder.node(NODE); // A
            builder.node(NODE); // B
            builder.node(NODE); // C
            builder.node(NODE); // D
            builder.link_to(root, LINK); // A->D
            builder.link(LINK); // C->D
            builder.link(LINK); // B->C
        }

        const A: u32 = 0;
        const B: u32 = 1;
        const C: u32 = 2;
        const D: u32 = 3;

        const AD: u32 = 0;
        const CD: u32 = 1;
        const BC: u32 = 2;

        let mut nodes = NodesRowTable::new();
        let mut links = LinksRowTable::new();

        let map = builder.export(&mut nodes, &mut links);
        {
            let node_ids = map.nodes;
            let compare = {
                let mut v = vec![A, B, C, D];
                v.iter_mut().for_each(|i| *i += 1);
                v
            };
            assert_eq!(node_ids, compare);

            let link_ids = map.links;
            dbg!(&links);
            let compare = {
                let mut v = vec![AD, CD, BC];
                v.iter_mut().for_each(|i| *i += 1);
                v
            };
            assert_eq!(link_ids, compare);
        }
    }
}
