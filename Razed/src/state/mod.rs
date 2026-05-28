use std::{io::BufWriter, path::PathBuf, str::FromStr, sync::atomic::Ordering};

use crate::{
    data::{
        FrameDataBuffers, LayoutDebrisData, LayoutFragmentData, LayoutRenderableData,
        LayoutXpbdDebugData,
    },
    procedural::{VoxelGrid, VoxelGridOptions},
    render::RenderGroup,
    structure::{
        DebrisSystem, DeformSystem, DeformsRowTableView, FragmentSystem, FragmentsRowTableView,
        create_structure_lattice,
        debris::MotionAccumulator,
        lattice::{LatticeSystem, NodesRowTableView},
    },
};
use ::physics::xpbd::{RawXpbdLattice, XpbdOptions, XpbdSolver};
use ethel::{
    render::{
        ScreenSpace,
        command::{DrawArraysIndirectCommand, GpuCommandQueue},
    },
    state::{
        camera::{self, ViewPoint},
        cross::{Cross, Producer},
        data::{
            Column, IndirectIndex,
            hash::{Cell, FxSpatialHash, SpatialResolution},
        },
    },
};
use janus::context::DeltaTime;
use physics::rigid::RbVelocity;
use tracing::{Level, event};

ethel::table_spec! {
    struct Renderable {
        mesh_id: ethel::mesh::Id;
        position: glam::Vec4;
        rotation: glam::Quat;
        scale: glam::Vec4;
    }
}

const GROUND_LEVEL: f32 = 0.0;

#[derive(Debug)]
pub struct State {
    profiler: ethel::profile::Profiler,

    camera: camera::Orbital,
    mesh_ids: Vec<ethel::mesh::Id>,

    pub generic_objects: RenderableRowTable,

    lattice: LatticeSystem,
    deforms: DeformSystem,
    fragments: FragmentSystem,
    debris: DebrisSystem,

    /// Parallel to NodesRowTable of LatticeSystem
    lattice_bind_pose: Vec<glam::Vec3>,

    /// Mapping between fragment direct index and the **RENDERABLE** index
    frag_map: Vec<u32>,

    /// Selected lattice link id
    selection: Option<IndirectIndex>,
    dead_fragments: Vec<IndirectIndex>,

    pub frag_meshmap: FxSpatialHash<ethel::mesh::Id>,
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
            camera: camera::Orbital::new(
                Default::default(),
                Default::default(),
                camera::RotationLimits::new(CAMERA_YAW_CLAMP, CAMERA_PITCH_CLAMP),
            ),

            profiler: Default::default(),
            lattice_bind_pose: Default::default(),
            deforms: Default::default(),
            fragments: Default::default(),
            debris: Default::default(),
            mesh_ids: Default::default(),
            generic_objects: Default::default(),
            frag_map: Default::default(),
            selection: Default::default(),
            dead_fragments: Default::default(),
            frag_meshmap: Default::default(),
        }
    }
}

