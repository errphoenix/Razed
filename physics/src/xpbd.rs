use ethel::state::data::Column;
use janus::context::DeltaTime;

#[derive(Debug, Clone, Copy, Default)]
pub struct XpbdNodeOptions {
    pos: glam::Vec3,
    mass: f32,
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
    /// # Returns
    /// Returns the index of the node in the hierarchy.
    pub fn node(&mut self, options: XpbdNodeOptions) -> u32 {
        let id = self.nodes.len();
        self.stack.push(id as u32);
        self.nodes.push(options);
        id as u32
    }

    /// Finalise a link between the last 2 nodes in the stack.
    ///
    /// The last node is popped off the stack, but its parent remains. This
    /// allows for the construction of hierarchies or more complex lattice
    /// structures.
    ///
    /// # Panics
    /// Will panic if there are less than 2 nodes currently in the stack.
    ///
    /// # Returns
    /// Returns the index of the link in the hierarchy.
    pub fn link(&mut self, options: XpbdLinkOptions) -> u32 {
        assert!(
            self.stack.len() > 2,
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
                let inv_mass = 1.0 / node_opt.mass;
                let forces = glam::Vec3::ZERO;
                let velocity = glam::Vec3::ZERO;

                nodes.put((p_pos, c_pos, mass, inv_mass, forces, velocity))
            })
            .collect::<Vec<_>>();

        let link_ids = self
            .links
            .drain(..)
            .map(|link| {
                let relation = LinkNodes(link.node_a, link.node_b);
                let lambda = 0f32;
                let compliance = link.options.compliance;
                let rest_length = link.options.rest_length.unwrap_or_else(|| {
                    let i_a = node_ids[relation.0 as usize];
                    let i_b = node_ids[relation.1 as usize];

                    let ip_a = unsafe { nodes.get_indirect_unchecked(i_a) };
                    let ip_b = unsafe { nodes.get_indirect_unchecked(i_b) };

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
pub const DEFAULT_SUB_STEPS: u32 = 6;

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
pub struct LinkNodes(u32, u32);

ethel::table_spec! {
    struct Links {
        relation: LinkNodes;
        compliance: f32;
        rest_length: f32;
        lambda: f32;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct XpbdSolver {
    iterations: u32,
    substeps: u32,
    h: f32,
    h2: f32,
}

impl Default for XpbdSolver {
    fn default() -> Self {
        Self {
            iterations: DEFAULT_SOLVE_ITERATIONS,
            substeps: DEFAULT_SUB_STEPS,
            h: 0.0,
            h2: 0.0,
        }
    }
}

impl XpbdSolver {
    #[inline]
    pub fn new(iterations: u32, substeps: u32) -> Self {
        Self {
            iterations,
            substeps,
            h: 0.0,
            h2: 0.0,
        }
    }

    #[inline]
    pub fn iterations(&self) -> u32 {
        self.iterations
    }

    #[inline]
    pub fn substeps(&self) -> u32 {
        self.substeps
    }

    #[inline]
    pub fn set_iterations(&mut self, iterations: u32) {
        self.iterations = iterations;
    }

    #[inline]
    pub fn set_substeps(&mut self, substeps: u32) {
        self.substeps = substeps;
    }

    #[inline]
    pub fn set_step_time(&mut self, delta: DeltaTime) {
        self.h = delta.as_f32() / self.substeps as f32;
        self.h2 = self.h * self.h;
    }

    #[inline]
    pub fn step(&self, nodes: &mut NodesRowTable, links: &mut LinksRowTable) {
        for _ in 0..self.substeps {
            self.substep(nodes, links);
        }
    }

    #[inline]
    fn substep(&self, nodes: &mut NodesRowTable, links: &mut LinksRowTable) {
        self.predict_positions(nodes);
        links.lambda_mut_slice().fill(0.0);
        for _ in 0..self.iterations {
            self.solve_constraints(nodes, links);
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
            let compliance = *inv_stiffness / self.h2;

            let constraint = dist - *l;
            let d_y = (-constraint - compliance * *y) / (w_a + w_b + compliance);
            *y += d_y;

            let gradient = ab_d / dist;
            position[i_a as usize] += w_a * d_y * gradient;
            position[i_b as usize] -= w_b * d_y * gradient;
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
