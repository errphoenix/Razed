use ethel::state::data::{Column, IndirectIndex};
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

    /// Consume the current lattice structure configuration to SoA storage.
    ///
    /// The node IDs used in constraints are the indices used in [`NodeSoA`].
    /// If the nodes are to be moved to another data structure, these indices
    /// must be re-mapped.
    pub fn build(mut self) -> RawXpbdLattice {
        let mut nodes = RawNodes::with_capacity(self.nodes.len());
        let mut constraints = RawConstraints::with_capacity(self.links.len());

        self.nodes.drain(..).for_each(|node| {
            let pos = node.pos;
            let mass = node.mass;
            let inv_mass = if node.fixed { 0.0 } else { 1.0 / mass };

            nodes.add(pos, mass, inv_mass);
        });

        self.links.drain(..).for_each(|constraint| {
            let a = constraint.node_a;
            let b = constraint.node_b;
            let compliance = constraint.options.compliance;
            let rest_length = constraint.options.rest_length.unwrap_or_else(|| {
                let p_a = nodes.positions[a as usize];
                let p_b = nodes.positions[b as usize];
                p_a.distance(p_b)
            });

            constraints.add([a, b], compliance, rest_length);
        });

        RawXpbdLattice { nodes, constraints }
    }
}

#[derive(Clone, Debug, Default)]
pub struct RawNodes {
    pub positions: Vec<glam::Vec3>,
    pub masses: Vec<f32>,
    pub inv_masses: Vec<f32>,
}

impl RawNodes {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            positions: Vec::with_capacity(capacity),
            masses: Vec::with_capacity(capacity),
            inv_masses: Vec::with_capacity(capacity),
        }
    }

    pub fn add(&mut self, pos: glam::Vec3, phys_mass: f32, inv_mass: f32) {
        self.positions.push(pos);
        self.masses.push(phys_mass);
        self.inv_masses.push(inv_mass);
    }

    pub fn len(&self) -> usize {
        self.positions.len()
    }

    pub fn iter(
        &self,
    ) -> std::iter::Zip<
        std::slice::Iter<'_, glam::Vec3>,
        std::iter::Zip<std::slice::Iter<'_, f32>, std::slice::Iter<'_, f32>>,
    > {
        self.positions
            .iter()
            .zip(self.masses.iter().zip(&self.inv_masses))
    }
}

#[derive(Clone, Debug, Default)]
pub struct RawConstraints {
    pub node_ids: Vec<[u32; 2]>,
    pub compliances: Vec<f32>,
    pub rest_lengths: Vec<f32>,
}

impl RawConstraints {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            node_ids: Vec::with_capacity(capacity),
            compliances: Vec::with_capacity(capacity),
            rest_lengths: Vec::with_capacity(capacity),
        }
    }

    pub fn add(&mut self, nodes: [u32; 2], compliance: f32, rest_length: f32) {
        self.node_ids.push(nodes);
        self.compliances.push(compliance);
        self.rest_lengths.push(rest_length);
    }

    pub fn len(&self) -> usize {
        self.node_ids.len()
    }

    pub fn iter(
        &self,
    ) -> std::iter::Zip<
        std::slice::Iter<'_, [u32; 2]>,
        std::iter::Zip<std::slice::Iter<'_, f32>, std::slice::Iter<'_, f32>>,
    > {
        self.node_ids
            .iter()
            .zip(self.compliances.iter().zip(&self.rest_lengths))
    }
}

#[derive(Clone, Debug, Default)]
pub struct RawXpbdLattice {
    pub nodes: RawNodes,
    pub constraints: RawConstraints,
}

pub const DEFAULT_SOLVE_ITERATIONS: u32 = 8;
pub const DEFAULT_SUB_STEPS: u32 = 4;
pub const DAMPING: f32 = 0.9985;

/// Indicates that a data table constrains node SoA data.
pub trait HasNodes {
    fn nodes(&mut self) -> Nodes<'_>;
}

/// Bridge-trait for a node-compatible data table that implements [`HasNodes`].
///
/// This exposes access to [`free`](Column::free), [`insert`](Column::insert),
/// and [`solve`](Column::solve_indirect) [`Column`] operations.
///
/// Direct access to the underlying SoA data is allowed by [`HasNodes::nodes`].
pub trait NodeColumn<Def: Default>: Column<Def> + HasNodes {}

impl<Def: Default, T: Column<Def> + HasNodes> NodeColumn<Def> for T {}

/// Indicates that a data table constrains constraint SoA data.
pub trait HasConstraints {
    fn constraints(&mut self) -> Constraints<'_>;
}

