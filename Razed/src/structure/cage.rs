use std::num::NonZeroUsize;

use ethel::{
    data::{
        Column, DirectIndex, IndexArrayColumn, IndirectIndex, SparseSlot,
        hash::{Cell, FxSpatialHash},
        table::TableView,
    },
    render::buffer::{StorageSection, partitioned::PartitionedBuffer},
};
use janus::sync::TriVec;

use crate::{
    data::{CagePartitionedBuffer, LayoutCage},
    render::pass::{CagePoints, LatticeAttachments, NodeAttachment},
    structure::lattice::NodesRowTableView,
};

pub const PER_POINT_LATTICE_ATTACHMENTS: usize = 4;
pub const PER_CAGE_MAX_LATTICE_ATTACHMENTS: usize = 8;
pub const PER_CAGE_POINTS: usize = 8;
pub const CAGE_DIAG_EXTENT: f32 = 1.5;
pub const QUERY_LATTICE_ATTACH_MAX_RANGE: f32 = 32.0;

#[derive(Clone, Debug)]
pub struct CageAos {
    pub map_index: IndirectIndex,

    pub bindref: glam::Vec3,
    pub lattice_binds: [glam::Vec4; PER_CAGE_MAX_LATTICE_ATTACHMENTS],
    pub lattice_lut: [IndirectIndex; PER_CAGE_MAX_LATTICE_ATTACHMENTS],
    pub points_bind: [glam::Vec4; PER_CAGE_POINTS],
    pub points_barycenter_bind: [glam::Vec4; PER_CAGE_POINTS],
    pub attachments: [LatticeAttachments; PER_CAGE_POINTS],
}

#[derive(Debug, Default)]
pub struct CageSystem {
    local_buffer: Vec<CageAos>,
    gpu_map: IndexArrayColumn<()>,

    pipe: Option<CagePipeCpu>,

    /// Mapping of lattice node point ID to cage ID attached to the node.
    node_map: Vec<IndirectIndex>,

