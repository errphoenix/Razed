pub mod clip;
pub mod convex;
pub mod post_process;

#[allow(unused_imports)]
pub use post_process::compute_normals;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct QuadFace {
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub d: u32,
}

impl From<[u32; 4]> for QuadFace {
    fn from(value: [u32; 4]) -> Self {
        Self {
            a: value[0],
            b: value[1],
            c: value[2],
            d: value[3],
        }
    }
}

impl QuadFace {
    pub const fn triangulate(self) -> (TriFace, TriFace) {
        (
            TriFace {
                a: self.a,
                b: self.b,
                c: self.c,
            },
            TriFace {
                a: self.c,
                b: self.b,
                c: self.d,
            },
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct TriFace {
    pub a: u32,
    pub b: u32,
    pub c: u32,
}

impl From<[u32; 3]> for TriFace {
    fn from(value: [u32; 3]) -> Self {
        Self {
            a: value[0],
            b: value[1],
            c: value[2],
        }
    }
}

impl std::ops::Index<usize> for TriFace {
    type Output = u32;

    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.a,
            1 => &self.b,
            2 => &self.c,
            _ => panic!("cannot index above 2 in triangular face"),
        }
    }
}

impl std::ops::Index<usize> for QuadFace {
    type Output = u32;

    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.a,
            1 => &self.b,
            2 => &self.c,
            3 => &self.d,
            _ => panic!("cannot index above 3 in quadrilateral face"),
        }
    }
}

pub trait Face: std::ops::Index<usize, Output = u32> + Clone {
    fn len(&self) -> usize;

    fn into_alloc(self) -> Vec<u32>;

    fn iter_indices(&self) -> impl std::iter::Iterator<Item = u32>;
}

impl IntoIterator for TriFace {
    type Item = u32;

    type IntoIter = std::array::IntoIter<u32, 3>;

    fn into_iter(self) -> Self::IntoIter {
        [self.a, self.b, self.c].into_iter()
    }
}

impl IntoIterator for QuadFace {
    type Item = u32;

    type IntoIter = std::array::IntoIter<u32, 4>;

    fn into_iter(self) -> Self::IntoIter {
        [self.a, self.b, self.c, self.d].into_iter()
    }
}

impl Face for TriFace {
    fn len(&self) -> usize {
        3
    }

    fn into_alloc(self) -> Vec<u32> {
        vec![self.a, self.b, self.c]
    }

    fn iter_indices(&self) -> impl std::iter::Iterator<Item = u32> {
        self.into_iter()
    }
}

impl Face for QuadFace {
    fn len(&self) -> usize {
        4
    }

    fn into_alloc(self) -> Vec<u32> {
        let a = [0u32; 4];
        vec![self.a, self.b, self.c, self.d]
    }

    fn iter_indices(&self) -> impl std::iter::Iterator<Item = u32> {
        self.into_iter()
    }
}

impl<const N: usize> Face for [u32; N] {
    fn len(&self) -> usize {
        N
    }

    fn into_alloc(self) -> Vec<u32> {
        let mut vec = Vec::with_capacity(N);
        for e in self {
            vec.push(e);
        }
        vec
    }

    fn iter_indices(&self) -> impl std::iter::Iterator<Item = u32> {
        self.iter().copied()
    }
}

impl Face for Vec<u32> {
    fn len(&self) -> usize {
        self.len()
    }

    fn into_alloc(self) -> Vec<u32> {
        self
    }

    fn iter_indices(&self) -> impl std::iter::Iterator<Item = u32> {
        self.iter().copied()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Facen<F: Face> {
    pub indexed: F,
    pub normal: glam::Vec3,
}

pub type DynFace = Box<dyn Face>;

impl<F: Face> std::ops::Index<usize> for Facen<F> {
    type Output = u32;

    fn index(&self, index: usize) -> &Self::Output {
        &self.indexed[index]
    }
}

impl<F: Face> Face for Facen<F> {
    fn len(&self) -> usize {
        self.indexed.len()
    }

    fn into_alloc(self) -> Vec<u32> {
        self.indexed.into_alloc()
    }

    fn iter_indices(&self) -> impl std::iter::Iterator<Item = u32> {
        self.indexed.iter_indices()
    }
}

impl<F: Face> Facen<F> {
    pub fn new(face: impl Into<F>, normal: glam::Vec3) -> Self {
        Self {
            indexed: face.into(),
            normal,
        }
    }

    pub fn into_alloc_n(self) -> Facen<Vec<u32>> {
        Facen {
            indexed: self.indexed.into_alloc(),
            normal: self.normal,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Plane {
    pub normal: glam::Vec3,
    pub d: f32,
}

impl Plane {
    pub const fn new(normal: glam::Vec3, d: f32) -> Self {
        Self { normal, d }
    }
}
