pub(crate) mod physics;

use std::sync::atomic::Ordering;

use crate::{
    data::{
        FrameDataBuffers, LayoutEntityData, LayoutFragmentData, LayoutXpbdDebugData, Renderable,
    },
    state::physics::XpbdSystem,
    structure::{
        self, FragmentSystem,
        fragment::{VoxelGrid, VoxelGridOptions},
    },
};
use ::physics::xpbd::{LatticeIds, XpbdLatticeBuilder, XpbdOptions, XpbdSolver};
use ethel::{
    render::{ScreenSpace, command::DrawArraysIndirectCommand},
    state::{
        camera,
        data::{Column, SparseSlot},
    },
};
use tracing::event;

ethel::table_spec! {
    struct EntityData {
        position: glam::Vec4;
        rotation: glam::Quat;
        scale: glam::Vec4;
    }
}

const GROUND_LEVEL: f32 = -15.0;

#[derive(Debug)]
pub struct State {
    renderables: Vec<Renderable>,
    mesh_ids: Vec<ethel::mesh::Id>,

    entity_data: EntityDataRowTable,
    xpbd: XpbdSystem,
    fragments: FragmentSystem,

    /// Mapping between fragment direct index and the **RENDERABLE** index
    frag_map: Vec<u32>,

    /// Selected xpbd link id
    selection: Option<u32>,

    camera: camera::Orbital,
}

const CAMERA_YAW_CLAMP: std::ops::Range<f32> = f32::NEG_INFINITY..f32::INFINITY;
const CAMERA_PITCH_CLAMP: std::ops::Range<f32> =
    -std::f32::consts::FRAC_PI_2..std::f32::consts::FRAC_PI_2;

impl Default for State {
    fn default() -> Self {
        Self {
            xpbd: XpbdSystem::new(XpbdSolver::new(
                XpbdOptions::default().with_ground_level(Some(GROUND_LEVEL)),
            )),

            fragments: Default::default(),
            renderables: Default::default(),
            mesh_ids: Default::default(),
            entity_data: Default::default(),
            frag_map: Default::default(),
            selection: Default::default(),
            camera: camera::Orbital::new(
                Default::default(),
                Default::default(),
                camera::RotationLimits::new(CAMERA_YAW_CLAMP, CAMERA_PITCH_CLAMP),
            ),
        }
    }
}

impl ethel::StateHandler<FrameDataBuffers> for State {
    const COMMAND_QUEUE_LENGTH: usize = 512;

