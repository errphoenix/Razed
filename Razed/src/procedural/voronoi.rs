use ethel::mesh::MeshStaging;
use polys::{Plane, convex::Convex};
use rand::{Rng, RngExt};

#[derive(Debug)]
pub struct CubeVoronoi {
    pub stager: MeshStaging,
}

#[derive(Debug, Default, Clone)]
pub struct CubeVoronoiGenerator<R: Rng> {
    rng: R,
    max_offset: f32,

    seeds: Vec<glam::Vec3>,
    meshes: Vec<Convex<polys::TriFace>>,
}

impl<R: Rng> CubeVoronoiGenerator<R> {
    pub fn new(rng: R, max_offset: f32) -> Self {
        Self {
            rng: rng,
            max_offset,
            seeds: Vec::new(),
            meshes: Vec::new(),
        }
    }

    pub fn generate(
        &mut self,
        seed_input: &[glam::Vec3],
        volume: glam::Vec3,
        unit: glam::Vec3,
        seek_range: f32,
    ) {
        self.seeds.clear();
        self.seeds.extend_from_slice(seed_input);

        let offset_seeds = {
            let mut seeds = self.seeds.clone();
            let h_offset = self.max_offset * 0.5;
            if self.max_offset != 0.0 {
                seeds.iter_mut().for_each(|p| {
                    p.x += self.rng.random_range(-h_offset..h_offset);
                    p.y += self.rng.random_range(-h_offset..h_offset);
                    p.z += self.rng.random_range(-h_offset..h_offset);
                });
            }
            seeds
        };

        let half_volume = volume * 0.5;
        let half_unit = unit * 0.5;

        for i in 0..self.seeds.len() {
            // subject mesh (original seed)
            let seed = offset_seeds[i];
            let mut mesh = Convex::<Vec<u32>>::parallelepiped(half_volume);
            mesh.translate(half_volume - half_unit);

            let mut clip_mesh = polys::clip::ClipMesh::new(mesh);

            for j in 0..self.seeds.len() {
                if i == j {
                    continue;
                }

                // test mesh (rng offset seed)
                let other = offset_seeds[j];

                if (other - seed).length().abs() > seek_range {
                    continue;
                }

                let m = seed.midpoint(other);
                let normal = (seed - other).normalize();
                let d = normal.dot(m);

                let plane = Plane::new(normal, d);
                clip_mesh.process_vertices(&plane);
                clip_mesh.process_edges();
                clip_mesh.process_faces(&plane);
            }

            let mut mesh = clip_mesh.finish();

            let centroid = mesh.centroid();
            mesh.make_local();
            mesh.translate(centroid - self.seeds[i]);

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

                //TODO: add compute uv pass to polys crate
                // after that, resume material implementation for
                // fragments_draw pass

                t_vb.push(ethel::mesh::Vertex {
                    pos_x: p0.x,
                    pos_y: p0.y,
                    pos_z: p0.z,
                    norm_x: n.x,
                    norm_y: n.y,
                    norm_z: n.z,
                    uv_x: 0f32,
                    uv_y: 0f32,
                });
                t_vb.push(ethel::mesh::Vertex {
                    pos_x: p1.x,
                    pos_y: p1.y,
                    pos_z: p1.z,
                    norm_x: n.x,
                    norm_y: n.y,
                    norm_z: n.z,
                    uv_x: 0f32,
                    uv_y: 0f32,
                });
                t_vb.push(ethel::mesh::Vertex {
                    pos_x: p2.x,
                    pos_y: p2.y,
                    pos_z: p2.z,
                    norm_x: n.x,
                    norm_y: n.y,
                    norm_z: n.z,
                    uv_x: 0f32,
                    uv_y: 0f32,
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
