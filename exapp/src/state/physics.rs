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

    pub fn update(&mut self, delta: DeltaTime) {
        // todo: perf telemetry
        self.solver.set_step_time(delta);
        self.solver.step(&mut self.nodes, &mut self.links);
    }

    pub fn nodes(&self) -> &NodesRowTable {
        &self.nodes
    }

    pub fn links(&self) -> &LinksRowTable {
        &self.links
    }

    pub fn nodes_mut(&mut self) -> &mut NodesRowTable {
        &mut self.nodes
    }

    pub fn links_mut(&mut self) -> &mut LinksRowTable {
        &mut self.links
    }

    pub fn nodes_links_mut(&mut self) -> (&mut NodesRowTable, &mut LinksRowTable) {
        (&mut self.nodes, &mut self.links)
    }

    pub fn import_lattice(&mut self, lattice_builder: XpbdLatticeBuilder) {
        lattice_builder.export(&mut self.nodes, &mut self.links);
    }
}