    fn upload_gpu(
        &mut self,
        frame_boundary: &ethel::state::cross::Cross<
            ethel::state::cross::Producer,
            FrameDataBuffers,
        >,
        command_queue: &mut ethel::render::command::GpuCommandQueue<ethel::DrawCommand>,
    ) {
        // command_queue.push(DrawArraysIndirectCommand {
        //     count: 36,
        //     instance_count: self.renderables.len() as u32,
        //     first_vertex: 0,
        //     base_instance: 0,
        // });

        let fragment_count = self.fragments.table().len() as u32;
        command_queue.push(DrawArraysIndirectCommand {
            count: 36,
            // degenerate 0 offset handled in shader
            instance_count: fragment_count - 1,
            first_vertex: 0,
            base_instance: 0,
        });

        frame_boundary.cross(|section, storage| {
            let buf_idx = section.as_index();

            {
                let fragments = &storage.fragments;

                let imap_nodes = self.xpbd.nodes().handles();
                let pod_nodes_positions = self.xpbd.nodes().current_pos_slice();
                let pod_nodes_rotors = self.xpbd.rotor_system().rotations();
                let pod_parents = self.fragments.table().parents_slice();
                let pod_weights = self.fragments.table().influence_slice();
                let pod_offsets = self.fragments.table().rest_offset_slice();

                // SAFETY: the use of LayoutFragmentData ensures we are
                // blitting to a valid section of the fragments partitioned
                // buffer.
                unsafe {
                    fragments.blit_part(buf_idx, LayoutFragmentData::ImapNodes as usize, imap_nodes, 0);
                    fragments.blit_part_padded(buf_idx, LayoutFragmentData::PodNodesPositions as usize, pod_nodes_positions, 0, 4);
                    fragments.blit_part(buf_idx, LayoutFragmentData::PodNodesRotors as usize, pod_nodes_rotors, 0);
                    fragments.blit_part(buf_idx, LayoutFragmentData::PodParents as usize, pod_parents, 0);
                    fragments.blit_part(buf_idx, LayoutFragmentData::PodWeights as usize, pod_weights, 0);
                    fragments.blit_part_padded(buf_idx, LayoutFragmentData::PodOffsets as usize, pod_offsets, 0, 4);
                }
            }

            {
                let scene = &storage.scene;

                let entity_map = &self.renderables;
                let mesh_handles = &self.mesh_ids;
                unsafe {
                    scene.blit_part(
                        buf_idx,
                        LayoutEntityData::EntityIndexMap as usize,
                        entity_map,
                        0,
                    );
                    scene.blit_part(
                        buf_idx,
                        LayoutEntityData::MeshData as usize,
                        mesh_handles,
                        0,
                    );
                }

                let imap_entity_data = self.entity_data.handles();
                let pod_positions = self.entity_data.position_slice();
                let pod_rotations = self.entity_data.rotation_slice();
                let pod_scales = self.entity_data.scale_slice();

                unsafe {
                    scene.blit_part(
                        buf_idx,
                        LayoutEntityData::ImapEntityData as usize,
                        imap_entity_data,
                        0,
                    );

                    scene.blit_part(
                        buf_idx,
                        LayoutEntityData::PodPositions as usize,
                        pod_positions,
                        0,
                    );
                    scene.blit_part(
                        buf_idx,
                        LayoutEntityData::PodRotations as usize,
                        pod_rotations,
                        0,
                    );
                    scene.blit_part(
                        buf_idx,
                        LayoutEntityData::PodScales as usize,
                        pod_scales,
                        0,
                    );
                }

                let xpbd_dbg = &storage.xpbd_debug;
                let constraints = self.xpbd.links().relation_slice();
                let imap_nodes = self.xpbd.nodes().handles();
                let pod_nodes = self.xpbd.nodes().current_pos_slice();
                let selected_link = {
                    let handle = self.selection.unwrap_or_default();
                    self.xpbd.links().get_indirect(handle).unwrap_or_default()
                };

                let node_count = self.xpbd.links().len() as u32;
                storage.xpbd_debug_link_count.store(node_count, Ordering::Release);

                const VEC3_VEC4_PADDING: usize = 4;

                // SAFETY: the use of LayoutXpbdDebugData ensures we are
                // blitting to a valid section of the xpbd_dbg partitioned
                // buffer.
                unsafe {
                    xpbd_dbg.blit_part(buf_idx, LayoutXpbdDebugData::Constraints as usize, constraints, 0);
                    xpbd_dbg.blit_part(buf_idx, LayoutXpbdDebugData::ImapNodes as usize, imap_nodes, 0);
                    xpbd_dbg.blit_part_padded(buf_idx, LayoutXpbdDebugData::PodNodes as usize, pod_nodes, 0, VEC3_VEC4_PADDING);
                    xpbd_dbg.blit_part(buf_idx, LayoutXpbdDebugData::ISelected as usize, &[selected_link], 0);
                }
            }

            {
                let commands = &storage.command;
                let mut data = commands.view_section_mut(buf_idx);
                if let Err(overflow) = command_queue.upload(&mut data) {
                    event!(
                        name: "boundary.upload_gpu.command.overflow",
                        tracing::Level::WARN,
                        "render command queue overflow during upload: {overflow} commands could not be uploaded and will be discarded"
                    )
                }

            }
        });

        command_queue.clear();
    }

