use std::f32;

use ethel::state::data::{Column, IndirectIndex};
use janus::context::DeltaTime;
use physics::xpbd::{Constraints, HasConstraints, HasNodes, Nodes, RawXpbdLattice, XpbdSolver};
use rustc_hash::FxHashSet;

use crate::structure::FragmentsRowTableView;

ethel::table_spec! {
    struct Nodes {
        predicted_pos: glam::Vec3;
        current_pos: glam::Vec3;

        mass: f32;
        inv_mass: f32;
        forces: glam::Vec3;
        velocity: glam::Vec3;

        covariant: glam::Mat3;
        rotation_ex: glam::Mat3;
    }
}

ethel::table_spec! {
    struct Links {
        relation: [IndirectIndex; 2];

        b_edge: glam::Vec3;
        edge: glam::Vec3;

        compliance: f32;
        rest_length: f32;
        lambda: f32;

        stable_state: f32;
        effective_mass: f32;
        integrity: f32;
    }
}

impl HasNodes for NodesRowTable {
    fn nodes(&mut self) -> physics::xpbd::Nodes<'_> {
        Nodes {
            proj_pos: &mut self.predicted_pos,
            live_pos: &mut self.current_pos,
            inv_masses: &self.inv_mass,
            forces: &mut self.forces,
            velocities: &mut self.velocity,
            handles: &self.handles,
        }
    }
}

impl HasConstraints for LinksRowTable {
    fn constraints(&mut self) -> physics::xpbd::Constraints<'_> {
        Constraints {
            relations: &self.relation,
            compliances: &self.compliance,
            rest_lengths: &self.rest_length,
            effective_masses: &self.effective_mass,
            integrities: &self.integrity,
            lambdas: &mut self.lambda,
            handles: &self.handles,
        }
    }
}

#[derive(Debug, Default)]
pub struct LatticeSystem {
    node_id_buffer: Vec<IndirectIndex>,

    nodes: NodesRowTable,
    links: LinksRowTable,

    solver: XpbdSolver,

    /// alltime accumulated set of dead node IDs; hashing avoids dedup op
    damaged_nodes_data: Vec<IndirectIndex>,
    damaged_nodes_hash: FxHashSet<IndirectIndex>,
}

impl LatticeSystem {
    pub fn new(solver: XpbdSolver) -> Self {
        Self {
            solver,
            ..Default::default()
        }
    }

    pub fn with_capacity(solver: XpbdSolver, capacity: usize) -> Self {
        Self {
            solver,
            nodes: NodesRowTable::with_capacity(capacity),
            links: LinksRowTable::with_capacity(capacity),
            ..Default::default()
        }
    }

    pub fn with_data(solver: XpbdSolver, nodes: NodesRowTable, links: LinksRowTable) -> Self {
        Self {
            solver,
            nodes,
            links,
            ..Default::default()
        }
    }

    /// Returns a slice of disabled node IDs during this frame.
    ///
    /// No duplicates are present. All entries are unique for **this** frame.
    ///
    /// Requires a prior call to [`Self::register_dead_nodes`] during the
    /// same frame.
    pub fn unique_damaged_nodes_frame(&self) -> &[IndirectIndex] {
        &self.damaged_nodes_data
    }

    pub fn clear_damage_buffers(&mut self) {
        self.damaged_nodes_data.clear();
        self.damaged_nodes_hash.clear();
    }

    pub fn compute_edges(&mut self) {
        let links = &self.links.relation;
        let edges = &mut self.links.edge;
        let nodes = &self.nodes.current_pos;

        for (&[a, b], edge) in links.iter().zip(edges) {
            let di_a = self
                .nodes
                .solve_indirect(a)
                .expect("registered node is always present");
            let di_b = self
                .nodes
                .solve_indirect(b)
                .expect("registered node is always present");
            let p_a = nodes[di_a.as_index()];
            let p_b = nodes[di_b.as_index()];

            let e_ab = p_b - p_a;
            *edge = e_ab;
        }
    }

