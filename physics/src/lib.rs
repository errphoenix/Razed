pub mod xpbd;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Line {
    pub dir: glam::Vec3,
}

impl From<glam::Vec3> for Line {
    fn from(value: glam::Vec3) -> Self {
        Self::from_vector(value)
    }
}

impl Line {
    pub fn from_vector(vector: glam::Vec3) -> Self {
        Self { dir: vector }
    }

    pub fn into_ray(self, p: glam::Vec3) -> Ray {
        Ray {
            origin: p,
            line: self,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ray {
    pub origin: glam::Vec3,
    pub line: Line,
}

impl Ray {
    pub fn new(origin: glam::Vec3, dir: glam::Vec3) -> Self {
        Self {
            origin,
            line: Line { dir },
        }
    }

    pub fn as_line(self) -> Line {
        self.line
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Segment {
    pub start: glam::Vec3,
    pub end: glam::Vec3,
}

impl Segment {
    pub fn new(a: glam::Vec3, b: glam::Vec3) -> Self {
        Self { start: a, end: b }
    }

    pub fn to_line(self) -> Line {
        Line::from_vector(self.direction())
    }

    pub fn direction(&self) -> glam::Vec3 {
        let d = self.direction_u();
        d / d.length()
    }

    pub fn direction_u(&self) -> glam::Vec3 {
        self.end - self.start
    }

    pub fn length_squared(&self) -> f32 {
        self.start.distance_squared(self.end)
    }
}

impl From<(glam::Vec3, glam::Vec3)> for Segment {
    fn from((a, b): (glam::Vec3, glam::Vec3)) -> Self {
        Self { start: a, end: b }
    }
}

const EPSILON: f32 = 1e-5;

/// Ray/Ray intersection from Ronald Goldman's "Intersection of Two Lines in
/// Three-Space", from "Graphics Gems".
pub fn intersect_ray_segment(
    ray: impl Into<Ray>,
    segment: impl Into<Segment>,
    threshold: f32,
) -> Option<f32> {
    let ray = ray.into();
    let segment = segment.into();

    let d1 = ray.line.dir;
    let d2 = segment.direction_u();
    let w = segment.start - ray.origin;

    let a = d1.dot(d1);
    let e = d2.dot(d2);
    let b = d1.dot(d2);
    let c = w.dot(d1);
    let f = w.dot(d2);

    let denom = a * e - b * b;

    let (t1, t2) = if denom.abs() < EPSILON {
        let t2 = (f / e).clamp(0.0, 1.0);
        let t1 = ((c + b * t2) / a).max(0.0);
        (t1, t2)
    } else {
        let t1 = (e * c - b * f) / denom;
        let t2 = (b * c - a * f) / denom;

        let t2_clamped = t2.clamp(0.0, 1.0);

        let t1 = if t2 != t2_clamped {
            ((c + b * t2_clamped) / a).max(0.0)
        } else {
            t1.max(0.0)
        };

        (t1, t2_clamped)
    };

    let on_ray = ray.origin + t1 * d1;
    let on_segment = segment.start + t2 * d2;

    let dist_sq = on_ray.distance_squared(on_segment);
    if dist_sq <= threshold * threshold {
        Some(t1)
    } else {
        None
    }
}

impl From<ethel::state::camera::ViewPoint> for Ray {
    fn from(value: ethel::state::camera::ViewPoint) -> Self {
        Ray::new(value.position, value.forward())
    }
}
