use ethel::state::data::{Column, ParallelIndexArrayColumn, column::IterColumn};
use physics::xpbd::{LinkNodes, LinksRowTable, NodesRowTable};

pub struct RotorSystem {
    /// Final computed rotations of nodes
    rotations: Vec<glam::Quat>,

    /// Mapping between node handle to internal storage handles
    node_map: Vec<RotorHandle>,

    //todo
    relatives: ParallelIndexArrayColumn<Vec<glam::Vec3>>,
    basis: ParallelIndexArrayColumn<Vec<glam::Vec3>>,
}

impl RotorSystem {
    pub fn clear_relatives(&mut self) {
        self.relatives.iter_mut().for_each(Vec::clear);
    }

    pub fn recompute_basis_cache(
        &mut self,
        nodes: &NodesRowTable,
        constraints: &LinksRowTable,
        overwrite: bool,
    ) {
        if overwrite {
            self.basis.clear();
        }

        for LinkNodes(node_a, node_b) in constraints.relation_view() {
            let rot_a = self.node_rotors_handle(*node_a).basis;
            let rot_b = self.node_rotors_handle(*node_b).basis;

            let i_a = unsafe { nodes.get_indirect_unchecked(*node_a) };
            let i_b = unsafe { nodes.get_indirect_unchecked(*node_b) };

            let pos_a = nodes.current_pos_slice()[i_a as usize];
            let pos_b = nodes.current_pos_slice()[i_b as usize];

            let ci_a = unsafe { self.basis.get_indirect_unchecked(rot_a) };
            let ci_b = unsafe { self.basis.get_indirect_unchecked(rot_b) };

            let basis_a = (pos_b - pos_a).normalize();
            let basis_b = -basis_a;

            self.basis.contiguous_mut()[ci_a as usize].push(basis_a);
            self.basis.contiguous_mut()[ci_b as usize].push(basis_b);
        }
    }

    pub fn recompute_relatives(&mut self, nodes: &NodesRowTable, constraints: &LinksRowTable) {
        self.clear_relatives();

        for LinkNodes(node_a, node_b) in constraints.relation_view() {
            let rot_a = self.node_rotors_handle(*node_a).relative;
            let rot_b = self.node_rotors_handle(*node_b).relative;

            let i_a = unsafe { nodes.get_indirect_unchecked(*node_a) };
            let i_b = unsafe { nodes.get_indirect_unchecked(*node_b) };

            let pos_a = nodes.current_pos_slice()[i_a as usize];
            let pos_b = nodes.current_pos_slice()[i_b as usize];

            let ci_a = unsafe { self.relatives.get_indirect_unchecked(rot_a) };
            let ci_b = unsafe { self.relatives.get_indirect_unchecked(rot_b) };

            let relative_a = (pos_b - pos_a).normalize();
            let relative_b = -relative_a;

            self.relatives.contiguous_mut()[ci_a as usize].push(relative_a);
            self.relatives.contiguous_mut()[ci_b as usize].push(relative_b);
        }
    }

    pub fn recompute_rotations(&mut self, nodes: &NodesRowTable) {
        self.rotations.clear();
        for handle in nodes.handles_view() {
            let rotor = self.node_rotors_handle(*handle);
            if let Some(basis_id) = self.basis.get_indirect(rotor.basis) {
                let basis = &self.basis.contiguous()[basis_id as usize];

                // SAFETY: relatives are computed every frame before computing
                // rotations.
                let relatives_id = unsafe { self.relatives.get_indirect_unchecked(rotor.relative) };
                let relatives = &self.relatives.contiguous()[relatives_id as usize];

                let mut rot = glam::Quat::IDENTITY;
                basis.iter().zip(relatives).for_each(|(&basis, &rel)| {
                    rot = rot.mul_quat(glam::Quat::from_rotation_arc(basis, rel));
                });
                self.rotations.push(rot);
            }
        }
    }

    /// Get the stable handle for the internal rotors data for `node_id`.
    pub fn node_rotors_handle(&mut self, node_id: u32) -> RotorHandle {
        let index = node_id as usize;

        if self.node_map.len() <= index {
            self.node_map.resize(index, RotorHandle::default());
        }
        let map = &mut self.node_map[index];

        if map.basis == 0 {
            map.basis = self.basis.put(Vec::new());
        }
        if map.relative == 0 {
            map.relative = self.relatives.put(Vec::new());
        }
        *map
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RotorHandle {
    pub basis: u32,
    pub relative: u32,
}
