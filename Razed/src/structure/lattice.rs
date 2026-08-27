use std::f32;

use ethel::state::data::{Column, IndirectIndex};
use janus::context::DeltaTime;
use physics::xpbd::{Constraints, HasConstraints, HasNodes, Nodes, RawXpbdLattice, XpbdSolver};
use rustc_hash::FxHashMap;

use crate::structure::FragmentsRowTableView;

ethel::table_spec! {
    struct Nodes {
        predicted_pos: glam::Vec3;
        current_pos: glam::Vec3;

        mass: f32;
        inv_mass: f32;
        forces: glam::Vec3;
        velocity: glam::Vec3;

        constraint_count: u32;
    }
}

ethel::table_spec! {
    struct Links {
        relation: [IndirectIndex; 2];

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct DamagedNode {
    pub id: IndirectIndex,
    pub constraints_left: u32,
}

#[derive(Debug, Default)]
pub struct LatticeSystem {
    // temporary mapping for lattice import
    node_id_buffer: Vec<IndirectIndex>,

    nodes: NodesRowTable,
    links: LinksRowTable,
    solver: XpbdSolver,

    /// transient frame data for damaged nodes that lost
    /// any constraint
    damaged_nodes_data: Vec<DamagedNode>,
    // transient frame data for damaged nodes, also holds
    // the index into damages_nodes_data and tracks constraints
    // count for the node
    damaged_nodes_hash: FxHashMap<IndirectIndex, u32>,
}
impl LatticeSystem {
    pub fn new(solver: XpbdSolver) -> Self {
        Self {
            solver,
            ..Default::default()
        }
    }

    #[allow(unused)]
    pub fn with_capacity(solver: XpbdSolver, capacity: usize) -> Self {
        Self {
            solver,
            nodes: NodesRowTable::with_capacity(capacity),
            links: LinksRowTable::with_capacity(capacity),
            ..Default::default()
        }
    }

    #[allow(unused)]
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
    pub fn unique_damaged_nodes_frame(&self) -> &[DamagedNode] {
        &self.damaged_nodes_data
    }

    pub fn clear_damage_buffers(&mut self) {
        self.damaged_nodes_data.clear();
        self.damaged_nodes_hash.clear();
    }

    fn register_node_damage(
        vec: &mut Vec<DamagedNode>,
        hash: &mut FxHashMap<IndirectIndex, u32>,
        id: IndirectIndex,
        constraints_count: u32,
    ) {
        let i = *hash.entry(id).or_insert_with(|| {
            let i = vec.len();
            vec.push(DamagedNode {
                id,
                constraints_left: 0,
            });
            i as u32
        });
        vec[i as usize].constraints_left = constraints_count;
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

            let (n_a, n_b) = {
                let da = self.nodes.solve_indirect(node_a).unwrap();
                let db = self.nodes.solve_indirect(node_b).unwrap();

                let n_a = &mut self.nodes.constraint_count[da.as_index()];
                *n_a -= 1;
                let n_a = *n_a;
                let n_b = &mut self.nodes.constraint_count[db.as_index()];
                *n_b -= 1;
                let n_b = *n_b;

                (n_a, n_b)
            };

            let vec = &mut self.damaged_nodes_data;
            let hash = &mut self.damaged_nodes_hash;
            Self::register_node_damage(vec, hash, node_a, n_a);
            Self::register_node_damage(vec, hash, node_b, n_b);
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
        let constraints_count = &self.nodes.constraint_count;
        let inv_masses = &mut self.nodes.inv_mass;

        phys_masses
            .iter()
            .zip(inv_masses.iter_mut())
            .zip(handles)
            .zip(constraints_count)
            .skip(1)
            .for_each(|(((m, m_inv), &id), &c_count)| {
                // this is an anchor node that must not have mass
                if *m_inv == 0.0 {
                    return;
                }

                if *m < f32::EPSILON {
                    let vec = &mut self.damaged_nodes_data;
                    let hash = &mut self.damaged_nodes_hash;
                    Self::register_node_damage(vec, hash, id, c_count);
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
    #[allow(unused, reason = "wip feature")]
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
    #[allow(unused, reason = "wip feature")]
    pub fn apply_forces_multi(&mut self, indices: &[IndirectIndex], force: glam::Vec3) {
        for &index in indices {
            self.apply_forces(index, force);
        }
    }

    #[inline]
    pub fn apply_forces_batched(&mut self, force: glam::Vec3) {
        let (_, _, m, _, f, _, _) = self.nodes_mut().split_mut();
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
    #[allow(unused)]
    pub fn links_mut(&mut self) -> &mut LinksRowTable {
        &mut self.links
    }

    #[inline]
    #[allow(unused)]
    pub fn nodes_links_mut(&mut self) -> (&mut NodesRowTable, &mut LinksRowTable) {
        (&mut self.nodes, &mut self.links)
    }

    /// See [`physics::xpbd::XpbdSolver::broken_links`].
    #[inline]
    #[allow(unused)]
    pub fn frame_broken_links(&self) -> &[IndirectIndex] {
        self.solver.broken_links()
    }

    /// See [`physics::xpbd::XpbdSolver::degenerate_nodes`].
    #[inline]
    #[allow(unused)]
    pub fn frame_degenerate_nodes(&self) -> &[IndirectIndex] {
        self.solver.degenerate_nodes()
    }

    #[inline]
    pub fn import_lattice(&mut self, lattice: RawXpbdLattice) {
        let node_count = lattice.nodes.len();
        let cd = node_count.saturating_sub(self.node_id_buffer.capacity());
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
                0,
            ));
            self.node_id_buffer.push(handle);
        });
        lattice
            .constraints
            .iter()
            .for_each(|(&[a, b], (&compliance, &rest_length))| {
                let a = self.node_id_buffer[a as usize];
                let b = self.node_id_buffer[b as usize];

                let da = self.nodes.solve_indirect(a).unwrap();
                let db = self.nodes.solve_indirect(a).unwrap();
                self.nodes.constraint_count[da.as_index()] += 1;
                self.nodes.constraint_count[db.as_index()] += 1;

                self.links
                    .insert(([a, b], compliance, rest_length, 0f32, 0f32, 0f32, 0f32));
            });

        self.node_id_buffer.clear();
    }
}
