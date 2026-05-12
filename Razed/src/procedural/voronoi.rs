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

    pub fn generate(&mut self, seed_input: &[glam::Vec3], area: glam::Vec3, seek_range: f32) {
        self.seeds.clear();
        self.seeds.extend_from_slice(seed_input);

        let outer_planes = {
            let hx = area.x * 0.5;
            let hy = area.y * 0.5;
            let hz = area.z * 0.5;

            vec![
                Plane::new(glam::Vec3::X, -hx),
                Plane::new(-glam::Vec3::X, hx),
                Plane::new(glam::Vec3::Y, -hy),
                Plane::new(-glam::Vec3::Y, hy),
                Plane::new(glam::Vec3::Z, -hz),
                Plane::new(-glam::Vec3::Z, hz),
            ]
        };

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

        for i in 0..self.seeds.len() {
            // subject mesh (original seed)
            let seed = self.seeds[i];
            let mut mesh = Convex::<Vec<u32>>::parallelepiped(glam::Vec3::splat(0.5));
            mesh.translate(seed);

            let mut clip_mesh = polys::clip::ClipMesh::new(mesh);

            for j in (i + 1)..self.seeds.len() {
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
            mesh.translate(centroid - seed);

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
