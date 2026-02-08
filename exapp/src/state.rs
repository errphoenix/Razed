use crate::data::{Entity, FrameDataBuffers, LayoutEntityData};
use ethel::{
    render::command::DrawArraysIndirectCommand,
    state::{
        camera,
        data::{Column, ParallelIndexArrayColumn, column::IterColumn},
    },
};
use tracing::event;

#[derive(Debug, Default)]
pub struct State {
    entities: Vec<Entity>,
    mesh_ids: Vec<ethel::mesh::Id>,

    positions: ParallelIndexArrayColumn<glam::Vec4>,
    rotations: ParallelIndexArrayColumn<glam::Quat>,

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
        self.entities.iter().for_each(|_| {
            command_queue.push(DrawArraysIndirectCommand {
                count: 3,
                instance_count: 1,
                first_vertex: 0,
                base_instance: 0,
            });
        });

        frame_boundary.cross(|section, storage| {
            let buf_idx = section.as_index();

            {
                let scene = &storage.scene;

                let entity_map = &self.entities;
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

                let imap_positions = self.positions.handles();
                let imap_rotations = self.rotations.handles();
                let pod_positions = self.positions.contiguous();
                let pod_rotations = self.rotations.contiguous();
                unsafe {
                    scene.blit_part(
                        buf_idx,
                        LayoutEntityData::ImapPositions as usize,
                        imap_positions,
                        0,
                    );
                    scene.blit_part(
                        buf_idx,
                        LayoutEntityData::ImapRotations as usize,
                        imap_rotations,
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

        // random demo
        if input.keys().key_pressed(janus::input::KeyCode::KeyH) {
            for _ in 0..32 {
                let position = {
                    let x = rand::random::<f32>() * 64.0 - 32.0;
                    let y = rand::random::<f32>() * 32.0 - 16.0;
                    let z = rand::random::<f32>() * 64.0 - 32.0;
                    glam::Vec3::new(x, y, z)
                };
                let rotation = {
                    let x = rand::random::<f32>() * std::f32::consts::PI;
                    let y = rand::random::<f32>() * std::f32::consts::PI;
                    let z = rand::random::<f32>() * std::f32::consts::PI;
                    glam::Quat::from_euler(glam::EulerRot::YXZ, y, x, z)
                };

                self.create_entity(0, position, rotation);
            }
        }
    }
}

impl State {
    pub fn create_entity(
        &mut self,
        mesh_id: u32,
        position: glam::Vec3,
        rotation: glam::Quat,
    ) -> u32 {
        let position = glam::Vec4::new(position.x, position.y, position.z, 1.0);

        let position_handle = self.positions.put(position);
        let rotation_handle = self.rotations.put(rotation);

        let entity = Entity {
            mesh_id,
            position_handle,
            rotation_handle,
            _pad: 0,
        };

        let id = self.entities.len();
        self.entities.push(entity);
        id as u32
    }
}