/// Bridge-trait for a constraint-compatible data table that implements
/// [`HasConstraints`].
///
/// This exposes access to [`free`](Column::free), [`insert`](Column::insert),
/// and [`solve`](Column::solve_indirect) [`Column`] operations.
///
/// Direct access to the underlying SoA data is allowed by [`HasConstraints::constraints`].
pub trait ConstraintColumn<Def: Default>: Column<Def> + HasConstraints {}

impl<Def: Default, T: Column<Def> + HasConstraints> ConstraintColumn<Def> for T {}

/// Compatibility data structure for raw node data.
#[derive(Debug)]
pub struct Nodes<'a> {
    pub proj_pos: &'a mut [glam::Vec3],
    pub live_pos: &'a mut [glam::Vec3],
    pub inv_masses: &'a [f32],
    pub forces: &'a mut [glam::Vec3],
    pub velocities: &'a mut [glam::Vec3],
    pub handles: &'a [IndirectIndex],
}

impl Nodes<'_> {
    pub fn len(&self) -> usize {
        self.handles.len()
    }
}

/// Compatibility data structure for raw constraint data.
#[derive(Debug)]
pub struct Constraints<'a> {
    pub relations: &'a [[IndirectIndex; 2]],
    pub compliances: &'a [f32],
    pub rest_lengths: &'a [f32],
    pub effective_masses: &'a [f32],
    pub integrities: &'a [f32],
    pub lambdas: &'a mut [f32],
    pub handles: &'a [IndirectIndex],
}

