use std::{
    io::BufWriter,
    path::PathBuf,
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use crate::{
    data::{
        FrameDataBuffers, LayoutDebrisData, LayoutFragmentData, LayoutRenderableData,
        LayoutXpbdDebugData,
    },
    procedural::{VoxelGrid, VoxelGridOptions},
    render::RenderGroup,
    structure::{
        CageRowTableView, CageSystem, DebrisSystem, FragmentSystem, FragmentsRowTableView,
        create_structure_lattice,
        debris::MotionAccumulator,
        lattice::{LatticeSystem, NodesRowTableView},
    },
};
use ::physics::xpbd::{RawXpbdLattice, XpbdOptions, XpbdSolver};
use ethel::{
    assets::{AssetMetadataRegistry, TextureMetadata, pipe::RegistryPipe},
    data::SparseSlot,
    profile::Profiler,
    render::{
        Resolution, ScreenSpace,
        command::{DrawArraysIndirectCommand, GpuCommandQueue},
    },
    state::{
        camera::{self, ViewPoint},
        cross::{Cross, Producer},
        data::{
            Column, IndirectIndex,
            hash::{Cell, FxSpatialHash, SpatialResolution},
        },
        time::AccumulationWindow,
    },
};
use gui::{
    InterfaceSystem,
    text::{GlyphAtlas, GlyphRaster},
};
use janus::{
    context::DeltaTime,
    input::{Cursor, KeyEvent},
    sync::TriCell,
};
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

#[derive(Clone, Copy, Debug)]
pub struct PerfAverages {
    pub fps_average: AccumulationWindow<5, f32>,
    pub tps_average: AccumulationWindow<5, u32>,
    pub render_time_average: AccumulationWindow<5, f32>,
    pub simul_time_average: AccumulationWindow<5, f32>,
    last_display: Instant,
}
impl PerfAverages {
    pub const BUCKET_DURATION: Duration = Duration::from_millis(200);
    pub const DISPLAY_INTERVAL: Duration = Duration::from_millis(1000);

    pub fn update(&mut self) -> bool {
        if self.last_display.elapsed() >= Self::DISPLAY_INTERVAL {
            self.last_display = Instant::now();
            return true;
        }
        false
    }
}
impl Default for PerfAverages {
    fn default() -> Self {
        Self {
            fps_average: AccumulationWindow::new(Self::BUCKET_DURATION),
            tps_average: AccumulationWindow::new(Self::BUCKET_DURATION),
            render_time_average: AccumulationWindow::new(Self::BUCKET_DURATION),
            simul_time_average: AccumulationWindow::new(Self::BUCKET_DURATION),
            last_display: Instant::now(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SimCtl {
    pub speed: f32,
    pub running: bool,
}
impl Default for SimCtl {
    fn default() -> Self {
        Self {
            speed: 1f32,
            running: true,
        }
    }
}

#[derive(Debug)]
pub struct State {
    sim_control: SimCtl,

    local_keyev_buf: Vec<KeyEvent>,

    profiler: ethel::profile::Profiler,
    render_frame_time: TriCell<DeltaTime>,

    perf_avg: PerfAverages,

    ui_system: InterfaceSystem,
    pub glyph_atlas: GlyphAtlas,
    pub glyph_pipe: Option<crossbeam::channel::Sender<GlyphRaster>>,

    pub textures_metadata_registry: AssetMetadataRegistry<TextureMetadata>,
    pub texture_registry_pipe: RegistryPipe<crate::assets::Texture>,

    camera: camera::Orbital,

    #[allow(unused, reason = "currently unused, might be discarded")]
    mesh_ids: Vec<ethel::mesh::Id>, // generic objects mesh ids imap

    pub generic_objects: RenderableRowTable,

    lattice: LatticeSystem,
    cage: CageSystem,
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
            sim_control: SimCtl::default(),
            ui_system: crate::ui::initialize_default(Resolution::default()),
            lattice: LatticeSystem::new(XpbdSolver::new(
                XpbdOptions::default().with_ground_level(Some(GROUND_LEVEL)),
            )),
            camera: camera::Orbital::new(
                Default::default(),
                Default::default(),
                camera::RotationLimits::new(CAMERA_YAW_CLAMP, CAMERA_PITCH_CLAMP),
            ),
            profiler: Profiler::default(),
            lattice_bind_pose: Default::default(),
            cage: Default::default(),
            fragments: Default::default(),
            debris: Default::default(),
            mesh_ids: Default::default(),
            generic_objects: Default::default(),
            frag_map: Default::default(),
            selection: Default::default(),
            dead_fragments: Default::default(),
            frag_meshmap: Default::default(),
            local_keyev_buf: Default::default(),
            textures_metadata_registry: AssetMetadataRegistry::new(),
            texture_registry_pipe: Default::default(),
            glyph_atlas: Default::default(),
            render_frame_time: Default::default(),
            perf_avg: Default::default(),
            glyph_pipe: None,
        }
    }
}

impl ethel::StateHandler<FrameDataBuffers, RenderGroup> for State {
    fn step_duration(&self) -> std::time::Duration {
        std::time::Duration::from_millis(16)
    }

    fn upload_gpu(
        &mut self,
        frame_boundary: &Cross<Producer, FrameDataBuffers>,
        command_queue: &mut GpuCommandQueue<ethel::DrawCommand, RenderGroup>,
    ) {
        self.profiler.push_trace("upload");
        // prepare uploads
        {
            let t0 = Instant::now();
            self.ui_composite_batches();
            let t1 = Instant::now();
            self.profiler.log_explicit("composite_ui", t0 - t1);
            let t0 = t1;

            // populate command buffers
            {
                command_queue.clear();

                let fragment_count = self.fragments.data().len() - 1;
                let debris_count = self.debris.total_debris_count();

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

            let t1 = Instant::now();
            self.profiler.log_explicit("prepare_commands", t0 - t1);
            let t0 = t1;

            // send glyph raster copy requests
            {
                if let Some(pipe) = &self.glyph_pipe {
                    let rasters = self.ui_system.text_composer().rasters();
                    for raster in rasters {
                        pipe.send(raster).unwrap();
                    }
                }
            }

            let t1 = Instant::now();
            self.profiler.log_explicit("rasterize_glyphs", t0 - t1);
        }

        // initialize start profiler frame point after cross() operation
        // to avoid including sync time in profile report
        let t0 = Arc::new(AtomicU64::new(0));
        let t00 = Instant::now();
        frame_boundary.cross(|section, storage| {
            let elapsed = t00.elapsed().as_nanos();
            t0.store(elapsed as u64, Ordering::Relaxed);
            let buf_idx = section.as_index();

            let _ = self
                .render_frame_time
                .set_and_advance(storage.render_frame_last_duration.get());

            // interface quads & commands upload
            {
                // with TRIANGLE_STRIP draw mode
                const QUAD_VERTEX_COUNT: u32 = 4;

                let quads = &storage.interface_storage;
                let commands = &storage.interface_commands;

                let batches = self.ui_system.batches();

                let mut quad_offset = 0;
                for (i, batch) in batches.enumerate() {
                    let count = batch.data().len() as u32;

                    let command = gui::render::UiRenderCommandBasic {
                        vertex_count: QUAD_VERTEX_COUNT,
                        instance_count: count,
                        instance_offset: quad_offset,
                        texture_units: batch.textures(),
                    };

                    // SAFETY: safety is guaranteed by using the correct
                    // triple buffer section index.
                    // This loop is the only place where the 2 buffers are
                    // written to.
                    unsafe {
                        quads.blit_section(buf_idx, batch.data(), quad_offset as usize);
                        commands.blit_section(buf_idx, &[command], i);
                    }

                    quad_offset += count;
                }
            }

            // cage upload (only new)
            {
                let cage_data = self.cage.data();
                // let imap_cages = cage_data.slots_map();
                // let pod_cage_locals = cage_data.local_points_slice();
                // let pod_cage_locals_bind = cage_data.local_points_bind_slice();
                // let pod_cage_rotations = cage_data.rotation_slice();
                // let pod_cage_covariants = cage_data.covariant_slice();
                // let pod_cage_references = cage_data.world_bind_reference_slice();

                storage
                    .cage_points_count
                    .store(cage_data.len() as u32, Ordering::Release);

                //let tb_cages = &storage.cages;
                // tb_cages.blit_imap_cages(buf_idx, imap_cages, 0);
                // tb_cages.blit_pod_cages_localpoints(buf_idx, pod_cage_locals, 0);
                // tb_cages.blit_pod_cages_localpoints_bind(buf_idx, pod_cage_locals_bind, 0);
                // //tb_cages.blit_pod_cages_rotations(buf_idx, pod_cage_rotations, 0);
                // tb_cages.blit_pod_cages_covariants(buf_idx, pod_cage_covariants, 0);
                // tb_cages.blit_pod_cages_world_bind_reference(buf_idx, pod_cage_references, 0);

                // let output_rotations = tb_cages.view_pod_cages_rotations(buf_idx);
                // let out_rotations_len = output_rotations.len().min(cage_data.rotation.capacity());

                // const CAGE_SIZE: usize = crate::structure::cage::PER_CAGE_POINTS;
                // unsafe {
                //     let src = output_rotations.as_slice().as_ptr();
                //     let dst = pod_cage_rotations.as_ptr() as *mut [glam::Quat; CAGE_SIZE];
                //     std::ptr::copy_nonoverlapping(src, dst, out_rotations_len);
                // }
            }

            const VEC3_VEC4_PADDING: usize = 4;

            // fragments upload
            {
                let fragments = &storage.fragments;
                let pod_cage_ids = self.fragments.data().deformation_cage_slice();
                let pod_bind_pose = self.fragments.data().bind_position_slice();
                let pod_mesh_id = self.fragments.data().mesh_id_slice();

                // SAFETY: the use of LayoutFragmentData ensures we blit to a
                // valid section of the partitioned buffer.
                unsafe {
                    fragments.blit_part(
                        buf_idx,
                        LayoutFragmentData::PodCageIds as usize,
                        pod_cage_ids,
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
                let imap_nodes = self.lattice.nodes().slots_map();
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

        let sync_duration = t0.load(Ordering::Acquire);
        let t1 = Instant::now();
        let nanos = ((t1 - t00).as_nanos() as u64) - sync_duration;
        self.profiler
            .log_explicit("transfer", Duration::from_nanos(nanos));
        self.profiler.pop_trace();
    }

    fn fixed_step(
        &mut self,
        _input: &mut ethel::InputSystem,
        _screen: &mut janus::sync::Mirror<ScreenSpace>,
        _view_point: &janus::sync::TriCell<camera::ViewPoint>,
        delta: janus::context::DeltaTime,
    ) {
        if !self.sim_control.running {
            return;
        }

        self.perf_avg.tps_average.register(1, Instant::now());
        let delta = delta * self.sim_control.speed;

        self.profiler.capture_duration("lattice_update", || {
            const WIND_FORCE: f32 = 0.0;
            self.lattice
                .apply_forces_batched(glam::vec3(WIND_FORCE, -9.81, WIND_FORCE));
            self.lattice.update(delta);
        });
        self.profiler.capture_duration("debris_simulate", || {
            self.debris.simulate_bodies(delta);
        });
    }

    fn on_key_event(&mut self, event: KeyEvent) {
        self.local_keyev_buf.push(event);
    }

    fn on_new_frame(
        &mut self,
        input: &mut ethel::InputSystem,
        screen: &mut janus::sync::Mirror<ScreenSpace>,
        view_point: &janus::sync::TriCell<ViewPoint>,
        delta: janus::context::DeltaTime,
    ) {
        self.profiler.page();

        {
            // simulation control keys
            const SIM_SPEED_STEP: f32 = 0.01;
            if input.keys().key_pressed(janus::input::KeyCode::Home) {
                self.sim_control.running = !self.sim_control.running;
            }
            if input.keys().key_down(janus::input::KeyCode::PageUp) {
                self.sim_control.speed += SIM_SPEED_STEP;
            }
            if input.keys().key_down(janus::input::KeyCode::PageDown) {
                self.sim_control.speed -= SIM_SPEED_STEP;
                self.sim_control.speed = self.sim_control.speed.max(0.01);
            }
        }

        let t0 = Instant::now();
        self.textures_metadata_registry.pipe_sync_commands();

        if screen.resolution().is_changed() {
            self.ui_system.set_resolution(screen.resolution());
        }

        {
            let now = t0;
            let render_frame_time_millis = self.render_frame_time.get().as_millis();
            let simul_frame_time_millis = delta.as_millis();
            let fps = 1000.0 / render_frame_time_millis;

            self.perf_avg.fps_average.register(fps, now);
            self.perf_avg
                .render_time_average
                .register(render_frame_time_millis, now);
            self.perf_avg
                .simul_time_average
                .register(simul_frame_time_millis, now);
        }

        self.update_environment();
        self.ui_update_layout();
        self.ui_process_input(input.cursor(), delta);
        self.ui_system.process_widget_states(delta);

        let vp_prev = view_point.get();

        if !input.cursor_options().check_grabbed() {
            screen.sync().unwrap();
            self.select_lattice_raycast(input, screen.get(), view_point);
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

            let dw = input.mouse_wheel();
            *self.camera.distance_mut() -= dw * delta.as_f32() * 100.0;

            let _ = view_point.set_and_advance(*self.camera.viewpoint());
        }

        // shadow tricell with previous read viewpoint value
        // ensures we are still reading the correct frame despite the advance
        // operation
        let view_point = vp_prev;

        if input.keys().key_pressed(janus::input::KeyCode::KeyB) {
            self.debris.data_mut().clear();
            self.debris.clear_rubber();
        }
        if input.keys().key_pressed(janus::input::KeyCode::KeyP) {
            self.save_profiler_report();
        }
        if input.keys().key_pressed(janus::input::KeyCode::KeyH) {
            self.spawn_debug_structure(&view_point);
        }

        const CAMERA_KEY: janus::input::KeyCode = janus::input::KeyCode::Tab;
        if input.keys().key_pressed(CAMERA_KEY) {
            input.cursor_options().set_grabbed(true);
        }
        if input.keys().key_released(CAMERA_KEY) {
            input.cursor_options().set_grabbed(false);
        }

        self.local_keyev_buf.clear();
        self.lattice.clear_damage_buffers();
        let t1 = Instant::now();
        self.profiler.log_explicit("on_new_frame", t1 - t0);

        self.profiler.capture_duration("lattice_sync_integ", || {
            let fragments = FragmentsRowTableView::from(self.fragments.data());
            self.lattice.pull_integrity_mass(&fragments);
            self.lattice.sync_constraint_attributes();
        });

        self.profiler.capture_duration("lattice_trivialities", || {
            let deforms = CageRowTableView::from(self.cage.data());
            self.fragments.compute_world_positions(&deforms);

            // synchronizes lattice damage from xpbd-lattice solver
            self.lattice.register_dead_nodes();
        });

        //self.process_cage_damage();

        // synchronizes lattice and cage damage to fragments.
        // after this point, the order of fragment elements must not change;
        // i.e. there must be no free operations on the fragment table until
        // after the release_debris_bodies function.
        self.process_fragment_damage();
        self.release_debris_bodies();
        self.delete_disabled_fragments();

        self.profiler
            .capture_duration("cage_compute_covariants", || {
                self.cage.apply_rotations();
                let lattice = NodesRowTableView::from(self.lattice.nodes());
                self.cage.compute_covariants(&lattice);
            });

        self.profiler.capture_duration("debris_sleep", || {
            const SIM_SPEED_LOW_THRESHOLD: f32 = 0.75;
            if self.sim_control.running || self.sim_control.speed < SIM_SPEED_LOW_THRESHOLD {
                self.debris.accumulate_motion();
                self.debris.freeze_old_debris(delta);
            }
        });
        self.profiler.capture_duration("debris_hash", || {
            self.debris.hash_debris();
        });
    }
}

impl State {
    pub fn update_environment(&mut self) {
        use crate::ui::env_names::*;

        let env = self.ui_system.env_mut();
        if self.perf_avg.update() {
            let render_time_avg = self.perf_avg.render_time_average.average();
            let simul_time_avg = self.perf_avg.simul_time_average.average();
            let fps_avg = self.perf_avg.fps_average.average();
            let tps_total = self.perf_avg.tps_average.accumulated();
            let lattice_node_count = self.lattice.nodes().len();
            let lattice_constr_count = self.lattice.links().len();
            let fragment_count = self.fragments.data().len();
            let cages_count = self.cage.data().len();
            let debris_count = self.debris.data().len();

            env.insert(DEBUG_PERF_FPS_AVG, fps_avg as u32);
            env.insert(DEBUG_PERF_TPS_TOTAL, tps_total);
            env.insert(DEBUG_PERF_LAST_SIMUL_FRAME_TIME_MILLIS, simul_time_avg);
            env.insert(DEBUG_PERF_LAST_RENDER_FRAME_TIME_MILLIS, render_time_avg);
            env.insert(DEBUG_COUNTER_LATTICE_NODES, lattice_node_count);
            env.insert(DEBUG_COUNTER_LATTICE_CONSTRAINTS, lattice_constr_count);
            env.insert(DEBUG_COUNTER_FRAGMENTS, fragment_count);
            env.insert(DEBUG_COUNTER_CAGES, cages_count);
            env.insert(DEBUG_COUNTER_DEBRIS, debris_count);
        }

        env.insert(
            SIM_CTL_STATE,
            if self.sim_control.running {
                "running"
            } else {
                "paused"
            },
        );
        env.insert(SIM_CTL_SPEED, self.sim_control.speed);
    }

    pub fn ui_system(&self) -> &InterfaceSystem {
        &self.ui_system
    }

    pub fn ui_system_mut(&mut self) -> &mut InterfaceSystem {
        &mut self.ui_system
    }

    pub fn ui_update_layout(&mut self) {
        self.ui_system.invalidate_text_changes();
        self.ui_system.evaluate_layout();
        self.ui_system.synchronise_layout();
    }

    pub fn ui_process_input(&mut self, cursor: &Cursor, delta: DeltaTime) {
        let x = cursor.x_f32();
        let y = cursor.y_f32();
        self.ui_system.process_hover_events(x, y, delta);
        self.ui_system.feed_key_events(&self.local_keyev_buf, delta);
    }

    pub fn ui_composite_batches(&mut self) {
        self.ui_system.prepare_elements(&mut self.glyph_atlas);
        let texture_metadata = &self.textures_metadata_registry;
        self.ui_system.clear_compositor_layers();
        self.ui_system.composite_layers(texture_metadata);
    }

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

        // SAFETY: safe access to the selected commands buffer is guaranteed
        // by correct usage of the triple buffer section index
        let mut data = unsafe { buffer.view_section_mut(tri_section) };

        let il0 = command_queue.index() as u32;
        let next = command_queue.upload_next_group(&mut data);
        let length = command_queue.index() as u32 - il0;
        // SAFETY: safe access to the selected commands buffer is guaranteed
        // by correct usage of the triple buffer section index.
        // The set length is correct, as it is the length of the command queue
        // that has just been blitted to the buffer.
        unsafe {
            buffer.set_length(tri_section, length);
        }

        if let Some(next) = next {
            self.upload_gpu_commands(command_queue, next, frame_data, tri_section);
        }
    }

    fn delete_disabled_fragments(&mut self) {
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
    }

    fn release_debris_bodies(&mut self) {
        let disabled_frags = self.fragments.frame_disabled_frags();
        let count = self.fragments.frame_disabled_frags_count();
        if count > 0 {
            struct DebrisData {
                position: glam::Vec3,
                velocity: glam::Vec3,
                ang_velocity: glam::Vec3,
                forces: glam::Vec3,
                torque: glam::Vec3,
                mass: f32,
                mesh_id: ethel::mesh::Id,
            }

            let mut buffer = Vec::<DebrisData>::with_capacity(count);
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
        let damaged_nodes = self.lattice.unique_damaged_nodes_frame();
        self.profiler.capture_duration("lattice_damage", || {
            // self.deforms.sync_lattice_damage(damaged_nodes);
            // self.deforms.delete_dead_points();

            self.fragments.clear_damage_buffer();
            self.fragments.sync_lattice_damage(damaged_nodes);
        });
    }

    fn spawn_debug_structure(&mut self, view_point: &ViewPoint) {
        const WIDTH: f32 = 12.0;
        const HEIGHT: f32 = 6.0;
        const DEPTH: f32 = 12.0;
        const FLOORS: u32 = 6;
        const TOTAL_HEIGHT: f32 = HEIGHT * FLOORS as f32;

        let center = glam::vec3(view_point.position.x, GROUND_LEVEL, view_point.position.z);
        let lattice = create_structure_lattice(center, WIDTH, HEIGHT, DEPTH, FLOORS);

        const INNER_SPACE: i32 = 3;

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
        {
            const FRAME_COUNT_MIN: usize = 1000;
            let frame_count = self.profiler.frames_stack().len();
            if frame_count < FRAME_COUNT_MIN {
                event!(
                    Level::WARN,
                    "User requested profiler report but frames count is too little: expected atleast {}, but only got {}.",
                    FRAME_COUNT_MIN,
                    frame_count
                )
            }
        }

        let path = PathBuf::from_str("framestack_latest.bin").unwrap();
        if path.exists() {
            if let Ok(creation_time) = path.metadata().map(|meta| meta.created()).flatten() {
                let date: chrono::DateTime<chrono::Utc> = creation_time.into();
                let formatted_date = date.format("%d_%m_%y-%H_%M_%S");

                let name = format!("framestack_old_{}", formatted_date);
                let duplicates = std::fs::read_dir(".")
                    .unwrap()
                    .filter_map(|f| f.ok())
                    .filter(|f| f.file_name().into_string().unwrap().starts_with(&name))
                    .count();

                let d_identifier = if duplicates != 0 {
                    format!("_{duplicates}")
                } else {
                    String::new()
                };

                let new_path = format!("framestack_old_{}{d_identifier}.bin", formatted_date);

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
        input: &ethel::InputSystem,
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
            const RAY_SIZE: f32 = 0.1;

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
        self.fragments
            .create_deformation_cages(&lattice_hash, &lattice, &mut self.cage);
    }
}