    fn step(
        &mut self,
        input: &mut ethel::InputSystem,
        screen: &mut janus::sync::Mirror<ScreenSpace>,
        view_point: &mut janus::sync::Mirror<camera::ViewPoint>,
        delta: janus::context::DeltaTime,
    ) {
        view_point.sync().unwrap();

        if !input.cursor_options().grabbed {
            screen.sync().unwrap();

            if let Some(selected) = self.selection.take()
                && input.keys().key_pressed(janus::input::KeyCode::Delete)
            {
                self.xpbd.break_constraint(selected);
            }

            let cursor = input.cursor().current_f32();
            let inverse_view = view_point.into_mat4();

            let mouse_world_dir = screen.to_world_space(cursor, inverse_view);
            if input.keys().key_pressed(janus::input::KeyCode::Space) {
                let dy = mouse_world_dir.y;
                if dy.abs() > 0.001 {
                    let t = -view_point.position.y / dy;
                    let anchor = view_point.position + mouse_world_dir * t;
                    self.camera.set_anchor(anchor);
                }
            }

            let mouse_ray = ::physics::Ray::new(view_point.position, mouse_world_dir);

            let node_positions = self.xpbd.nodes().current_pos_slice();
            let constraints = self.xpbd.links().relation_view();
            let mut closest = None::<f32>;

            for (i, ::physics::xpbd::LinkNodes(a, b)) in constraints.into_iter().enumerate() {
                const RAY_SIZE: f32 = 0.05;

                let a_i = unsafe { self.xpbd.nodes().get_indirect_unchecked(*a) };
                let b_i = unsafe { self.xpbd.nodes().get_indirect_unchecked(*b) };
                let a_p = *unsafe { node_positions.get_unchecked(a_i as usize) };
                let b_p = *unsafe { node_positions.get_unchecked(b_i as usize) };

                if let Some(t) = ::physics::intersect_ray_segment(mouse_ray, (a_p, b_p), RAY_SIZE) {
                    if let Some(ct) = closest
                        && t > ct
                    {
                        continue;
                    }

                    closest = Some(t);
                    let id = *unsafe { self.xpbd.links().handles().get_unchecked(i) };
                    self.selection = Some(id as u32);
                }
            }
        } else {
            let (dx, dy) = input.cursor().delta_f32();
            let (dx, dy) = (dx.to_radians(), dy.to_radians());
            self.camera.update(dx, dy);

            let dw = *input.mouse_wheel();
            *self.camera.distance_mut() -= dw * delta.as_f32() * 100.0;

            view_point.publish_with(|vp| {
                *vp = *self.camera.viewpoint();
            });
        }

        const WIND_FORCE: f32 = 1.0;
        self.xpbd
            .apply_forces_batched(glam::vec3(WIND_FORCE, -9.81, WIND_FORCE));

        {
            let broken_links = self.xpbd.frame_broken_links();
            self.fragments
                .handle_constraint_break(broken_links, self.xpbd.links());

            let broken_frags = self.fragments.frame_disabled_frags_direct();
            for &broken in broken_frags {
                let renderable_id = *unsafe { self.frag_map.get_unchecked(broken as usize) };
                let entity_id = self.renderables[renderable_id as usize].data_handle;
                let e_index = unsafe { self.entity_data.get_indirect_unchecked(entity_id) };
                let pos = unsafe {
                    self.entity_data
                        .position_mut_slice()
                        .get_unchecked_mut(e_index as usize)
                };

                pos.w = 0.0;
            }
        }

        self.xpbd.update(delta);

        // random demo
        if input.keys().key_pressed(janus::input::KeyCode::KeyH) {
            let vp = view_point.get();

            const WIDTH: f32 = 12.0;
            const HEIGHT: f32 = 6.0;
            const DEPTH: f32 = 16.0;
            const FLOORS: u32 = 4;
            const TOTAL_HEIGHT: f32 = HEIGHT * FLOORS as f32;

            let center = glam::vec3(vp.position.x, GROUND_LEVEL, vp.position.z);

            let lattice = structure::create_structure_lattice(center, WIDTH, HEIGHT, DEPTH, FLOORS);

            let mut voxel_grid = VoxelGrid::new(
                |_| true,
                VoxelGridOptions::default()
                    .with_width(WIDTH)
                    .with_height(TOTAL_HEIGHT)
                    .with_depth(DEPTH),
            );
            voxel_grid.build(center + glam::vec3(0f32, TOTAL_HEIGHT * 0.5, 0f32));

            self.register_structure(&voxel_grid, lattice);
        }

        const CAMERA_KEY: janus::input::KeyCode = janus::input::KeyCode::Tab;
        if input.keys().key_pressed(CAMERA_KEY) {
            input.cursor_options().publish_with(|opt| {
                opt.grabbed = true;
            });
        }
        if input.keys().key_released(CAMERA_KEY) {
            input.cursor_options().publish_with(|opt| {
                opt.grabbed = false;
            });
        }
    }
}