impl Constraints<'_> {
    pub fn len(&self) -> usize {
        self.handles.len()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct XpbdSolver {
    iterations: u32,
    substeps: u32,
    h: f32,
    h2: f32,
    step_multiplier: f32,

    allow_breaking: bool,
    clear_degenerate_nodes: bool,
    ground_level: Option<f32>,

    broken_links: Vec<IndirectIndex>,
    degenerate_nodes: Vec<IndirectIndex>,

    /// Maps a node indirect index (stable) to number of active constraints
    frame_constraint_map: Vec<u32>,
}

impl Default for XpbdSolver {
    fn default() -> Self {
        Self {
            h: 0.0,
            h2: 0.0,
            iterations: DEFAULT_SOLVE_ITERATIONS,
            substeps: DEFAULT_SUB_STEPS,
            step_multiplier: DEFAULT_STEP_MULT,

            allow_breaking: true,
            clear_degenerate_nodes: true,
            ground_level: None,

            broken_links: Vec::with_capacity(32),
            degenerate_nodes: Vec::with_capacity(32),

            frame_constraint_map: Vec::new(),
        }
    }
}

pub const DEFAULT_STEP_MULT: f32 = 1.2;

#[derive(Clone, Copy, Debug)]
pub struct XpbdOptions {
    pub iterations: u32,
    pub substeps: u32,
    pub step_multiplier: f32,
    pub allow_breaking: bool,
    pub clear_degenerate_nodes: bool,
    pub ground_level: Option<f32>,
}

impl XpbdOptions {
    pub const fn new(
        iterations: u32,
        substeps: u32,
        step_multiplier: f32,
        allow_breaking: bool,
        clear_degenerate_nodes: bool,
        ground_level: Option<f32>,
    ) -> Self {
        Self {
            iterations,
            substeps,
            step_multiplier,
            allow_breaking,
            clear_degenerate_nodes,
            ground_level,
        }
    }

    pub const fn with_iterations(self, iterations: u32) -> Self {
        Self {
            iterations,
            step_multiplier: self.step_multiplier,
            substeps: self.substeps,
            allow_breaking: self.allow_breaking,
            clear_degenerate_nodes: self.clear_degenerate_nodes,
            ground_level: self.ground_level,
        }
    }

    pub const fn with_substeps(self, substeps: u32) -> Self {
        Self {
            substeps,
            iterations: self.iterations,
            step_multiplier: self.step_multiplier,
            allow_breaking: self.allow_breaking,
            clear_degenerate_nodes: self.clear_degenerate_nodes,
            ground_level: self.ground_level,
        }
    }

    pub const fn with_breaking(self, breaking: bool) -> Self {
        Self {
            allow_breaking: breaking,
            iterations: self.iterations,
            substeps: self.substeps,
            step_multiplier: self.step_multiplier,
            clear_degenerate_nodes: self.clear_degenerate_nodes,
            ground_level: self.ground_level,
        }
    }

    pub const fn with_ground_level(self, ground_level: Option<f32>) -> Self {
        Self {
            ground_level,
            iterations: self.iterations,
            substeps: self.substeps,
            step_multiplier: self.step_multiplier,
            allow_breaking: self.allow_breaking,
            clear_degenerate_nodes: self.clear_degenerate_nodes,
        }
    }

    pub const fn with_step_multiplier(self, step_multiplier: f32) -> Self {
        Self {
            step_multiplier,
            iterations: self.iterations,
            substeps: self.substeps,
            allow_breaking: self.allow_breaking,
            clear_degenerate_nodes: self.clear_degenerate_nodes,
            ground_level: self.ground_level,
        }
    }
}

impl Default for XpbdOptions {
    fn default() -> Self {
        Self {
            iterations: DEFAULT_SOLVE_ITERATIONS,
            substeps: DEFAULT_SUB_STEPS,
            step_multiplier: DEFAULT_STEP_MULT,
            allow_breaking: true,
            clear_degenerate_nodes: true,
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
            step_multiplier: options.step_multiplier,

            allow_breaking: options.allow_breaking,
            clear_degenerate_nodes: options.clear_degenerate_nodes,
            ground_level: options.ground_level,

            broken_links: Vec::with_capacity(32 * options.allow_breaking as usize),
            degenerate_nodes: Vec::with_capacity(32 * options.clear_degenerate_nodes as usize),

            frame_constraint_map: Vec::new(),
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
        self.h = delta.as_f32() / self.substeps as f32 * self.step_multiplier;
        self.h2 = self.h * self.h;
    }

    /// Break a link by its ID.
    ///
    /// # Panics
    /// Will panic:
    /// * If `link_id` is an invalid constraint handle.
    /// * If the XPBD solver's `allow_breaking` flag is `false`.
    pub fn break_link(&mut self, link_id: IndirectIndex) {
        assert!(
            self.allow_breaking,
            "cannot query broken links: allow_breaking flag for XPBD is set to false"
        );

        self.broken_links.push(link_id);
    }

    /// Returns a slice over the degenerate nodes computed during the last
    /// step.
    ///
    /// This is reset at the beginning of every step. Degenerate nodes IDs
    /// are accumulated every sub-step.
    ///
    /// # Panics
    /// Will panic if the XPBD solver's `clear_degenerate_flag` flag is `false`.
    pub fn degenerate_nodes(&self) -> &[IndirectIndex] {
        assert!(
            self.clear_degenerate_nodes,
            "cannot query degenerate nodes: clear_degenerate_nodes flag for XPBD is set to false"
        );

        &self.degenerate_nodes
    }

    /// Returns a slice over the constraint IDs that were broken in the last
    /// step.
    ///
    /// This is reset at the beginning of every step. Broken constraint IDs
    /// are accumulated every sub-step.
    ///
    /// # Panics
    /// Will panic if the XPBD solver's `allow_breaking` flag is `false`.
    pub fn broken_links(&self) -> &[IndirectIndex] {
        assert!(
            self.allow_breaking,
            "cannot query broken links: allow_breaking flag for XPBD is set to false"
        );

        &self.broken_links
    }

    #[inline]
    pub fn step<ND: Default, CD: Default>(
        &mut self,
        node_table: &mut impl NodeColumn<ND>,
        constraint_table: &mut impl ConstraintColumn<CD>,
    ) {
        if self.clear_degenerate_nodes {
            self.degenerate_nodes.clear();

            let relations = constraint_table.constraints().relations;
            let node_count = node_table.size();

            self.frame_constraint_map.fill(0u32);
            self.frame_constraint_map.resize(node_count, 0u32);

            for [a, b] in relations {
                self.frame_constraint_map[a.as_index()] += 1;
                self.frame_constraint_map[b.as_index()] += 1;
            }
        }

        if self.allow_breaking {
            self.broken_links.iter().for_each(|&handle| {
                if self.clear_degenerate_nodes {
                    if let Some(id) = constraint_table.solve_indirect(handle) {
                        let [a, b] = constraint_table.constraints().relations[id.as_index()];

                        // delete node if this deleted constraint was their last
                        // else, only keep tracking count: constraints are only
                        // recounted at the start of the next step, but multiple
                        // constraints to the same node might break in one step.
                        {
                            let ca = unsafe {
                                self.frame_constraint_map.get_unchecked_mut(a.as_index())
                            };
                            if *ca == 1 {
                                // nodes.free(a);
                                self.degenerate_nodes.push(a);
                            } else {
                                *ca -= 1;
                            }
                            let cb = unsafe {
                                self.frame_constraint_map.get_unchecked_mut(b.as_index())
                            };
                            if *cb == 1 {
                                // nodes.free(b);
                                self.degenerate_nodes.push(b);
                            } else {
                                *cb -= 1;
                            }
                        }
                    }
                }

                constraint_table.free(handle);
            });

            // clear last frame, this is only done at this point to allow
            // external systems to act on accumulated broken links
            self.broken_links.clear();

            let constraints = constraint_table.constraints();
            let handles = constraints.handles;
            let lambdas = constraints.lambdas;
            let compliances = constraints.compliances;
            let eff_masses = constraints.effective_masses;
            let integrities = constraints.integrities;

            for (((handle, lambda), (&eff_mass, &integrity)), _a) in handles
                .iter()
                .zip(lambdas)
                .zip(eff_masses.iter().zip(integrities))
                .zip(compliances)
                .skip(1)
            {
                const BREAK_THRESHOLD: f32 = 220.0;

                let force_strain = (*lambda / self.h2) * eff_mass;
                let threshold = integrity * BREAK_THRESHOLD;
                let compression_threshold = integrity.exp() - 1.0;

                if force_strain > threshold
                    || force_strain < -compression_threshold * BREAK_THRESHOLD
                {
                    self.broken_links.push(*handle);
                }
            }
        }

        for _ in 0..self.substeps {
            self.substep(node_table, constraint_table);
        }
        for v in node_table.nodes().velocities {
            *v *= DAMPING;
        }
    }

    #[inline]
    fn substep<ND: Default, CD: Default>(
        &mut self,
        node_table: &mut impl NodeColumn<ND>,
        constraint_table: &mut impl ConstraintColumn<CD>,
    ) {
        self.predict_positions(node_table.nodes());
        if self.ground_level.is_some() {
            self.apply_ground_constraint(node_table.nodes());
        }

        constraint_table.constraints().lambdas.fill(0.0);
        for _ in 0..self.iterations {
            self.solve_constraints(node_table, constraint_table.constraints());
        }
        self.finalise_nodes(node_table.nodes());
    }

    #[inline]
    fn predict_positions(&self, nodes: Nodes<'_>) {
        let node_count = nodes.len();
        let p_pos = nodes.proj_pos;
        let c_pos = nodes.live_pos;
        let inv_mass = nodes.inv_masses;
        let forces = nodes.forces;
        let velocity = nodes.velocities;

        for i in 1..node_count {
            let x = c_pos[i];
            let f = std::mem::take(&mut forces[i]);
            let v = velocity[i];
            let w = inv_mass[i];

            let p = &mut p_pos[i];

            *p = x + self.h * v + self.h2 * f * w;
        }
    }

    #[inline]
    fn solve_constraints<ND: Default>(
        &self,
        node_table: &mut impl NodeColumn<ND>,
        constraints: Constraints<'_>,
    ) {
        let relations = constraints.relations;
        let compliances = constraints.compliances;
        let rest_lengths = constraints.rest_lengths;
        let lambdas = constraints.lambdas;

        for (((ab, inv_stiffness), l), y) in relations
            .iter()
            .zip(compliances)
            .zip(rest_lengths)
            .zip(lambdas)
        {
            let i_a = node_table.solve_indirect(ab[0]).unwrap();
            let i_b = node_table.solve_indirect(ab[1]).unwrap();

            let nodes = node_table.nodes();
            let inv_mass = nodes.inv_masses;
            let position = nodes.proj_pos;

            let w_a = inv_mass[i_a.as_index()];
            let w_b = inv_mass[i_b.as_index()];

            let p_a = position[i_a.as_index()];
            let p_b = position[i_b.as_index()];

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
            position[i_a.as_index()] += w_a * d_y * gradient;
            position[i_b.as_index()] -= w_b * d_y * gradient;
        }
    }

    #[inline]
    fn apply_ground_constraint(&self, nodes: Nodes<'_>) {
        const RESTITUTION: f32 = 0.2;
        const FRICTION: f32 = 0.4;

        let ground_level = self.ground_level.unwrap_or_default();

        let proj_pos = nodes.proj_pos;
        let live_pos = nodes.live_pos;
        let forces = nodes.forces;
        let velocities = nodes.velocities;

        for ((n_pos, c_pos), (_forces, vel)) in proj_pos
            .iter_mut()
            .zip(live_pos)
            .zip(forces.iter_mut().zip(velocities))
        {
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
    fn finalise_nodes(&self, nodes: Nodes<'_>) {
        let proj_pos = nodes.proj_pos;
        let live_pos = nodes.live_pos;
        let velocities = nodes.velocities;

        for ((p, x), v) in proj_pos.iter().zip(live_pos).zip(velocities) {
            *v = (*p - *x) / self.h;
            *x = *p;
        }
    }
}