    pub fn compute_covariances(&mut self) {
        fn outer_product(a: glam::Vec3, b: glam::Vec3) -> glam::Mat3 {
            glam::Mat3::from_cols(a * b.x, a * b.y, a * b.z)
        }

        let links = &self.links.relation;
        let bind_edges = &self.links.b_edge;
        let edges = &self.links.edge;

        self.nodes.covariant.fill(glam::Mat3::ZERO);
        for (&[a, b], (&bind_edge, &edge)) in links.iter().zip(bind_edges.iter().zip(edges)) {
            let di_a = self
                .nodes
                .solve_indirect(a)
                .expect("registered node is always present");
            let di_b = self
                .nodes
                .solve_indirect(b)
                .expect("registered node is always present");

            {
                let cov_a = &mut self.nodes.covariant[di_a.as_index()];
                let o_dot_a = outer_product(edge, bind_edge);
                *cov_a += o_dot_a;
            }
            {
                let cov_b = &mut self.nodes.covariant[di_b.as_index()];
                let o_dot_b = outer_product(-edge, -bind_edge);
                *cov_b += o_dot_b;
            }
        }
    }

    pub fn extract_node_rotations(&mut self, iterations: usize) {
        let covariants = &self.nodes.covariant;
        let rot_extracts = &mut self.nodes.rotation_ex;

        'cov: for (&cov, rot) in covariants.iter().zip(rot_extracts) {
            let mut r = cov;
            for _ in 0..iterations {
                let ri = r.try_inverse();
                if ri.is_none() {
                    continue 'cov;
                }
                let ri = ri.unwrap();
                r = (r + ri.transpose()) * 0.5;
            }
            *rot = r;
        }
    }

    pub fn register_dead_nodes(&mut self) {
        for id in self.solver.broken_links() {
            let index = self
                .links()
                .solve_indirect(*id)
                .expect("broken link id is always valid");

            let [node_a, node_b] = *unsafe {
                self.links()
                    .relation_slice()
                    .get_unchecked(index.as_index())
            };
            if self.damaged_nodes_hash.insert(node_a) {
                self.damaged_nodes_data.push(node_a);
            }
            if self.damaged_nodes_hash.insert(node_b) {
                self.damaged_nodes_data.push(node_b);
            }
        }
    }

    pub fn pull_integrity_mass(&mut self, fragments: &FragmentsRowTableView) {
        self.nodes.mass.fill(0.0);

        let parents = fragments.parents;
        let weights = fragments.parents_weights;
        let integrity = fragments.integrity;
        let mass_coeff = fragments.health_coeff;

        for ((ids, weights), (&hp, &coeff)) in parents
            .iter()
            .zip(weights)
            .zip(integrity.iter().zip(mass_coeff))
            .skip(1)
        {
            let hp_mass_coeff = hp * coeff;
            for (id, &w) in ids.iter().zip(weights) {
                if id.as_int() == 0 {
                    continue;
                }

                if let Some(index) = self.nodes.solve_indirect(*id) {
                    let weighted_mass = w * hp_mass_coeff;
                    self.nodes.mass[index.as_index()] += weighted_mass;
                }
            }
        }

        let handles = &self.nodes.handles;
        let phys_masses = &self.nodes.mass;
        let inv_masses = &mut self.nodes.inv_mass;

        phys_masses
            .iter()
            .zip(inv_masses.iter_mut())
            .zip(handles)
            .skip(1)
            .for_each(|((m, m_inv), &id)| {
                // this is an anchor node that must not have mass
                if *m_inv == 0.0 {
                    return;
                }

                if *m < f32::EPSILON {
                    if self.damaged_nodes_hash.insert(id) {
                        self.damaged_nodes_data.push(id);
                    }
                    return;
                }
                *m_inv = 1.0 / *m
            });
    }

    pub fn sync_constraint_attributes(&mut self) {
        let node_inv_masses = &self.nodes.inv_mass;
        let node_masses = &self.nodes.mass;

        let nodes = &self.links.relation;
        let eff_masses = &mut self.links.effective_mass;
        let integrities = &mut self.links.integrity;
        let stables = &mut self.links.stable_state;

        for ([a, b], ((eff_mass, integrity), stable)) in nodes
            .iter()
            .zip(eff_masses.iter_mut().zip(integrities).zip(stables))
            .skip(1)
        {
            let i_a = self.nodes.solve_indirect(*a).unwrap();
            let i_b = self.nodes.solve_indirect(*b).unwrap();
            let m_a = node_inv_masses[i_a.as_index()];
            let m_b = node_inv_masses[i_b.as_index()];
            let pm_a = node_masses[i_a.as_index()];
            let pm_b = node_masses[i_b.as_index()];

            *eff_mass = m_a + m_b;

            let res = (pm_a + pm_b) * 0.5;
            if *stable == 0.0 {
                *stable = res;
                *integrity = 1.0;
            } else {
                *integrity = res / *stable;
            }
        }
    }

    pub fn update(&mut self, delta: DeltaTime) {
        self.solver.set_step_time(delta);
        self.solver.step(&mut self.nodes, &mut self.links);
    }

    /// Break a `constraint` by its handle.
    pub fn break_constraint(&mut self, constraint: IndirectIndex) {
        if self.links.solve_indirect(constraint).is_some() {
            self.solver.break_link(constraint);
        }
    }

    #[inline]
    pub fn apply_forces(&mut self, index: IndirectIndex, force: glam::Vec3) {
        if let Some(node) = self.nodes.solve_indirect(index) {
            let mass = *unsafe { self.nodes.mass_slice().get_unchecked(node.as_index()) };
            let f = unsafe {
                self.nodes
                    .forces_mut_slice()
                    .get_unchecked_mut(node.as_index())
            };
            *f += force * mass;
        }
    }

    #[inline]
    pub fn apply_forces_multi(&mut self, indices: &[IndirectIndex], force: glam::Vec3) {
        for &index in indices {
            self.apply_forces(index, force);
        }
    }

    #[inline]
    pub fn apply_forces_batched(&mut self, force: glam::Vec3) {
        let (_, _, m, _, f, _, _, _) = self.nodes_mut().split_mut();
        for (f, m) in f.join(m) {
            *f += force * *m;
        }
    }

    #[inline]
    pub fn nodes(&self) -> &NodesRowTable {
        &self.nodes
    }

    #[inline]
    pub fn links(&self) -> &LinksRowTable {
        &self.links
    }

    #[inline]
    pub fn nodes_mut(&mut self) -> &mut NodesRowTable {
        &mut self.nodes
    }

    #[inline]
    pub fn links_mut(&mut self) -> &mut LinksRowTable {
        &mut self.links
    }

    #[inline]
    pub fn nodes_links_mut(&mut self) -> (&mut NodesRowTable, &mut LinksRowTable) {
        (&mut self.nodes, &mut self.links)
    }

    /// See [`physics::xpbd::XpbdSolver::broken_links`].
    #[inline]
    pub fn frame_broken_links(&self) -> &[IndirectIndex] {
        self.solver.broken_links()
    }

    /// See [`physics::xpbd::XpbdSolver::degenerate_nodes`].
    #[inline]
    pub fn frame_degenerate_nodes(&self) -> &[IndirectIndex] {
        self.solver.degenerate_nodes()
    }

    #[inline]
    pub fn import_lattice(&mut self, lattice: RawXpbdLattice) {
        let node_count = lattice.nodes.len();
        let cd = node_count - self.node_id_buffer.capacity();
        if cd > 0 {
            self.node_id_buffer.reserve(cd);
        }

        lattice.nodes.iter().for_each(|(&pos, (&mass, &inv_mass))| {
            let handle = self.nodes.insert((
                pos,
                pos,
                mass,
                inv_mass,
                glam::Vec3::ZERO,
                glam::Vec3::ZERO,
                glam::Mat3::IDENTITY,
                glam::Mat3::IDENTITY,
            ));
            self.node_id_buffer.push(handle);
        });
        lattice
            .constraints
            .iter()
            .for_each(|(&[a, b], (&edge, (&compliance, &rest_length)))| {
                let a = self.node_id_buffer[a as usize];
                let b = self.node_id_buffer[b as usize];

                self.links.insert((
                    [a, b],
                    edge, // bind
                    edge, // current is same as bind at init
                    compliance,
                    rest_length,
                    0f32,
                    0f32,
                    0f32,
                    0f32,
                ));
            });

        self.node_id_buffer.clear();
    }
}
