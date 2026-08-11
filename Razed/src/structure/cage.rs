use std::{cell::UnsafeCell, num::NonZeroUsize};

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

#[derive(Clone, Copy, Debug)]
pub struct CageUploadItem {
    pub map_index: IndirectIndex,
    pub bindref: glam::Vec3,
    pub lattice_binds: [glam::Vec4; PER_CAGE_MAX_LATTICE_ATTACHMENTS],
    pub lattice_lut: [IndirectIndex; PER_CAGE_MAX_LATTICE_ATTACHMENTS],
    pub points_bind: [glam::Vec4; PER_CAGE_POINTS],
    pub points_barycenter_bind: [glam::Vec4; PER_CAGE_POINTS],
    pub attachments: [LatticeAttachments; PER_CAGE_POINTS],
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Copy)]
pub struct CageRemapDelta {
    /// The new GPU index to set
    ///
    /// A value of `None` is basically a delete operation.
    pub gpu_index: Option<NonZeroUsize>,
    /// The stable ID of the CPU-GPU cage map
    pub map_id: IndirectIndex,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Copy)]
pub struct CageDeleteOp {
    pub map_id: IndirectIndex,
}

#[derive(Clone, Debug, Default)]
pub struct CageSyncFrameBuffers {
    upload: TriVec<CageUploadItem>,
    remap: TriVec<CageRemapDelta>,
    delete: TriVec<CageDeleteOp>,
}
impl CageSyncFrameBuffers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cpu(&self) -> CageSyncFrameOps<'_, CageSyncCpu> {
        CageSyncFrameOps::from_buffers(self)
    }

    pub fn gpu(&self) -> CageSyncFrameOps<'_, CageSyncGpu> {
        CageSyncFrameOps::from_buffers(self)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct OffsetRotation {
    pub offset: glam::Vec4,
    pub rotation: glam::Quat,
}

#[derive(Debug, Default)]
pub struct CageSystem {
    local_upload_buf: Vec<CageUploadItem>,
    local_delete_buf: Vec<CageDeleteOp>,

    gpu_map: IndexArrayColumn<()>,

    deformation_feedback: Vec<OffsetRotation>,

    /// Mapping of lattice node point ID to cage ID attached to the node.
    node_map: Vec<IndirectIndex>,

    generate_query_near_buf: Vec<Cell>,
}
unsafe impl Send for CageSystem {}
unsafe impl Sync for CageSystem {}
impl CageSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_deformation_feedback(&mut self, data: &[OffsetRotation]) {
        self.deformation_feedback.clear();
        self.deformation_feedback.extend_from_slice(data);
    }

    pub fn deformation_feedback(&self) -> &[OffsetRotation] {
        &self.deformation_feedback
    }

    pub fn gpu_map(&self) -> &IndexArrayColumn<()> {
        &self.gpu_map
    }

    pub fn gpu_map_mut(&mut self) -> &mut IndexArrayColumn<()> {
        &mut self.gpu_map
    }

    pub fn gpu_index_of(&self, cage_id: IndirectIndex) -> Option<u32> {
        self.gpu_map()
            .slots_map()
            .get(cage_id.as_index())
            .copied()
            .map(DirectIndex::as_int)
    }

    pub fn clear_op_buffers(&mut self) {
        self.local_upload_buf.clear();
        self.local_delete_buf.clear();
    }

    pub fn upload_buffer(&self) -> &[CageUploadItem] {
        &self.local_upload_buf
    }

    pub fn delete_buffer(&self) -> &[CageDeleteOp] {
        &self.local_delete_buf
    }

    pub fn delete_cage(&mut self, cage_id: IndirectIndex) {
        let direct = self.gpu_map.solve_indirect(cage_id);
        if direct.is_some_and(|d| d.as_int() == 0) || direct.is_none() {
            tracing::error!("attempted to delete uninitialized or invalid cage: operation aborted");
            self.gpu_map.free(cage_id);
            return;
        }
        self.local_delete_buf.push(CageDeleteOp { map_id: cage_id });
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
        let cage = CageUploadItem {
            map_index: map_id,
            bindref: cage_center,
            lattice_binds: lattice_bind_pos,
            lattice_lut: cage_data.attached_lattice,
            points_bind: points_pos,
            points_barycenter_bind: points_barycenter_bind,
            attachments: points_attachments,
        };
        self.local_upload_buf.push(cage);
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

pub struct CageSyncCpu;
pub struct CageSyncGpu;

#[derive(Debug)]
pub struct CageSyncFrameOps<'buffers, Op> {
    _op: std::marker::PhantomData<Op>,
    upload: &'buffers TriVec<CageUploadItem>,
    remap: &'buffers TriVec<CageRemapDelta>,
    delete: &'buffers TriVec<CageDeleteOp>,
}
impl<'buffers, Op> CageSyncFrameOps<'buffers, Op> {
    pub const fn from_buffers(buffers: &'buffers CageSyncFrameBuffers) -> Self {
        Self {
            _op: std::marker::PhantomData,
            upload: &buffers.upload,
            remap: &buffers.remap,
            delete: &buffers.delete,
        }
    }
}
impl<'buffers, Op> From<&'buffers CageSyncFrameBuffers> for CageSyncFrameOps<'buffers, Op> {
    fn from(value: &'buffers CageSyncFrameBuffers) -> Self {
        Self::from_buffers(value)
    }
}
impl<'buffers> CageSyncFrameOps<'buffers, CageSyncCpu> {
    pub fn upload(&self, section: StorageSection, data: &[CageUploadItem]) {
        self.upload.extend_from_slice(section.as_index(), data);
    }