impl State {
    pub fn create_renderable(
        &mut self,
        mesh_id: u32,
        position: glam::Vec3,
        rotation: glam::Quat,
        scale: glam::Vec3,
    ) -> u32 {
        let position = glam::Vec4::new(position.x, position.y, position.z, 1.0);
        let scale = glam::Vec4::new(scale.x, scale.y, scale.z, 1.0);

        let data_handle = self.entity_data.put((position, rotation, scale));
        let entity = Renderable {
            mesh_id,
            data_handle,
        };

        let id = self.renderables.len();
        self.renderables.push(entity);
        id as u32
    }

    pub fn register_structure(
        &mut self,
        voxel_grid: &VoxelGrid,
        lattice: XpbdLatticeBuilder,
    ) -> LatticeIds {
        let l0 = self.xpbd.nodes().handles().len();
        let lattice_map = self.xpbd.import_lattice(lattice);
        let l1 = self.xpbd.nodes().handles().len();

        if l0 == l1 {
            return lattice_map;
        }

        // handle degenerate
        if self.frag_map.is_empty() {
            self.frag_map.push(0);
        }

        // todo: CLEANUP THIS UGLY PIECE OF SHIT
        let owners = &self
            .xpbd
            .nodes()
            .slots_map()
            .iter()
            .map(|v| v.saturating_sub(l0 as u32))
            .collect::<Vec<_>>();

        let handles = &self.xpbd.nodes().handles()[l0..l1];
        let positions = &self.xpbd.nodes().current_pos_slice()[l0..l1];

        let l0 = self.fragments.table().handles().len();
        self.fragments
            .generate_fragments(voxel_grid, (owners, handles, positions));
        let l1 = self.fragments.table().handles().len();

        // currently unnecessary
        // fragments are rendered directly, not as renderables
        // eventually this will no longer be the case: fragments will be
        // adapted to renderables through compute shaders.
        for frag_idx in l0..l1 {
            let table = self.fragments.table();
            let position = *unsafe { table.position_slice().get_unchecked(frag_idx) };
            let e_id = self.create_renderable(0, position, Default::default(), glam::Vec3::ONE);
            self.frag_map.push(e_id);
        }

        // debug render of nodes
        // for &node_id in &lattice_map.nodes {
        //     let position = {
        //         let nodes = self.xpbd.nodes();
        //         let pos_id = unsafe { nodes.get_indirect_unchecked(node_id) };
        //         *unsafe { nodes.current_pos_slice().get_unchecked(pos_id as usize) }
        //     };
        //
        //     self.create_renderable(0, position, Default::default(), glam::Vec3::ONE * 0.5);
        // }

        lattice_map
    }
}
