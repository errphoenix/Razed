use ethel::state::data::Column;
use janus::context::DeltaTime;

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
            let i_a = node_data.get_indirect_unchecked(ab.0);
            let i_b = node_data.get_indirect_unchecked(ab.1);
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