    pub fn delete(&self, section: StorageSection, data: &[CageDeleteOp]) {
        self.delete.extend_from_slice(section.as_index(), data);
    }

    pub fn remap(&self, section: StorageSection, local_map: &mut IndexArrayColumn<()>) {
        self.remap.drain(section.as_index(), ..).for_each(
            |CageRemapDelta { gpu_index, map_id }| match gpu_index {
                Some(new_index) => {
                    let new_index = new_index.get();
                    let new_direct = DirectIndex::from_index(new_index, map_id.generation());
                    local_map.slots_map_mut()[map_id.as_index()] = new_direct;
                }
                _ => {
                    local_map.free(map_id);
                }
            },
        );
    }
}
impl<'buffers> CageSyncFrameOps<'buffers, CageSyncGpu> {
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

    pub fn upload(&self, section: StorageSection, gpu_buf: &CagePartitionedBuffer) {
        let mut offset = gpu_buf.length_pod_bindref() + 1;
        self.upload.drain(section.as_index(), ..).for_each(|data| {
            let CageUploadItem {
                map_index,
                bindref,
                lattice_binds,
                points_bind,
                lattice_lut,
                points_barycenter_bind,
                attachments,
            } = data;

            let bindref = glam::vec4(bindref.x, bindref.y, bindref.z, 1.0);
            let points_bind = CagePoints(points_bind);
            gpu_buf.blit_rmap(&[map_index], offset);
            gpu_buf.blit_pod_bindref(&[bindref], offset);
            gpu_buf.blit_pod_bind_lattice(&[lattice_binds], offset);
            gpu_buf.blit_pod_lut_lattice(&[lattice_lut], offset);
            gpu_buf.blit_pod_points_bind(&[points_bind], offset);
            gpu_buf.blit_pod_barycenter_bind(&[points_barycenter_bind], offset);
            gpu_buf.blit_pod_attachments(&[attachments], offset);

            let gpu_index = NonZeroUsize::new(offset).expect("offset is never 0");
            self.remap(section, Some(gpu_index), map_index);

            offset += 1;
        });
    }

    pub fn delete(
        &self,
        section: StorageSection,
        gpu_buf: &CagePartitionedBuffer,
        imap: &[DirectIndex],
    ) {
        self.delete
            .drain(section.as_index(), ..)
            .for_each(|CageDeleteOp { map_id }| {
                let direct = imap[map_id.as_index()];
                if !map_id.related_to_direct(&direct) {
                    tracing::error!(
                        "error while processing cage deletion: generation mismatch, expected {}, got {}",
                        direct.generation(),
                        map_id.generation()
                    );
                    return;
                }

                let index = direct.as_index();
                let length = gpu_buf.length_pod_bindref();
                Self::swap_remove_cage(gpu_buf.inner(), index, length);
                self.remap(section, None, map_id);
            });
    }

    pub fn remap(
        &self,
        section: StorageSection,
        gpu_index: Option<NonZeroUsize>,
        map_id: IndirectIndex,
    ) {
        self.remap
            .push(section.as_index(), CageRemapDelta { gpu_index, map_id });
    }
}
