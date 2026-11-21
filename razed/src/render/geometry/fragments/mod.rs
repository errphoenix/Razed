pub mod deformed;
pub mod instanced;

pub use deformed::*;
use ethel::state::data::table::TableView;
pub use instanced::*;

use crate::{
    data::{DebrisPartitionedTriBuffer, FragmentsPartitionedTriBuffer},
    simulation::{cage::CageSystem, fragment::FragmentsRowTableView},
};

use super::*;

#[derive(Debug)]
pub enum FragmentsInstancedData<'data> {
    Debris(&'data DebrisPartitionedTriBuffer),
    Fragments(&'data FragmentsPartitionedTriBuffer),
}

#[derive(Debug, Clone)]
pub struct FragmentDomains {
    // both index in fragments arrays
    ids_instanced: Vec<u32>,
    ids_deforming: Vec<u32>,

    deform_threshold: f32,
}
impl Default for FragmentDomains {
    fn default() -> Self {
        Self {
            ids_instanced: Vec::new(),
            ids_deforming: Vec::new(),
            deform_threshold: Self::DEFAULT_DEFORM_THRESHOLD,
        }
    }
}
impl FragmentDomains {
    pub const DEFAULT_DEFORM_THRESHOLD: f32 = 0.01;

    pub fn new(deform_threshold: f32) -> Self {
        Self {
            deform_threshold,
            ..Default::default()
        }
    }

    pub fn map_domains(&mut self, fragments: &FragmentsRowTableView, cage_sys: &CageSystem) {
        self.ids_instanced.clear();
        self.ids_deforming.clear();

        for i in 1..fragments.len() {
            let cage_id = fragments.deformation_cage[i];
            match cage_sys.get_delta(cage_id) {
                Some(delta) => {
                    if delta <= self.deform_threshold {
                        self.ids_instanced.push(i as u32);
                    } else {
                        self.ids_deforming.push(i as u32);
                    }
                }
                None => {}
            }
        }
    }

    pub fn instanced(&self) -> &[u32] {
        &self.ids_instanced
    }

    pub fn deforming(&self) -> &[u32] {
        &self.ids_deforming
    }

    pub const fn set_deform_threshold(&mut self, deform_threshold: f32) {
        self.deform_threshold = deform_threshold;
    }

    pub const fn deform_threshold(&self) -> f32 {
        self.deform_threshold
    }
}