impl ethel::StateHandler<FrameDataBuffers, RenderGroup> for State {
    fn upload_gpu(
        &mut self,
        frame_boundary: &Cross<Producer, FrameDataBuffers>,
        command_queue: &mut GpuCommandQueue<ethel::DrawCommand, RenderGroup>,
    ) {
        command_queue.clear();

        let fragment_count = self.fragments.data().len() - 1;
        let debris_count = self.debris.total_debris_count();

        // populate command buffers
        {
            {
                command_queue.push_group(RenderGroup::Fragment);
                for _ in 0..fragment_count {
                    command_queue.push_command(DrawArraysIndirectCommand {
                        count: 0,
                        instance_count: 1,
                        first_vertex: 0,
                        base_instance: 0,
                    });
                }
            }
            {
                command_queue.push_group(RenderGroup::Debris);
                for _ in 0..debris_count {
                    command_queue.push_command(DrawArraysIndirectCommand {
                        count: 0,
                        instance_count: 1,
                        first_vertex: 0,
                        base_instance: 0,
                    });
                }
            }
        }
        frame_boundary.cross(|section, storage| {
            let buf_idx = section.as_index();

            const VEC3_VEC4_PADDING: usize = 4;

            // fragments upload
            {
                let fragments = &storage.fragments;

                let imap_deforms = self.deforms.data().handles();
                let pod_deforms_positions = self.deforms.data().deformed_slice();
                let pod_deforms_bind_pose = &self.deforms.data().pose_slice();
                let pod_anchors = self.fragments.data().anchors_slice();
                let pod_anchor_weights = self.fragments.data().anchors_weights_slice();
                let pod_bind_pose = self.fragments.data().bind_position_slice();
                let pod_mesh_id = self.fragments.data().mesh_id_slice();

                // SAFETY: the use of LayoutFragmentData ensures we blit to a
                // valid section of the partitioned buffer.
                unsafe {
                    fragments.blit_part(
                        buf_idx,
                        LayoutFragmentData::ImapDeforms as usize,
                        imap_deforms,
                        0,
                    );
                    fragments.blit_part_padded(
                        buf_idx,
                        LayoutFragmentData::PodDeformsPositions as usize,
                        pod_deforms_positions,
                        0,
                        VEC3_VEC4_PADDING,
                    );
                    fragments.blit_part_padded(
                        buf_idx,
                        LayoutFragmentData::PodDeformsBindPose as usize,
                        pod_deforms_bind_pose,
                        0,
                        VEC3_VEC4_PADDING,
                    );
                    fragments.blit_part(
                        buf_idx,
                        LayoutFragmentData::PodAnchors as usize,
                        pod_anchors,
                        0,
                    );
                    fragments.blit_part(
                        buf_idx,
                        LayoutFragmentData::PodAnchorsWeights as usize,
                        pod_anchor_weights,
                        0,
                    );
                    fragments.blit_part(
                        buf_idx,
                        LayoutFragmentData::PodBindPose as usize,
                        pod_bind_pose,
                        0,
                    );
                    fragments.blit_part(
                        buf_idx,
                        LayoutFragmentData::PodMeshId as usize,
                        pod_mesh_id,
                        0,
                    );
                }
            }

            // debris upload
            {
                let debris = &storage.debris;
                let pod_positions = self.debris.data().position_slice();
                let pod_rotations = self.debris.data().rotation_slice();
                let pod_mesh_id = self.debris.data().mesh_id_slice();
                let pod_positions_rubber = &self.debris.rubber().position_slice()[1..];
                let pod_rotations_rubber = &self.debris.rubber().rotation_slice()[1..];
                let pod_mesh_id_rubber = &self.debris.rubber().mesh_id_slice()[1..];
                let debris_offset_1 = self.debris.data().len() * size_of::<f32>();
                let debris_offset_4 = debris_offset_1 * 4;

                // SAFETY: the use of LayoutDebrisData ensures we blit to a
                // valid section of the partitioned buffer.
                unsafe {
                    debris.blit_part_padded(
                        buf_idx,
                        LayoutDebrisData::PodPositions as usize,
                        pod_positions,
                        0,
                        VEC3_VEC4_PADDING,
                    );
                    debris.blit_part_padded(
                        buf_idx,
                        LayoutDebrisData::PodPositions as usize,
                        pod_positions_rubber,
                        debris_offset_4,
                        VEC3_VEC4_PADDING,
                    );

                    debris.blit_part(
                        buf_idx,
                        LayoutDebrisData::PodRotations as usize,
                        pod_rotations,
                        0,
                    );
                    debris.blit_part(
                        buf_idx,
                        LayoutDebrisData::PodRotations as usize,
                        pod_rotations_rubber,
                        debris_offset_4,
                    );

                    debris.blit_part(
                        buf_idx,
                        LayoutDebrisData::PodMeshId as usize,
                        pod_mesh_id,
                        0,
                    );
                    debris.blit_part(
                        buf_idx,
                        LayoutDebrisData::PodMeshId as usize,
                        pod_mesh_id_rubber,
                        debris_offset_1,
                    );
                }

                let debris_count = self.debris.total_debris_count() as u32;
                storage.debris_count.store(debris_count, Ordering::Release);
            }

            // generic objects upload
            {
                let scene = &storage.generic_objects;

                let mesh_ids = self.generic_objects.mesh_id_slice();
                let pod_positions = self.generic_objects.position_slice();
                let pod_rotations = self.generic_objects.rotation_slice();
                let pod_scales = self.generic_objects.scale_slice();

                // SAFETY: the use of LayoutRenderableData ensures we blit
                // to a valid section of the partitioned buffer.
                unsafe {
                    scene.blit_part(buf_idx, LayoutRenderableData::MeshId as usize, mesh_ids, 0);

                    scene.blit_part(
                        buf_idx,
                        LayoutRenderableData::PodPositions as usize,
                        pod_positions,
                        0,
                    );
                    scene.blit_part(
                        buf_idx,
                        LayoutRenderableData::PodRotations as usize,
                        pod_rotations,
                        0,
                    );
                    scene.blit_part(
                        buf_idx,
                        LayoutRenderableData::PodScales as usize,
                        pod_scales,
                        0,
                    );
                }
            }

            // lattice debug upload
            {
                let xpbd_dbg = &storage.lattice_debug;
                let constraints = self.lattice.links().relation_slice();
                let imap_nodes = self.lattice.nodes().handles();
                let pod_nodes = self.lattice.nodes().current_pos_slice();
                let selected_link = {
                    let handle = self.selection.unwrap_or_default();
                    self.lattice
                        .links()
                        .solve_indirect(handle)
                        .unwrap_or_default()
                };

                let node_count = self.lattice.links().len() as u32;
                storage
                    .lattice_constraint_count
                    .store(node_count, Ordering::Release);

                // SAFETY: the use of LayoutXpbdDebugData ensures we blit to a
                // valid section of the partitioned buffer.
                unsafe {
                    xpbd_dbg.blit_part(
                        buf_idx,
                        LayoutXpbdDebugData::Constraints as usize,
                        constraints,
                        0,
                    );
                    xpbd_dbg.blit_part(
                        buf_idx,
                        LayoutXpbdDebugData::ImapNodes as usize,
                        imap_nodes,
                        0,
                    );
                    xpbd_dbg.blit_part_padded(
                        buf_idx,
                        LayoutXpbdDebugData::PodNodes as usize,
                        pod_nodes,
                        0,
                        VEC3_VEC4_PADDING,
                    );
                    xpbd_dbg.blit_part(
                        buf_idx,
                        LayoutXpbdDebugData::ISelected as usize,
                        &[selected_link],
                        0,
                    );
                }
            }

            // final command queue copy
            self.upload_gpu_commands(
                command_queue,
                command_queue.first_group().unwrap(),
                storage,
                buf_idx,
            );
        });
    }

