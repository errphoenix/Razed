use ethel::mesh::MeshStaging;
use polys::{Plane, convex::Convex};
use rand::{Rng, RngExt};

use crate::procedural::VoxelGrid;

#[derive(Debug)]
pub struct CubeVoronoi {
    pub stager: MeshStaging,
}

#[derive(Debug, Default, Clone)]
pub struct CubeVoronoiGenerator {
    seeds: Vec<glam::Vec3>,
    meshes: Vec<Convex<polys::TriFace>>,
}

impl CubeVoronoiGenerator {
    pub fn new(grid: VoxelGrid, rng: &mut impl Rng, max_offset: f32) -> Self {
        let mut seeds = Vec::with_capacity(grid.count());
        grid.to_world(glam::Vec3::ZERO, &mut seeds);

        seeds.iter_mut().for_each(|p| {
            p.x += rng.random_range(-max_offset..max_offset);
            p.y += rng.random_range(-max_offset..max_offset);
            p.z += rng.random_range(-max_offset..max_offset);
        });

        Self {
            seeds,
            meshes: Vec::with_capacity(grid.count()),
        }
    }

    pub fn generate(&mut self) {
        const MAX_RANGE: f32 = 3.0;

        for i in 0..self.seeds.len() {
            // todo: parallelepiped instead of cube
            let mut mesh = Convex::<Vec<u32>>::unit_cube(3.0);
            let seed = self.seeds[i];

            for j in 0..self.seeds.len() {
                if i == j {
                    continue;
                }

                let other = self.seeds[j];

                if (other - seed).length() > MAX_RANGE {
                    continue;
                }

                let m = seed.midpoint(other);
                let normal = (other - seed).normalize();
                let d = normal.dot(m);
                mesh = mesh.clip_plane(Plane::new(normal, d));

                if mesh.vertices().is_empty() {
                    break;
                }
            }

            self.meshes.push(mesh.triangulate());
        }
    }

    pub fn meshes(&self) -> &[Convex<polys::TriFace>] {
        &self.meshes
    }

    pub fn meshes_mut(&mut self) -> &mut Vec<Convex<polys::TriFace>> {
        &mut self.meshes
    }

    pub fn consolidate(&self, mut stager: MeshStaging) -> CubeVoronoi {
        let mut t_vb = Vec::new();

        for mesh in &self.meshes {
            for face in mesh.faces() {
                let polys::TriFace { a, b, c } = face.indexed;
                let n = face.normal;

                let p0 = mesh.vertices()[a as usize];
                let p1 = mesh.vertices()[b as usize];
                let p2 = mesh.vertices()[c as usize];

                t_vb.push(ethel::mesh::Vertex {
                    position: [p0.x, p0.y, p0.z, 1.0],
                    normal: [n.x, n.y, n.z, 0.0],
                });
                t_vb.push(ethel::mesh::Vertex {
                    position: [p1.x, p1.y, p1.z, 1.0],
                    normal: [n.x, n.y, n.z, 0.0],
                });
                t_vb.push(ethel::mesh::Vertex {
                    position: [p2.x, p2.y, p2.z, 1.0],
                    normal: [n.x, n.y, n.z, 0.0],
                });
            }

            stager.stage(&t_vb);
            t_vb.clear();
        }

        CubeVoronoi { stager }
    }

    pub fn consolidate_alloc(&self) -> CubeVoronoi {
        let stager = MeshStaging::new();
        self.consolidate(stager)
    }
}