    generate_query_near_buf: Vec<Cell>,
}
impl CageSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn gpu_map(&self) -> &IndexArrayColumn<()> {
        &self.gpu_map
    }

    pub fn set_pipe(&mut self, pipe: CagePipeCpu) {
        self.pipe = Some(pipe);
    }

    pub fn pipe(&self) -> Option<&CagePipeCpu> {
        self.pipe.as_ref()
    }

    pub fn poll_remap(&mut self) {
        if let Some(pipe) = self.pipe.as_ref() {
            pipe.poll(&mut self.gpu_map);
        }
    }

    /// Queue a cage deletion request.
    ///
    /// The render thread will process the request and delete the associated
    /// cage data from the GPU buffers.
    ///
    /// If the `cage_id` is not associated to a valid GPU index, no request
    /// is sent at all.
    ///
    /// While GPU deletion is done on the render thread, the `cage_id` is
    /// immediately deleted from the local map residing on this thread.
    pub fn queue_delete_cage(&mut self, cage_id: IndirectIndex) {
        let gpu_index = self.gpu_index_of(cage_id);

        if gpu_index.is_some_and(|i| i == 0) || gpu_index.is_none() {
            tracing::warn!(
                "attempted to delete cage from ID that is not associated to a valid GPU index: {}",
                "either the render thread has not yet processed the associated cage, or the cage ID is degenerate."
            );
            return;
        }

        if let Some(pipe) = self.pipe() {
            pipe.queue_delete(cage_id);
        } else {
            tracing::error!(
                "unitialised CPU-side cage pipe, delete request will be discarded: {}",
                "pipe was never initialised."
            )
        }
    }

    pub fn gpu_index_of(&self, cage_id: IndirectIndex) -> Option<u32> {
        self.gpu_map()
            .slots_map()
            .get(cage_id.as_index())
            .copied()
            .map(DirectIndex::as_int)
    }

    pub fn upload_cages(&self, section: StorageSection, to: &TriVec<CageAos>) {
        to.extend_from_slice(section.as_index(), &self.local_buffer);
    }

    pub fn clear_cages_buffer(&mut self) {
        self.local_buffer.clear();
    }

    /// Generate a new cage and queue it for upload on the GPU buffers.
    ///
    /// Returns the [`IndirectIndex`] to the local map residing on this
    /// thread.
    pub fn generate_cage(
        &mut self,
        cage_center: glam::Vec3,
        lattice_hash: &FxSpatialHash<IndirectIndex>,
        lattice: &NodesRowTableView,
    ) -> IndirectIndex {
        let lattice_size = lattice.size();
        self.node_map.resize(lattice_size, Default::default());

        let near_buf = &mut self.generate_query_near_buf;
        let cage_data = Self::create_cage(
            cage_center,
            lattice_hash,
            lattice,
            near_buf,
            CAGE_DIAG_EXTENT,
        );

        let points_pos = cage_data.points.map(|p| {
            let local = p.world_point - cage_center;
            glam::vec4(local.x, local.y, local.z, 1.0)
        });
        let points_barycenter_bind = cage_data.points.map(|p| {
            let local = p.lattice_barycenter - cage_center;
            glam::vec4(local.x, local.y, local.z, 1.0)
        });
        let points_attachments = cage_data
            .points
            .map(|p| LatticeAttachments(p.lattice_attachments));
        let lattice_bind_pos = cage_data.lattice_bind_pos.map(|p| {
            let local = p - cage_center;
            glam::vec4(local.x, local.y, local.z, 1.0)
        });

        let map_id = self.gpu_map.insert(());
        let cage = CageAos {
            map_index: map_id,
            bindref: cage_center,
            lattice_binds: lattice_bind_pos,
            lattice_lut: cage_data.attached_lattice,
            points_bind: points_pos,
            points_barycenter_bind: points_barycenter_bind,
            attachments: points_attachments,
        };
        self.local_buffer.push(cage);
        map_id
    }

    fn create_cage(
        center: glam::Vec3,
        lattice_hash: &FxSpatialHash<IndirectIndex>,
        lattice: &NodesRowTableView,
        near_buf: &mut Vec<Cell>,
        cage_diag_half_extent: f32,
    ) -> CageData {
        let (attached_lattice, lattice_bind_pos) = {
            let mut attached_lattice = [IndirectIndex::default(); PER_CAGE_MAX_LATTICE_ATTACHMENTS];
            let mut lattice_bind_pos = [glam::Vec3::ZERO; PER_CAGE_MAX_LATTICE_ATTACHMENTS];

            let _ = lattice_hash.nearest_cells(
                lattice_hash.cell_at(center),
                PER_CAGE_MAX_LATTICE_ATTACHMENTS as u32,
                (QUERY_LATTICE_ATTACH_MAX_RANGE / lattice_hash.resolution.get()) as u32,
                near_buf,
                false,
            );
            near_buf
                .drain(..)
                .take(PER_CAGE_MAX_LATTICE_ATTACHMENTS)
                .enumerate()
                .for_each(|(i, cell)| {
                    let id = *lattice_hash.get(cell).unwrap();
                    let position = lattice.current_pos(id);
                    attached_lattice[i] = id;
                    lattice_bind_pos[i] = *position;
                });

            (attached_lattice, lattice_bind_pos)
        };

        // anchor order is guaranteed to be:
        // 0: -x, -y, -z,
        // 1:  x, -y, -z,
        // 2: -x,  y, -z,
        // 3:  x,  y, -z,
        // 4: -x, -y,  z,
        // 5:  x, -y,  z,
        // 6: -x,  y,  z,
        // 7:  x,  y,  z,
        let c = cage_diag_half_extent;
        let p000 = center - glam::Vec3::splat(c);
        let p100 = center + glam::vec3(c, -c, -c);
        let p010 = center + glam::vec3(-c, c, -c);
        let p110 = center + glam::vec3(c, c, -c);
        let p001 = center + glam::vec3(-c, -c, c);
        let p101 = center + glam::vec3(c, -c, c);
        let p011 = center + glam::vec3(-c, c, c);
        let p111 = center + glam::Vec3::splat(c);

        let points = [
            Self::create_cage_point(p000, &lattice_bind_pos),
            Self::create_cage_point(p100, &lattice_bind_pos),
            Self::create_cage_point(p010, &lattice_bind_pos),
            Self::create_cage_point(p110, &lattice_bind_pos),
            Self::create_cage_point(p001, &lattice_bind_pos),
            Self::create_cage_point(p101, &lattice_bind_pos),
            Self::create_cage_point(p011, &lattice_bind_pos),
            Self::create_cage_point(p111, &lattice_bind_pos),
        ];

        let lattice_weights_sums = points.map(|p| p.weight_sum);

        CageData {
            points,
            attached_lattice,
            lattice_bind_pos,
            lattice_weights_sums,
        }
    }

    fn create_cage_point(
        point: glam::Vec3,
        lattice_points: &[glam::Vec3; PER_CAGE_MAX_LATTICE_ATTACHMENTS],
    ) -> CagePointData {
        // sort lattice nodes near cage by proximity to this cage point
        // we only work on the N nearest points that are related to the cage
        let mut sorted_lattice_points = {
            let mut i = 0;
            lattice_points.map(|p| {
                let e = (i, p);
                i += 1;
                e
            })
        };
        sorted_lattice_points.sort_by(|(_, a), (_, b)| {
            let da = a.distance_squared(point);
            let db = b.distance_squared(point);
            da.total_cmp(&db)
        });

        let mut attachments = [NodeAttachment::default(); PER_POINT_LATTICE_ATTACHMENTS];
        let mut lattice_barycenter = glam::Vec3::ZERO;
        let mut lattice_weights_sum = 0f32;

        // process nearby lattice attachment points
        sorted_lattice_points
            .iter()
            .take(PER_POINT_LATTICE_ATTACHMENTS)
            .enumerate()
            .for_each(|(i, &(original_index, pos))| {
                let distance = point.distance(pos);
                let weight = 1.0 / (distance + 0.0001);
                lattice_weights_sum += weight;
                attachments[i] = NodeAttachment {
                    index: original_index as u32,
                    weight,
                };
            });

        // normalize lattice attachment weights
        attachments
            .iter_mut()
            .for_each(|attachment| attachment.weight /= lattice_weights_sum);

        // compute barycenter from attached nodes
        attachments
            .iter()
            .for_each(|&NodeAttachment { index, weight }| {
                let pos = lattice_points[index as usize];
                lattice_barycenter += pos * weight;
            });

        CagePointData {
            world_point: point,
            lattice_attachments: attachments,
            weight_sum: lattice_weights_sum,
            lattice_barycenter,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CageData {
    pub points: [CagePointData; PER_CAGE_POINTS],
    pub attached_lattice: [IndirectIndex; PER_CAGE_MAX_LATTICE_ATTACHMENTS],
    pub lattice_bind_pos: [glam::Vec3; PER_CAGE_MAX_LATTICE_ATTACHMENTS],
    pub lattice_weights_sums: [f32; PER_CAGE_POINTS],
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CagePointData {
    pub world_point: glam::Vec3,
    pub lattice_attachments: [NodeAttachment; PER_POINT_LATTICE_ATTACHMENTS],
    pub lattice_barycenter: glam::Vec3,
    pub weight_sum: f32,
}

pub fn pipes() -> (CagePipeCpu, CagePipeGpu) {
    let (gpu_tx, gpu_rx) = crossbeam::channel::unbounded::<CageDeleteGpu>();
    let (cpu_tx, cpu_rx) = crossbeam::channel::unbounded::<CageSyncRemap>();
    (
        CagePipeCpu {
            to_gpu: gpu_tx,
            from_gpu: cpu_rx,
        },
        CagePipeGpu {
            to_cpu: cpu_tx,
            from_cpu: gpu_rx,
        },
    )
}

#[derive(Clone, Debug)]
pub struct CageDeleteGpu {
    /// The stable ID of the CPU-GPU cage map
    pub map_id: IndirectIndex,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
pub struct CageSyncRemap {
    /// The new GPU index to set
    ///
    /// A value of `None` is basically a delete operation.
    pub gpu_index: Option<NonZeroUsize>,
    /// The stable ID of the CPU-GPU cage map
    pub map_id: IndirectIndex,
}

#[derive(Debug)]
pub struct CagePipeCpu {
    to_gpu: crossbeam::channel::Sender<CageDeleteGpu>,
    from_gpu: crossbeam::channel::Receiver<CageSyncRemap>,
}
impl CagePipeCpu {
    pub fn queue_delete(&self, map_id: IndirectIndex) {
        let _ = self.to_gpu.send(CageDeleteGpu { map_id });
    }

    pub fn poll(&self, map: &mut IndexArrayColumn<()>) {
        while let Ok(CageSyncRemap { gpu_index, map_id }) = self.from_gpu.try_recv() {
            match gpu_index {
                Some(new_index) => {
                    let new_index = new_index.get();
                    let new_direct = DirectIndex::from_index(new_index, map_id.generation());
                    map.slots_map_mut()[map_id.as_index()] = new_direct;
                }
                _ => {
                    map.free(map_id);
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct CagePipeGpu {
    to_cpu: crossbeam::channel::Sender<CageSyncRemap>,
    from_cpu: crossbeam::channel::Receiver<CageDeleteGpu>,
}
impl CagePipeGpu {
    pub fn queue_remap(&self, gpu_index: Option<NonZeroUsize>, map_id: IndirectIndex) {
        let _ = self.to_cpu.send(CageSyncRemap { gpu_index, map_id });
    }

    fn swap_remove_element<const PARTS: usize, T: Sized + Default>(
        buf: &PartitionedBuffer<PARTS>,
        partition: usize,
        to_remove: usize,
        length: usize,
    ) {
        unsafe {
            let mut view = buf.view_part_mut::<T>(partition);
            view.swap(to_remove, length - 1);
            buf.set_length(partition, (length - 1) as u32);
        }
    }

    fn swap_remove_cage<const PARTS: usize>(
        buf: &PartitionedBuffer<PARTS>,
        to_remove: usize,
        length: usize,
    ) -> IndirectIndex {
        Self::swap_remove_element::<PARTS, glam::Vec4>(
            buf,
            LayoutCage::PodBindRef as usize,
            to_remove,
            length,
        );
        Self::swap_remove_element::<PARTS, CagePoints>(
            buf,
            LayoutCage::PodPoints as usize,
            to_remove,
            length,
        );
        Self::swap_remove_element::<PARTS, CagePoints>(
            buf,
            LayoutCage::PodPointsBind as usize,
            to_remove,
            length,
        );
        Self::swap_remove_element::<PARTS, [glam::Quat; PER_CAGE_POINTS]>(
            buf,
            LayoutCage::PodRotation as usize,
            to_remove,
            length,
        );
        Self::swap_remove_element::<PARTS, [glam::Vec4; PER_CAGE_POINTS]>(
            buf,
            LayoutCage::PodBarycenterBind as usize,
            to_remove,
            length,
        );
        Self::swap_remove_element::<PARTS, [LatticeAttachments; PER_CAGE_POINTS]>(
            buf,
            LayoutCage::PodAttachments as usize,
            to_remove,
            length,
        );
        Self::swap_remove_element::<PARTS, [glam::Vec4; PER_CAGE_MAX_LATTICE_ATTACHMENTS]>(
            buf,
            LayoutCage::PodBindLattice as usize,
            to_remove,
            length,
        );
        Self::swap_remove_element::<PARTS, [IndirectIndex; PER_CAGE_MAX_LATTICE_ATTACHMENTS]>(
            buf,
            LayoutCage::PodLutLattice as usize,
            to_remove,
            length,
        );

        const RMAP_PART_INDEX: usize = LayoutCage::Rmap as usize;
        unsafe {
            let mut view = buf.view_part_mut::<IndirectIndex>(RMAP_PART_INDEX);
            view.swap(to_remove, length - 1);
            buf.set_length(RMAP_PART_INDEX, (length - 1) as u32);
            view[to_remove]
        }
    }

    pub fn poll(&self, gpu_data: &CagePartitionedBuffer, imap: &[DirectIndex]) {
        while let Ok(msg) = self.from_cpu.try_recv() {
            let id = msg.map_id;
            let direct = imap[id.as_index()];
            if !id.related_to_direct(&direct) {
                tracing::error!(
                    "error while processing cage deletion: generation mismatch, expected {}, got {}",
                    direct.generation(),
                    id.generation()
                );
                continue;
            }

            let index = direct.as_index();
            let length = gpu_data.length_pod_bindref();
            Self::swap_remove_cage(gpu_data.inner(), index, length);
            self.queue_remap(None, id);
        }
    }
}