    fn step(
        &mut self,
        input: &mut ethel::InputSystem,
        screen: &mut janus::sync::Mirror<ScreenSpace>,
        view_point: &mut janus::sync::Mirror<camera::ViewPoint>,
        delta: janus::context::DeltaTime,
    ) {
        self.profiler.page();
        view_point.sync().unwrap();

        if input.keys().key_pressed(janus::input::KeyCode::KeyB) {
            self.debris.data_mut().clear();
            self.debris.rubber_mut().clear();
        }

        if input.keys().key_pressed(janus::input::KeyCode::KeyP) {
            self.save_profiler_report();
        }

        if !input.cursor_options().grabbed {
            screen.sync().unwrap();
            self.select_lattice_raycast(input, screen.get(), view_point.get());
        } else {
            {
                const ANCHOR_Y_MOVE: glam::Vec3 = glam::vec3(0.0, 1.0, 0.0);
                const ANCHOR_Y_MOVE_SPRINT: f32 = 4.0;
                let anchor = self.camera.anchor();

                let mut d = ANCHOR_Y_MOVE * delta.as_f32();
                if input.keys().key_down(janus::input::KeyCode::ShiftLeft) {
                    d *= ANCHOR_Y_MOVE_SPRINT;
                }

                if input.keys().key_down(janus::input::KeyCode::ArrowUp) {
                    self.camera.set_anchor(anchor + d);
                } else if input.keys().key_down(janus::input::KeyCode::ArrowDown) {
                    self.camera.set_anchor(anchor - d);
                }
            }

            let (dx, dy) = input.cursor().delta_f32();
            let (dx, dy) = (dx.to_radians(), dy.to_radians());
            self.camera.update(dx, dy);

            let dw = *input.mouse_wheel();
            *self.camera.distance_mut() -= dw * delta.as_f32() * 100.0;

            view_point.publish_with(|vp| {
                *vp = *self.camera.viewpoint();
            });
        }

        if input.keys().key_pressed(janus::input::KeyCode::KeyH) {
            self.spawn_debug_structure(view_point.get());
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

        self.lattice.clear_damage_buffers();

        self.profiler.push_trace("struct_prepass");
        self.profiler.capture_duration("pull_integrity", || {
            let fragments = FragmentsRowTableView::from(self.fragments.data());
            self.lattice.pull_integrity_mass(&fragments);
        });
        self.profiler.capture_duration("sum_integrities", || {
            self.lattice.sync_constraint_attributes();
        });

        self.profiler.capture_duration("apply_forces", || {
            const WIND_FORCE: f32 = 0.0;
            self.lattice
                .apply_forces_batched(glam::vec3(WIND_FORCE, -9.81, WIND_FORCE));
        });

        self.profiler.pop_trace();
        self.profiler.push_trace("simulation");

        self.profiler.capture_duration("fragment_sync_cage", || {
            let deforms = DeformsRowTableView::from(self.deforms.data());
            self.fragments.compute_world_positions(&deforms);
        });

        // synchronizes lattice damage from xpbd-lattice solver
        self.profiler
            .capture_duration("lattice_damage_register", || {
                self.lattice.register_dead_nodes()
            });

        //self.process_cage_damage();

        // synchronizes lattice and cage damage to fragments.
        // after this point, the order of fragment elements must not change;
        // i.e. there must be no free operations on the fragment table until
        // after the release_debris_bodies function.
        self.process_fragment_damage();
        self.deforms.delete_dead_points();
        self.release_debris_bodies();
        self.delete_disabled_fragments();

        self.profiler.push_trace("update_structures");
        self.update_subsystems(delta);
        self.update_debris(delta);
        self.profiler.pop_trace();

        self.profiler.pop_trace(); // end simulation trace group
    }
}

impl State {
    pub fn create_generic_object(
        &mut self,
        mesh: ethel::mesh::Id,
        position: glam::Vec3,
        rotation: glam::Quat,
        scale: glam::Vec3,
    ) -> IndirectIndex {
        self.generic_objects.insert((
            mesh,
            position.to_homogeneous(),
            rotation,
            scale.to_homogeneous(),
        ))
    }

