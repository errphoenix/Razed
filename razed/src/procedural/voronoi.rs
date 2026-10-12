use ethel::mesh::{MeshStaging, Triangle, Vertex};
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

    curr_volume: glam::Vec3,

    seeds: Vec<glam::Vec3>,
    meshes: Vec<Convex<polys::TriFace>>,
}

impl<R: Rng> CubeVoronoiGenerator<R> {
    pub const fn new(rng: R, max_offset: f32) -> Self {
        Self {
            rng: rng,
            max_offset,
            seeds: Vec::new(),
            meshes: Vec::new(),
            curr_volume: glam::Vec3::ZERO,
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

        self.curr_volume = volume;
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
            let mut mapped = polys::MappedMesh::new(mesh);

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
                mapped.clip_process_vertices(&plane);
                mapped.clip_process_edges();
                mapped.clip_process_faces(&plane);
            }
            mapped.preserve_hard_edges(30f32.to_radians().cos());

            let mut mesh = mapped.unmap();

            let centroid = mesh.centroid();
            mesh.make_local();
            mesh.translate(centroid - self.seeds[i]);

            self.meshes.push(mesh.triangulate());
        }
    }

    #[allow(unused)]
    pub fn meshes(&self) -> &[Convex<polys::TriFace>] {
        &self.meshes
    }

    #[allow(unused)]
    pub fn meshes_mut(&mut self) -> &mut Vec<Convex<polys::TriFace>> {
        &mut self.meshes
    }

    pub fn consolidate(&self, mut stager: MeshStaging) -> CubeVoronoi {
        let mut t_vb = Vec::new();
        let mut t_nb = Vec::new();
        let mut t_tb = Vec::new();

        let uv_scaling = 1.0 / self.curr_volume;

        for mesh in &self.meshes {
            t_nb.resize(mesh.vertices().len(), glam::Vec3::ZERO);
            polys::compute_vertex_normals(mesh.faces(), mesh.vertices(), &mut t_nb);
            for tri in mesh.faces() {
                t_tb.push(Triangle {
                    v0: tri[0],
                    v1: tri[1],
                    v2: tri[2],
                });
            }
            for (&v, &n) in mesh.vertices().iter().zip(&t_nb) {
                let uv = polys::compute_uv_cubic(v, n, uv_scaling);
                t_vb.push(Vertex {
                    pos_x: v.x,
                    pos_y: v.y,
                    pos_z: v.z,
                    norm_x: n.x,
                    norm_y: n.y,
                    norm_z: n.z,
                    uv_x: uv.x,
                    uv_y: uv.y,
                });
            }

            stager.stage(&t_vb, &t_tb);
            t_vb.clear();
            t_nb.clear();
            t_tb.clear();
        }

        CubeVoronoi { stager }
    }

    pub fn consolidate_alloc(&self) -> CubeVoronoi {
        let stager = MeshStaging::new();
        self.consolidate(stager)
    }
}
