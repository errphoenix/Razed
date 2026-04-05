pub(crate) mod physics;

use std::{sync::atomic::Ordering, time::Instant};

use crate::{
    data::{
        FrameDataBuffers, LayoutDebrisData, LayoutEntityData, LayoutFragmentData,
        LayoutXpbdDebugData, Renderable,
    },
    state::physics::LatticeSystem,
    structure::{
        self, FragmentState, FragmentSystem,
        deforms::{DeformSystem, DeformsRowTableView},
    },
    voxel::{VoxelGrid, VoxelGridOptions},
};
use ::physics::xpbd::{LatticeIds, NodesRowTableView, XpbdLatticeBuilder, XpbdOptions, XpbdSolver};
use ethel::{
    render::{ScreenSpace, command::DrawArraysIndirectCommand},
    state::{
        camera,
        data::{
            Column, IndirectIndex,
            hash::{FxSpatialHash, SpatialResolution},
        },
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

const GROUND_LEVEL: f32 = 0.0;

#[derive(Debug)]
pub struct State {
    renderables: Vec<Renderable>,
    mesh_ids: Vec<ethel::mesh::Id>,

    entity_data: EntityDataRowTable,
    lattice: LatticeSystem,
    fragments: FragmentSystem,
    deforms: DeformSystem,

    /// Parallel to NodesRowTable of LatticeSystem
    lattice_bind_pose: Vec<glam::Vec3>,

    /// Mapping between fragment direct index and the **RENDERABLE** index
    frag_map: Vec<u32>,

    /// Selected lattice link id
    selection: Option<IndirectIndex>,

    camera: camera::Orbital,
}

const CAMERA_YAW_CLAMP: std::ops::Range<f32> = f32::NEG_INFINITY..f32::INFINITY;
const CAMERA_PITCH_CLAMP: std::ops::Range<f32> =
    -std::f32::consts::FRAC_PI_2..std::f32::consts::FRAC_PI_2;

impl Default for State {
    fn default() -> Self {
        Self {
            lattice: LatticeSystem::new(XpbdSolver::new(
                XpbdOptions::default().with_ground_level(Some(GROUND_LEVEL)),
            )),
            lattice_bind_pose: Default::default(),
            fragments: Default::default(),
            deforms: Default::default(),
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
        let t0 = Instant::now();
        // command_queue.push(DrawArraysIndirectCommand {
        //     count: 36,
        //     instance_count: self.renderables.len() as u32,
        //     first_vertex: 0,
        //     base_instance: 0,
        // });

        let fragment_count = self.fragments.fragments().len() as u32;
        command_queue.push(DrawArraysIndirectCommand {
            count: 36,
            // degenerate 0 offset handled in shader
            instance_count: fragment_count - 1,
            first_vertex: 0,
            base_instance: 0,
        });

        frame_boundary.cross(|section, storage| {
            let buf_idx = section.as_index();

            const VEC3_VEC4_PADDING: usize = 4;

            // fragments upload
            {
                let fragments = &storage.fragments;

                let imap_deforms = self.deforms.data().handles();
                let pod_deforms_positions = self.deforms.data().deformed_slice();
                let pod_deforms_bind_pose = &self.deforms.data().pose_slice();
                let pod_anchors = self.fragments.fragments().anchors_slice();
                let pod_anchor_weights = self.fragments.fragments().anchors_weights_slice();
                let pod_bind_pose = self.fragments.fragments().bind_position_slice();
                let pod_states = self.fragments.fragments().state_slice();

                // SAFETY: the use of LayoutFragmentData ensures we blit to a
                // valid section of the partitioned buffer.
                unsafe {
                    fragments.blit_part(buf_idx, LayoutFragmentData::ImapDeforms as usize, imap_deforms, 0);
                    fragments.blit_part_padded(buf_idx, LayoutFragmentData::PodDeformsPositions as usize, pod_deforms_positions, 0, VEC3_VEC4_PADDING);
                    fragments.blit_part_padded(buf_idx, LayoutFragmentData::PodDeformsBindPose as usize, pod_deforms_bind_pose, 0, VEC3_VEC4_PADDING);
                    fragments.blit_part(buf_idx, LayoutFragmentData::PodAnchors as usize, pod_anchors, 0);
                    fragments.blit_part(buf_idx, LayoutFragmentData::PodAnchorsWeights as usize, pod_anchor_weights, 0);
                    fragments.blit_part(buf_idx, LayoutFragmentData::PodBindPose as usize, pod_bind_pose, 0);
                    fragments.blit_part(buf_idx, LayoutFragmentData::PodStates as usize, pod_states, 0);
                }
            }

            // debris upload
            {
                let debris = &storage.debris;
                let pod_positions = self.fragments.debris().position_slice();
                let pod_rotations = self.fragments.debris().rotation_slice();

                // SAFETY: the use of LayoutDebrisData ensures we blit to a
                // valid section of the partitioned buffer.
                unsafe {
                    debris.blit_part_padded(buf_idx, LayoutDebrisData::PodPositions as usize, pod_positions, 0, VEC3_VEC4_PADDING);
                    debris.blit_part(buf_idx, LayoutDebrisData::PodRotations as usize, pod_rotations, 0);
                }

                let debris_count = self.fragments.debris().len() as u32 - 1;
                storage.debris_count.store(debris_count, Ordering::Release);
            }

            // standard scene upload
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
            }

            // lattice debug upload
            {
                let xpbd_dbg = &storage.xpbd_debug;
                let constraints = self.lattice.links().relation_slice();
                let imap_nodes = self.lattice.nodes().handles();
                let pod_nodes = self.lattice.nodes().current_pos_slice();
                let selected_link = {
                    let handle = self.selection.unwrap_or_default();
                    self.lattice.links().solve_indirect(handle).unwrap_or_default()
                };

                let node_count = self.lattice.links().len() as u32;
                storage.xpbd_debug_link_count.store(node_count, Ordering::Release);

                // SAFETY: the use of LayoutXpbdDebugData ensures we blit to a
                // valid section of the partitioned buffer.
                unsafe {
                    xpbd_dbg.blit_part(buf_idx, LayoutXpbdDebugData::Constraints as usize, constraints, 0);
                    xpbd_dbg.blit_part(buf_idx, LayoutXpbdDebugData::ImapNodes as usize, imap_nodes, 0);
                    xpbd_dbg.blit_part_padded(buf_idx, LayoutXpbdDebugData::PodNodes as usize, pod_nodes, 0, VEC3_VEC4_PADDING);
                    xpbd_dbg.blit_part(buf_idx, LayoutXpbdDebugData::ISelected as usize, &[selected_link], 0);
                }
            }

            // cage deforms debug upload
            {
                let deform_dbg = &storage.deform_debug;
                let deform_dbg_ctl = &storage.deform_debug_controls;

                let deform_points = self.deforms.data().deformed_slice();
                let defrom_controls = self.deforms.data().controllers_slice();

                deform_dbg.blit_section_padded(buf_idx, deform_points, 0, VEC3_VEC4_PADDING);
                deform_dbg_ctl.blit_section(buf_idx, defrom_controls, 0);
                storage.deform_debug_count.store(self.deforms.data().len() as u32, Ordering::Release);
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
        let t1 = Instant::now();
        println!("upload to gpu: {} nanos", (t1 - t0).as_nanos());
    }

    fn step(
        &mut self,
        input: &mut ethel::InputSystem,
        screen: &mut janus::sync::Mirror<ScreenSpace>,
        view_point: &mut janus::sync::Mirror<camera::ViewPoint>,
        delta: janus::context::DeltaTime,
    ) {
        let t0 = Instant::now();
        view_point.sync().unwrap();

        if !input.cursor_options().grabbed {
            screen.sync().unwrap();

            if let Some(selected) = self.selection.take()
                && input.keys().key_pressed(janus::input::KeyCode::Delete)
            {
                self.lattice.break_constraint(selected);
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

            let node_positions = self.lattice.nodes().current_pos_slice();
            let constraints = self.lattice.links().relation_view();
            let mut closest = None::<f32>;

            for (i, ::physics::xpbd::LinkNodes(a, b)) in constraints.into_iter().enumerate() {
                const RAY_SIZE: f32 = 0.05;

                let a_i = unsafe { self.lattice.nodes().solve_indirect_unchecked(*a) };
                let b_i = unsafe { self.lattice.nodes().solve_indirect_unchecked(*b) };
                let a_p = *unsafe { node_positions.get_unchecked(a_i.as_index()) };
                let b_p = *unsafe { node_positions.get_unchecked(b_i.as_index()) };

                if let Some(t) = ::physics::intersect_ray_segment(mouse_ray, (a_p, b_p), RAY_SIZE) {
                    if let Some(ct) = closest
                        && t > ct
                    {
                        continue;
                    }

                    closest = Some(t);
                    self.selection = self.lattice.links().handles().get(i).copied();
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
        self.lattice
            .apply_forces_batched(glam::vec3(0.0, -9.81 * WIND_FORCE, 0.0));

        {
            {
                let t0 = Instant::now();
                self.lattice.register_dead_nodes();
                let t1 = Instant::now();
                println!(
                    "lattice dmg (gather degens): {} nanos",
                    (t1 - t0).as_nanos()
                )
            }
            let damaged_nodes = self.lattice.unique_damaged_nodes_frame();
            let degenerate_nodes = self.lattice.frame_degenerate_nodes();

            {
                let t0 = Instant::now();
                let lattice = NodesRowTableView::from(self.lattice.nodes());
                self.deforms.clear_damage_buffers();
                self.deforms.sync_lattice_damage(degenerate_nodes);
                self.deforms.constrain_v3(&lattice);
                self.deforms.process_damage(&lattice);
                let t1 = Instant::now();
                println!(
                    "deform dmg (sync lattice, constrain, finalize): {} nanos",
                    (t1 - t0).as_nanos()
                )
            }

            {
                let t0 = Instant::now();
                let deleted_points = self.deforms.deleted_points_frame();
                let deforms = DeformsRowTableView::from(self.deforms.data());
                self.fragments.clear_damage_buffer();
                self.fragments.sync_lattice_damage(damaged_nodes);
                self.fragments.sync_deform_damage(deleted_points, &deforms);
                self.fragments.compute_world_positions(&deforms);
                let t1 = Instant::now();
                println!(
                    "fragment dmg (sync lattice, sync_deform, compute world pos): {} nanos",
                    (t1 - t0).as_nanos()
                )
            }

            {
                let disabled_frags = self.fragments.frame_disabled_frags();
                if disabled_frags.len() > 0 {
                    struct DebrisData {
                        position: glam::Vec3,
                        velocity: glam::Vec3,
                        forces: glam::Vec3,
                        torque: glam::Vec3,
                        mass: f32,
                    }

                    let mut buffer = Vec::<DebrisData>::with_capacity(disabled_frags.len());

                    for &frag_index in disabled_frags {
                        if frag_index.as_int() == 0 {
                            continue;
                        }

                        let data = self.fragments.fragments();
                        let position = data.world_position_slice()[frag_index.as_index()];
                        let mass_coeff = data.mass_coeff_slice()[frag_index.as_index()];
                        let integrity = data.integrity_slice()[frag_index.as_index()];
                        let mass = integrity * mass_coeff;

                        buffer.push(DebrisData {
                            position,
                            velocity: glam::Vec3::ZERO,
                            forces: glam::Vec3::ZERO,
                            torque: glam::Vec3::ZERO,
                            mass,
                        });
                    }

                    println!("creating {} debris", buffer.len());
                    buffer.drain(..).for_each(
                        |DebrisData {
                             position,
                             velocity,
                             forces,
                             torque,
                             mass,
                         }| {
                            self.fragments.debris_mut().insert((
                                FragmentState::Debris,
                                0,
                                position,
                                glam::Quat::IDENTITY,
                                velocity,
                                glam::Vec3::ZERO,
                                forces,
                                torque,
                                mass,
                                glam::Mat3::IDENTITY,
                                glam::Mat3::IDENTITY,
                                ::physics::Sphere::new(0.65),
                            ));
                        },
                    );
                }
            }
        }

        {
            let t0 = Instant::now();
            self.lattice.update(delta);
            let t1 = Instant::now();
            println!("lattice pass: {} nanos", (t1 - t0).as_nanos())
        }
        {
            let t0 = Instant::now();
            let lattice = NodesRowTableView::from(self.lattice.nodes());
            self.deforms.deform(&lattice);
            let t1 = Instant::now();
            println!("deform pass: {} nanos", (t1 - t0).as_nanos())
        }
        {
            let t0 = Instant::now();
            self.fragments.simulate_debris(delta);
            let t1 = Instant::now();
            println!("debris physics pass: {} nanos", (t1 - t0).as_nanos())
        }

        // random demo
        if input.keys().key_pressed(janus::input::KeyCode::KeyH) {
            let vp = view_point.get();

            const WIDTH: f32 = 8.0;
            const HEIGHT: f32 = 4.0;
            const DEPTH: f32 = 8.0;
            const FLOORS: u32 = 8;
            const TOTAL_HEIGHT: f32 = HEIGHT * FLOORS as f32;

            let center = glam::vec3(vp.position.x, GROUND_LEVEL, vp.position.z);
            let lattice = structure::create_structure_lattice(center, WIDTH, HEIGHT, DEPTH, FLOORS);

            const INNER_SPACE: i32 = 2;
            let mut voxel_grid = VoxelGrid::new(
                |cell| cell.x.abs() > INNER_SPACE || cell.z.abs() > INNER_SPACE,
                VoxelGridOptions::default()
                    .with_width(WIDTH)
                    .with_height(TOTAL_HEIGHT)
                    .with_depth(DEPTH),
            );
            voxel_grid.repopulate();

            let center = center + glam::vec3(0.0, TOTAL_HEIGHT * 0.5, 0.0);
            self.register_structure(center, &voxel_grid, lattice);
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

        let t1 = Instant::now();
        println!("total step: {} nanos", (t1 - t0).as_nanos());
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

        let data_handle = self.entity_data.insert((position, rotation, scale));
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
        origin: glam::Vec3,
        voxel_grid: &VoxelGrid,
        lattice: XpbdLatticeBuilder,
    ) -> LatticeIds {
        let l0 = self.lattice.nodes().handles().len();
        let lattice_map = self.lattice.import_lattice(lattice);
        let l1 = self.lattice.nodes().handles().len();

        if l0 == l1 {
            return lattice_map;
        }

        let lattice = NodesRowTableView::from_range(self.lattice.nodes(), l0, l1 - l0);
        let mut lattice_hash = FxSpatialHash::new(SpatialResolution::new(1.0));
        lattice_hash.dump_soa(lattice.current_pos, lattice.handles);

        let mut deforms_vox = VoxelGrid::new(
            voxel_grid.generator,
            *&voxel_grid.options().with_cell_size(1.0),
        );
        deforms_vox.repopulate();
        let generated_len =
            self.deforms
                .generate_points(origin, &deforms_vox, &lattice_hash, &lattice);

        let deforms = DeformsRowTableView::from_range(
            self.deforms.data(),
            generated_len.start,
            generated_len.end - generated_len.start,
        );
        let mut deforms_hash = FxSpatialHash::new(SpatialResolution::new(1.0));
        deforms_hash.dump_soa(deforms.pose, deforms.handles);

        // handle degenerate
        if self.frag_map.is_empty() {
            self.frag_map.push(0);
        }

        // load initial node positions as bind pose
        if self.lattice_bind_pose.is_empty() {
            self.lattice_bind_pose.push(Default::default());
        }
        // cut off length to leave l0..l1 range to blank state
        self.lattice_bind_pose.resize(l0, Default::default());

        let new_positions = &self.lattice.nodes().current_pos_slice()[l0..l1];
        self.lattice_bind_pose.extend(new_positions);

        let l0 = self.fragments.fragments().handles().len();
        self.fragments.generate_fragments(origin, voxel_grid);
        self.fragments.bind_lattice(&lattice_hash, &lattice);
        self.fragments.bind_deforms(&deforms_hash, &deforms);
        let l1 = self.fragments.fragments().handles().len();

        // currently unnecessary
        // fragments are rendered directly, not as renderables
        // eventually this will no longer be the case: fragments will be
        // adapted to renderables through compute shaders.
        // for frag_idx in l0..l1 {
        //     let table = self.fragments.fragments();
        //     let position = *unsafe { table.position_slice().get_unchecked(frag_idx) };
        //     let e_id = self.create_renderable(0, position, Default::default(), glam::Vec3::ONE);
        //     self.frag_map.push(e_id);
        // }

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