    fn upload_gpu_commands(
        &self,
        command_queue: &GpuCommandQueue<ethel::DrawCommand, RenderGroup>,
        group: RenderGroup,
        frame_data: &FrameDataBuffers,
        tri_section: usize,
    ) {
        let buffer = match group {
            RenderGroup::Generic => &frame_data.generic_commands,
            RenderGroup::Fragment => &frame_data.fragment_commands,
            RenderGroup::Debris => &frame_data.debris_commands,
            RenderGroup::LatticeDebug => unimplemented!("lattice debug has no command buffer"),
        };

        let mut data = buffer.view_section_mut(tri_section);

        let il0 = command_queue.index() as u32;
        let next = command_queue.upload_next_group(&mut data);
        let length = command_queue.index() as u32 - il0;
        buffer.set_length(tri_section, length);

        if let Some(next) = next {
            self.upload_gpu_commands(command_queue, next, frame_data, tri_section);
        }
    }

    fn delete_disabled_fragments(&mut self) {
        self.profiler.capture_duration("fragment_delete_old", || {
            {
                let disabled_frags = self.fragments.frame_disabled_frags();
                let handles = self.fragments.data().handles();
                for index in disabled_frags {
                    let h = handles[index.as_index()];
                    self.dead_fragments.push(h);
                }
            }

            self.fragments.data_mut().free_many(&self.dead_fragments);
            self.dead_fragments.clear();
        });
    }

