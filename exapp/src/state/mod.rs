pub(crate) mod physics;

use std::time::Instant;

use crate::{
    data::{FrameDataBuffers, LayoutEntityData, LayoutXpbdDebugData, Renderable},
    state::physics::XpbdSystem,
};
use ::physics::xpbd::{LatticeIds, XpbdLatticeBuilder, XpbdLinkOptions, XpbdNodeOptions};
use ethel::{
    render::command::DrawArraysIndirectCommand,
    state::{camera, data::Column},
};
use tracing::event;

ethel::table_spec! {
    struct EntityData {
        position: glam::Vec4;
        rotation: glam::Quat;
        scale: glam::Vec4;
    }
}

#[derive(Debug, Default)]
pub struct State {
    renderables: Vec<Renderable>,
    mesh_ids: Vec<ethel::mesh::Id>,

    entity_data: EntityDataRowTable,
    xpbd: XpbdSystem,

    // maps entity id to xpbd node handle
    node_map: Vec<u32>,

    camera: camera::Orbital,
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
        self.renderables.iter().for_each(|_| {
            command_queue.push(DrawArraysIndirectCommand {
                count: 6,
                instance_count: 1,
                first_vertex: 0,
                base_instance: 0,
            });
        });

        frame_boundary.cross(|section, storage| {
            let buf_idx = section.as_index();

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

                const VEC3_VEC4_PADDING: usize = 4;

                // SAFETY: the use of LayoutXpbdDebugData ensures we are
                // blitting to a valid section of the xpbd_dbg partitioned
                // buffer.
                unsafe {
                    xpbd_dbg.blit_part(buf_idx, LayoutXpbdDebugData::Constraints as usize, constraints, 0);
                    xpbd_dbg.blit_part(buf_idx, LayoutXpbdDebugData::ImapNodes as usize, imap_nodes, 0);
                    xpbd_dbg.blit_part_padded(buf_idx, LayoutXpbdDebugData::PodNodes as usize, pod_nodes, 0, VEC3_VEC4_PADDING);
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
        input: &ethel::InputSystem,
        view_point: &mut janus::sync::Mirror<camera::ViewPoint>,
        delta: janus::context::DeltaTime,
    ) {
        let _ = view_point.sync();

        let (dx, dy) = input.cursor().delta_f32();
        let (dx, dy) = (dx.to_radians(), dy.to_radians());
        self.camera.update(dx, dy);

        let dw = *input.mouse_wheel();
        *self.camera.distance_mut() -= dw * delta.as_f32() * 100.0;

        view_point.publish_with(|vp| {
            *vp = *self.camera.viewpoint();
        });

        self.xpbd.apply_forces_batched(glam::vec3(0., -9.81, 0.0));
        self.xpbd.update(delta);
        {
            let len = self.entity_data.len();
            let p_pos = self.xpbd.nodes().current_pos_slice();
            let e_pos = self.entity_data.position_mut_slice();

            for i in 1..len {
                let pos = unsafe { e_pos.get_unchecked_mut(i) };
                let imap = self.node_map[i];
                let phys_pos = *unsafe { p_pos.get_unchecked(imap as usize) };
                *pos = glam::vec4(phys_pos.x, phys_pos.y, phys_pos.z, 1.0);
            }
        }

        // random demo
        if input.keys().key_pressed(janus::input::KeyCode::KeyH) {
            let vp = view_point.get();
            let view_pos = vp.position;
            let view_fw = vp.forward();

            let mut lattice = XpbdLatticeBuilder::with_capacity(20);

            const MASS: f32 = 80.0;
            const COMPLIANCE: f32 = 0.0025;
            const SPACING: f32 = 2.0;

            let p0 = view_pos + view_fw * 3.0;
            let root = lattice.node(XpbdNodeOptions::new(
                p0 + glam::vec3(-SPACING, -SPACING, -SPACING),
                MASS,
            ));
            let bot_left = lattice.node(XpbdNodeOptions::new(
                p0 + glam::vec3(SPACING, -SPACING, -SPACING),
                MASS,
            ));
            let top_right = lattice.node(XpbdNodeOptions::new(
                p0 + glam::vec3(SPACING, SPACING, -SPACING),
                MASS,
            ));
            lattice.node(XpbdNodeOptions::new(
                p0 + glam::vec3(-SPACING, SPACING, -SPACING),
                MASS,
            ));

            let back_corner = lattice.node(XpbdNodeOptions::new(
                p0 + glam::vec3(-SPACING, SPACING, SPACING),
                MASS,
            ));
            lattice.node(XpbdNodeOptions::new(
                p0 + glam::vec3(-SPACING, -SPACING, SPACING),
                MASS,
            ));
            lattice.link_to(root, XpbdLinkOptions::new(COMPLIANCE));

            lattice.node(XpbdNodeOptions::new(
                p0 + glam::vec3(SPACING, -SPACING, SPACING),
                MASS,
            ));
            lattice.link_to(bot_left, XpbdLinkOptions::new(COMPLIANCE));
            lattice.node(XpbdNodeOptions::new(
                p0 + glam::vec3(SPACING, SPACING, SPACING),
                MASS,
            ));
            lattice.link_to(
                top_right,
                XpbdLinkOptions::with_rest_length(COMPLIANCE, 3.8),
            );
            lattice.link_to(back_corner, XpbdLinkOptions::new(COMPLIANCE));
            lattice.link(XpbdLinkOptions::new(COMPLIANCE));
            lattice.link(XpbdLinkOptions::new(COMPLIANCE));

            lattice.link(XpbdLinkOptions::new(COMPLIANCE));
            lattice.link(XpbdLinkOptions::new(COMPLIANCE));
            lattice.link_to(root, XpbdLinkOptions::new(COMPLIANCE));
            lattice.link(XpbdLinkOptions::new(COMPLIANCE));

            lattice.link(XpbdLinkOptions::new(COMPLIANCE));
            lattice.link(XpbdLinkOptions::new(COMPLIANCE));

            let map = self.create_lattice(lattice);
            let map = map.nodes;

            if self.node_map.len() == 0 {
                self.node_map.push(0);
            }
            self.node_map.reserve(map.len());
            for n_i in map {
                self.node_map.push(n_i);
            }

            self.xpbd.nodes_mut().inv_mass_mut_slice()[1] = 0.0;
            self.xpbd.nodes_mut().inv_mass_mut_slice()[2] = 0.0;
            self.xpbd.nodes_mut().inv_mass_mut_slice()[6] = 0.0;
            self.xpbd.nodes_mut().inv_mass_mut_slice()[7] = 0.0;
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

    pub fn create_lattice(&mut self, lattice: XpbdLatticeBuilder) -> LatticeIds {
        let lattice_map = self.xpbd.import_lattice(lattice);

        for &node_id in &lattice_map.nodes {
            let position = {
                let nodes = self.xpbd.nodes();
                let pos_id = unsafe { nodes.get_indirect_unchecked(node_id) };
                *unsafe { nodes.current_pos_slice().get_unchecked(pos_id as usize) }
            };

            self.create_renderable(0, position, Default::default(), glam::Vec3::ONE);
        }

        lattice_map
    }
}
