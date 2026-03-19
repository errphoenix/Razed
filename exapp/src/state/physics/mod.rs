use ethel::state::data::Column;
use janus::context::DeltaTime;
use physics::xpbd::{LinkNodes, LinksRowTable, NodesRowTable, XpbdLatticeBuilder, XpbdSolver};
use rustc_hash::FxHashSet;

#[derive(Debug, Default)]
pub struct LatticeSystem {
    nodes: NodesRowTable,
    links: LinksRowTable,

    solver: XpbdSolver,

    /// alltime accumulated set of dead node IDs; hashing avoids dedup op
    damaged_nodes_data: Vec<u32>,
    damaged_nodes_hash: FxHashSet<u32>,
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
    pub fn unique_damaged_nodes_frame(&self) -> &[u32] {
        &self.damaged_nodes_data
    }

    pub fn register_dead_nodes(&mut self) {
        self.damaged_nodes_data.clear();
        self.damaged_nodes_hash.clear();

        for id in self.solver.broken_links() {
            let LinkNodes(node_a, node_b) =
                *unsafe { self.links().relation_slice().get_unchecked(*id as usize) };
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
    pub fn break_constraint(&mut self, constraint: u32) {
        if self.links.get_indirect(constraint).is_some() {
            self.solver.break_link(constraint);
        }
    }

    #[inline]
    pub fn apply_forces(&mut self, index: u32, force: glam::Vec3) {
        if let Some(node) = self.nodes.get_indirect(index) {
            let mass = *unsafe { self.nodes.mass_slice().get_unchecked(node as usize) };
            let f = unsafe {
                self.nodes
                    .forces_mut_slice()
                    .get_unchecked_mut(node as usize)
            };
            *f += force * mass;
        }
    }

    #[inline]
    pub fn apply_forces_multi(&mut self, indices: &[u32], force: glam::Vec3) {
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
    pub fn frame_broken_links(&self) -> &[u32] {
        self.solver.broken_links()
    }

    /// See [`physics::xpbd::XpbdSolver::degenerate_nodes`].
    #[inline]
    pub fn frame_degenerate_nodes(&self) -> &[u32] {
        self.solver.degenerate_nodes()
    }

    #[inline]
    pub fn import_lattice(
        &mut self,
        lattice_builder: XpbdLatticeBuilder,
    ) -> physics::xpbd::LatticeIds {
        lattice_builder.export(&mut self.nodes, &mut self.links)
    }
}