    fn release_debris_bodies(&mut self) {
        let disabled_frags = self.fragments.frame_disabled_frags();
        if disabled_frags.len() > 0 {
            struct DebrisData {
                position: glam::Vec3,
                velocity: glam::Vec3,
                ang_velocity: glam::Vec3,
                forces: glam::Vec3,
                torque: glam::Vec3,
                mass: f32,
                mesh_id: ethel::mesh::Id,
            }

            let mut buffer = Vec::<DebrisData>::with_capacity(disabled_frags.len());

            for &frag_index in disabled_frags {
                if frag_index.as_int() == 0 {
                    continue;
                }

                let data = self.fragments.data();
                let mesh_id = data.mesh_id[frag_index.as_index()];
                let position = data.world_position[frag_index.as_index()];
                let mass_coeff = data.mass_coeff[frag_index.as_index()];
                let integrity = data.integrity[frag_index.as_index()];
                let mass = integrity * mass_coeff;

                let mut inherit_v = glam::Vec3::ZERO;
                let mut inherit_a = glam::Vec3::ZERO;
                let mut inherit_av = glam::Vec3::ZERO;
                {
                    let lattice = NodesRowTableView::from(self.lattice.nodes());
                    let parents = data.parents[frag_index.as_index()];
                    let weights = data.parents_weights[frag_index.as_index()];
                    parents.iter().zip(weights).for_each(|(id, w)| {
                        let velocity = lattice.velocity(*id);
                        let forces = lattice.forces(*id);
                        inherit_v += velocity * w;
                        inherit_a += forces * w;

                        let p = lattice.current_pos(*id);
                        let contact = p.midpoint(position);
                        inherit_av += contact.cross(*velocity) * w;
                    });
                }
                inherit_a *= 0.01;
                inherit_v *= 0.035;
                inherit_av *= 0.01;

                let position = position - glam::Vec3::X * 2f32;

                buffer.push(DebrisData {
                    position,
                    velocity: inherit_v,
                    ang_velocity: inherit_av,
                    forces: inherit_a,
                    torque: glam::Vec3::ZERO,
                    mass,
                    mesh_id,
                });
            }

            println!("creating {} debris", buffer.len());
            buffer.drain(..).for_each(
                |DebrisData {
                     position,
                     ang_velocity,
                     velocity,
                     forces,
                     torque,
                     mass,
                     mesh_id,
                 }| {
                    self.debris.data_mut().insert((
                        0.0,
                        position,
                        glam::Quat::IDENTITY,
                        RbVelocity::new(velocity, ang_velocity),
                        forces,
                        torque,
                        mass,
                        glam::Mat3::IDENTITY,
                        glam::Mat3::IDENTITY,
                        ::physics::Sphere::new(0.5),
                        MotionAccumulator::default(),
                        mesh_id,
                    ));
                },
            );
        }
    }

    fn process_fragment_damage(&mut self) {
        self.profiler.push_trace("fragment_damage");

        // let deleted_points = self.deforms.deleted_points_frame();
        // let deforms = DeformsRowTableView::from(self.deforms.data());
        let damaged_nodes = self.lattice.unique_damaged_nodes_frame();

        self.profiler
            .capture_duration("fragment_damage_sync_lattice", || {
                self.fragments.clear_damage_buffer();
                self.fragments.sync_lattice_damage(damaged_nodes);
            });
        // self.profiler
        //     .capture_duration("fragment_damage_sync_cage", || {
        //         self.fragments.sync_deform_damage(deleted_points, &deforms);
        //     });

        self.profiler.pop_trace();
    }

    fn process_cage_damage(&mut self) {
        self.profiler.push_trace("deform_damage");
        let degenerate_nodes = self.lattice.frame_degenerate_nodes();
        let lattice = NodesRowTableView::from(self.lattice.nodes());

        self.profiler
            .capture_duration("cage_damage_sync_lattice", || {
                self.deforms.clear_damage_buffers();
                self.deforms.sync_lattice_damage(degenerate_nodes, &lattice);
            });

        self.profiler.pop_trace();
    }

    fn update_debris(&mut self, delta: DeltaTime) {
        self.profiler
            .capture_duration("debris_hash", || self.debris.hash_debris());
        self.profiler.capture_duration("debris_phys_rb", || {
            self.debris.simulate_bodies(delta);
        });
        self.profiler.capture_duration("debris_sleep", || {
            self.debris.accumulate_motion();
            //self.debris.freeze_old_debris(delta);
        });
    }

    fn update_subsystems(&mut self, delta: DeltaTime) {
        self.profiler
            .capture_duration("update_lattice", || self.lattice.update(delta));
        self.profiler.capture_duration("update_cage", || {
            let lattice = NodesRowTableView::from(self.lattice.nodes());
            self.deforms.deform(&lattice)
        });
    }

    fn spawn_debug_structure(&mut self, view_point: &ViewPoint) {
        const WIDTH: f32 = 12.0;
        const HEIGHT: f32 = 6.0;
        const DEPTH: f32 = 12.0;
        const FLOORS: u32 = 8;
        const TOTAL_HEIGHT: f32 = HEIGHT * FLOORS as f32;

        let center = glam::vec3(view_point.position.x, GROUND_LEVEL, view_point.position.z);
        let lattice = create_structure_lattice(center, WIDTH, HEIGHT, DEPTH, FLOORS);

        const INNER_SPACE: i32 = 2;

        let mut voxel_grid = VoxelGrid::new(
            |cell| {
                let half_cell = Cell {
                    x: (WIDTH * 0.5) as i32,
                    y: (FLOORS as f32 * HEIGHT * 0.5) as i32,
                    z: (DEPTH * 0.5) as i32,
                };
                let cell = cell + half_cell;

                cell.x < INNER_SPACE
                    || cell.x > (WIDTH as i32) - INNER_SPACE - 1
                    || cell.z < INNER_SPACE
                    || cell.z > (DEPTH as i32) - INNER_SPACE - 1
            },
            VoxelGridOptions::default()
                .with_width(WIDTH)
                .with_height(TOTAL_HEIGHT)
                .with_depth(DEPTH),
        );
        voxel_grid.repopulate_defaults();

        let center = center + glam::vec3(0.0, TOTAL_HEIGHT * 0.5, 0.0);
        self.generate_structure(center, &voxel_grid, lattice);
    }

