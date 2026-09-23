//! Adaptive cubic Bézier approximation of the original B-Rep edge curves.
//! Visibility is tested on a fine chordal proxy; the display path keeps cubics.

use brepkit_math::vec::Point3;
use brepkit_topology::edge::EdgeCurve;

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

fn hermite(
    curve: &EdgeCurve,
    start: Point3,
    end: Point3,
    a: f64,
    b: f64,
    range: (f64, f64),
) -> Cubic3 {
    let p0 = curve.evaluate_with_endpoints(a, start, end);
    let p3 = curve.evaluate_with_endpoints(b, start, end);
    let derivative = |t: f64| {
        let span = (range.1 - range.0).abs().max(1e-12);
        let h = span * 1e-5;
        let lo = (t - h).clamp(range.0.min(range.1), range.0.max(range.1));
        let hi = (t + h).clamp(range.0.min(range.1), range.0.max(range.1));
        if hi == lo {
            return p3 - p0;
        }
        (curve.evaluate_with_endpoints(hi, start, end)
            - curve.evaluate_with_endpoints(lo, start, end))
            * (1.0 / (hi - lo))
    };
    let delta = b - a;
    Cubic3 {
        points: [
            p0,
            p0 + derivative(a) * (delta / 3.0),
            p3 - derivative(b) * (delta / 3.0),
            p3,
        ],
    }
}

pub fn approximate_edge(
    curve: &EdgeCurve,
    start: Point3,
    end: Point3,
    tolerance: f64,
) -> Vec<Cubic3> {
    let range = curve.domain_with_endpoints(start, end);
    let mut result = Vec::new();
    fn recurse(
        curve: &EdgeCurve,
        start: Point3,
        end: Point3,
        range: (f64, f64),
        a: f64,
        b: f64,
        tolerance: f64,
        level: u8,
        result: &mut Vec<Cubic3>,
    ) {
        let cubic = hermite(curve, start, end, a, b, range);
        let error = [0.25, 0.5, 0.75]
            .into_iter()
            .map(|t| {
                let actual = curve.evaluate_with_endpoints(a + (b - a) * t, start, end);
                (actual - cubic.evaluate(t)).length()
            })
            .fold(0.0_f64, f64::max);
        let polygon_length: f64 = cubic
            .points
            .windows(2)
            .map(|p| (p[1] - p[0]).length())
            .sum();
        if level < 18 && (error > tolerance || polygon_length > 8.0) {
            let mid = (a + b) * 0.5;
            recurse(
                curve,
                start,
                end,
                range,
                a,
                mid,
                tolerance,
                level + 1,
                result,
            );
            recurse(
                curve,
                start,
                end,
                range,
                mid,
                b,
                tolerance,
                level + 1,
                result,
            );
        } else {
            result.push(cubic);
        }
    }
    recurse(
        curve,
        start,
        end,
        range,
        range.0,
        range.1,
        tolerance,
        0,
        &mut result,
    );
    result
}

/// Fine proxy samples, each tagged with its parameter on the cubic.
pub fn flatten(cubic: &Cubic3, max_length: f64, flatness: f64) -> Vec<(f64, Point3)> {
    let mut result = vec![(0.0, cubic.points[0])];
    fn recurse(
        cubic: Cubic3,
        t0: f64,
        t1: f64,
        max_length: f64,
        flatness: f64,
        level: u8,
        out: &mut Vec<(f64, Point3)>,
    ) {
        let [a, b, c, d] = cubic.points;
        let chord_mid = a + (d - a) * 0.5;
        let deviation = (cubic.evaluate(0.5) - chord_mid).length();
        let chord_length = (d - a).length();
        let control_length = (b - a).length() + (c - b).length() + (d - c).length();
        if level < 20
            && (chord_length > max_length
                || deviation > flatness
                || control_length > max_length * 1.5)
        {
            let (left, right) = cubic.split(0.5);
            let tm = (t0 + t1) * 0.5;
            recurse(left, t0, tm, max_length, flatness, level + 1, out);
            recurse(right, tm, t1, max_length, flatness, level + 1, out);
        } else {
            out.push((t1, d));
        }
    }
    recurse(*cubic, 0.0, 1.0, max_length, flatness, 0, &mut result);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use brepkit_math::curves::Circle3D;
    use brepkit_math::vec::Vec3;

    #[test]
    fn split_preserves_cubic() {
        let cubic = Cubic3 {
            points: [
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 2.0, 0.0),
                Point3::new(2.0, 2.0, 0.0),
                Point3::new(3.0, 0.0, 0.0),
            ],
        };
        let part = cubic.portion(0.2, 0.8);
        for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
            assert!((part.evaluate(t) - cubic.evaluate(0.2 + 0.6 * t)).length() < 1e-10);
        }
    }

    #[test]
    fn line_flattens_to_half_unit_chords() {
        let cubic = Cubic3 {
            points: [
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(0.4, 0.0, 0.0),
                Point3::new(0.8, 0.0, 0.0),
                Point3::new(1.2, 0.0, 0.0),
            ],
        };
        let pts = flatten(&cubic, 0.5, 0.001);
        assert!(pts.windows(2).all(|w| (w[1].1 - w[0].1).length() <= 0.5));
    }

    #[test]
    fn circle_is_approximated_by_curved_segments() {
        let circle =
            Circle3D::new(Point3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0), 10.0).unwrap();
        let seam = circle.evaluate(0.0);
        let segments = approximate_edge(&EdgeCurve::Circle(circle), seam, seam, 0.005);
        assert!(segments.len() > 4);
        for segment in segments {
            for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
                let p = segment.evaluate(t);
                assert!(((p - Point3::new(0.0, 0.0, 0.0)).length() - 10.0).abs() < 0.005);
            }
        }
    }
}
