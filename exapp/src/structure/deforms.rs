pub const CONTROL_POINTS_COUNT: usize = 8;

ethel::table_spec! {
    struct Deforms {
        bind: glam::Vec3; // the base points of the bind pose
        deformed: glam::Vec3; // current deformed points

        controllers: [u32; CONTROL_POINTS_COUNT];
        weights: [f32; CONTROL_POINTS_COUNT];
    }
}

#[derive(Debug, Default)]
pub struct DeformSystem {
    data: DeformsRowTable,
}

impl DeformSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: DeformsRowTable::with_capacity(capacity),
        }
    }
}