    fn save_profiler_report(&mut self) {
        let path = PathBuf::from_str("framestack_latest.bin").unwrap();
        if path.exists() {
            if let Ok(creation_time) = path.metadata().map(|meta| meta.created()).flatten() {
                let date: chrono::DateTime<chrono::Utc> = creation_time.into();
                let formatted_date = date.format("%d_%m_%y-%H_%M_%S");
                let new_path = format!("framestack_old_{}.bin", formatted_date);

                std::fs::rename(&path, new_path).expect("assumed user has sufficient permissions");

                event!(
                    Level::INFO,
                    "Created backup of previous latest framestack file to: framestack_old_{}.bin",
                    formatted_date
                )
            } else {
                event!(
                    Level::WARN,
                    "There was an error trying to backup the previous framestack_latest file: it will be deleted."
                );
                std::fs::remove_file(&path).expect("assumed user has file delete permissions");
            }
        }

        let mut out = BufWriter::new(std::fs::File::create(&path).unwrap());
        self.profiler.present_encoded(&mut out).unwrap();
        event!(
            Level::INFO,
            "Exported performance statistics to file: {:?}",
            path.canonicalize().unwrap()
        );
    }

    fn select_lattice_raycast(
        &mut self,
        input: &mut ethel::InputSystem,
        screen: &ScreenSpace,
        view_point: &ViewPoint,
    ) {
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

        for (i, [a, b]) in constraints.into_iter().enumerate() {
            const RAY_SIZE: f32 = 0.05;

            // view start at 1 to ignore degenerate element 0
            let i = i + 1;

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
    }

    pub fn generate_structure(
        &mut self,
        origin: glam::Vec3,
        grid: &VoxelGrid,
        lattice: RawXpbdLattice,
    ) {
        let l0 = self.lattice.nodes().handles().len();
        self.lattice.import_lattice(lattice);
        let l1 = self.lattice.nodes().handles().len();

        if l0 == l1 {
            return;
        }

        let lattice = NodesRowTableView::from_range(self.lattice.nodes(), l0, l1 - l0 - 1);
        let mut lattice_hash = FxSpatialHash::new(SpatialResolution::new(1.0));
        lattice_hash.dump_soa(lattice.current_pos, lattice.handles);

        let mut deforms_vox = VoxelGrid::new(grid.generator, *(grid.options()));
        deforms_vox.repopulate_defaults();
        let generated_len =
            self.deforms
                .generate_points(origin, &deforms_vox, &lattice_hash, &lattice);

        let deforms = DeformsRowTableView::from_range(
            self.deforms.data(),
            generated_len.start,
            generated_len.end - generated_len.start - 1,
        );
        let mut deforms_hash = FxSpatialHash::new(SpatialResolution::new(1.0));
        deforms_hash.dump_soa(deforms.pose, deforms.handles);

        // handle degenerate
        if self.frag_map.is_empty() {
            self.frag_map.push(0);
        }

        // load current node positions as reference bind poses
        {
            if self.lattice_bind_pose.is_empty() {
                self.lattice_bind_pose.push(Default::default());
            }
            // cut off length to leave l0..l1 range to blank state
            self.lattice_bind_pose.resize(l0, Default::default());

            let new_positions = &self.lattice.nodes().current_pos_slice()[l0..l1];
            self.lattice_bind_pose.extend(new_positions);
        }

        let abs_grid = grid.to_abs_space();
        let frag_meshmap = &self.frag_meshmap;
        self.fragments.generate(origin, &abs_grid, &frag_meshmap);

        self.fragments.bind_lattice(&lattice_hash, &lattice);
        self.fragments.bind_deforms(&deforms_hash, &deforms);
    }
}
