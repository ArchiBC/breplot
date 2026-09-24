use brepkit_math::vec::Point3;

#[derive(Clone, Copy, Debug)]
pub struct Cubic3 {
    pub points: [Point3; 4],
}

impl Cubic3 {
    pub fn evaluate(&self, t: f64) -> Point3 {
        let [a, b, c, d] = self.points;
        let mt = 1.0 - t;
        a + (b - a) * (3.0 * mt * mt * t) + (c - a) * (3.0 * mt * t * t) + (d - a) * (t * t * t)
    }

    pub fn split(&self, t: f64) -> (Self, Self) {
        let [p0, p1, p2, p3] = self.points;
        let a = p0 + (p1 - p0) * t;
        let b = p1 + (p2 - p1) * t;
        let c = p2 + (p3 - p2) * t;
        let d = a + (b - a) * t;
        let e = b + (c - b) * t;
        let f = d + (e - d) * t;
        (
            Self {
                points: [p0, a, d, f],
            },
            Self {
                points: [f, e, c, p3],
            },
        )
    }

    pub fn portion(&self, start: f64, end: f64) -> Self {
        let start = start.clamp(0.0, 1.0);
        let end = end.clamp(start, 1.0);
        let (_, tail) = self.split(start);
        let local_end = if start == 1.0 {
            0.0
        } else {
            (end - start) / (1.0 - start)
        };
        tail.split(local_end).0
    }
}
