use ethel::state::data::{Column, IndirectIndex};
use janus::context::DeltaTime;
use physics::xpbd::{Constraints, HasConstraints, HasNodes, Nodes, RawXpbdLattice, XpbdSolver};
use rustc_hash::FxHashSet;

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

ethel::table_spec! {
    struct Links {
        relation: [IndirectIndex; 2];
        compliance: f32;
        rest_length: f32;
        lambda: f32;
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

    pub fn register_dead_nodes(&mut self) {
        self.damaged_nodes_data.clear();
        self.damaged_nodes_hash.clear();

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

    #[inline]
    pub fn update(&mut self, delta: DeltaTime) {
        // todo: perf telemetry
        self.solver.set_step_time(delta);
        self.solver.step(&mut self.nodes, &mut self.links);
    }

    /// Break a `constraint` by its handle.
    #[inline]
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
        let (_, _, m, _, f, _) = self.nodes_mut().split_mut();
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
            let handle =
                self.nodes
                    .insert((pos, pos, mass, inv_mass, glam::Vec3::ZERO, glam::Vec3::ZERO));
            self.node_id_buffer.push(handle);
        });
        lattice
            .constraints
            .iter()
            .for_each(|(&[a, b], (&compliance, &rest_length))| {
                let a = self.node_id_buffer[a as usize];
                let b = self.node_id_buffer[b as usize];
                self.links.insert(([a, b], compliance, rest_length, 0.0f32));
            });

        self.node_id_buffer.clear();
    }
}
