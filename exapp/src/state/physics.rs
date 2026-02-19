use ethel::state::data::Column;
use janus::context::DeltaTime;
use physics::xpbd::{LinksRowTable, NodesRowTable, XpbdLatticeBuilder, XpbdSolver};

#[derive(Debug, Default)]
pub struct XpbdSystem {
    solver: XpbdSolver,
    nodes: NodesRowTable,
    links: LinksRowTable,
}

impl XpbdSystem {
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
        }
    }

    pub fn with_data(solver: XpbdSolver, nodes: NodesRowTable, links: LinksRowTable) -> Self {
        Self {
            solver,
            nodes,
            links,
        }
    }

    #[inline]
    pub fn update(&mut self, delta: DeltaTime) {
        // todo: perf telemetry
        self.solver.set_step_time(delta);
        self.solver.step(&mut self.nodes, &mut self.links);
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

    #[inline]
    pub fn frame_broken_links(&self) -> &[u32] {
        self.solver.broken_links()
    }

    #[inline]
    pub fn import_lattice(
        &mut self,
        lattice_builder: XpbdLatticeBuilder,
    ) -> physics::xpbd::LatticeIds {
        lattice_builder.export(&mut self.nodes, &mut self.links)
    }
}
