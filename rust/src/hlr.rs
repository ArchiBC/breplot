//! Orthographic line clipping against projected face triangles.
//! Inspired by Scenery's interval splitting: each occluder contributes a hidden
//! parameter interval, then merged intervals partition the source segment.

#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    /// Larger depth is closer to the viewer.
    pub depth: f64,
}

impl Point {
    pub fn lerp(self, other: Self, t: f64) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
            depth: self.depth + (other.depth - self.depth) * t,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Triangle {
    points: [Point; 3],
    bounds: [f64; 4],
    area: f64,
}

impl Triangle {
    pub fn new(points: [Point; 3]) -> Option<Self> {
        let area = cross(points[0], points[1], points[2]);
        let scale = points
            .iter()
            .fold(0.0_f64, |m, p| m.max(p.x.abs()).max(p.y.abs()));
        if area.abs() <= 1e-14 * scale.max(1.0).powi(2) {
            return None;
        }
        let bounds = [
            points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min),
            points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min),
            points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max),
            points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max),
        ];
        Some(Self {
            points,
            bounds,
            area,
        })
    }

    fn depth_at(&self, x: f64, y: f64) -> f64 {
        let [a, b, c] = self.points;
        let w0 = ((b.x - x) * (c.y - y) - (b.y - y) * (c.x - x)) / self.area;
        let w1 = ((c.x - x) * (a.y - y) - (c.y - y) * (a.x - x)) / self.area;
        let w2 = 1.0 - w0 - w1;
        w0 * a.depth + w1 * b.depth + w2 * c.depth
    }

    fn hidden_interval(&self, a: Point, b: Point, depth_eps: f64) -> Option<(f64, f64)> {
        if !overlaps(self.bounds, segment_bounds(a, b)) {
            return None;
        }
        let mut lo = 0.0_f64;
        let mut hi = 1.0_f64;
        let sign = self.area.signum();
        for i in 0..3 {
            let p = self.points[i];
            let q = self.points[(i + 1) % 3];
            let edge_x = q.x - p.x;
            let edge_y = q.y - p.y;
            let value = sign * (edge_x * (a.y - p.y) - edge_y * (a.x - p.x));
            let slope = sign * (edge_x * (b.y - a.y) - edge_y * (b.x - a.x));
            // The projected triangle is convex. Clip the segment parameter by
            // each half-plane, retaining where value + slope*t >= 0.
            if slope.abs() < 1e-14 {
                if value < -1e-12 {
                    return None;
                }
            } else {
                let crossing = -value / slope;
                if slope > 0.0 {
                    lo = lo.max(crossing);
                } else {
                    hi = hi.min(crossing);
                }
                if hi <= lo {
                    return None;
                }
            }
        }
        lo = lo.max(0.0);
        hi = hi.min(1.0);
        if hi - lo <= 1e-12 {
            return None;
        }
        // Both the source segment depth and the triangle depth vary linearly
        // under orthographic projection. Their difference can change sign once.
        let delta = |t: f64| {
            let p = a.lerp(b, t);
            self.depth_at(p.x, p.y) - p.depth - depth_eps
        };
        let d0 = delta(lo);
        let d1 = delta(hi);
        if d0 <= 0.0 && d1 <= 0.0 {
            return None;
        }
        if d0 <= 0.0 || d1 <= 0.0 {
            let crossing = lo + (hi - lo) * (-d0) / (d1 - d0);
            if d0 > 0.0 {
                hi = crossing;
            } else {
                lo = crossing;
            }
        }
        (hi - lo > 1e-12).then_some((lo, hi))
    }
}

fn cross(a: Point, b: Point, c: Point) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

fn segment_bounds(a: Point, b: Point) -> [f64; 4] {
    [a.x.min(b.x), a.y.min(b.y), a.x.max(b.x), a.y.max(b.y)]
}

fn overlaps(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] <= b[2] && b[0] <= a[2] && a[1] <= b[3] && b[1] <= a[3]
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fragment {
    pub a: Point,
    pub b: Point,
    pub hidden: bool,
}

impl PartialEq for Point {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y && self.depth == other.depth
    }
}

pub struct Scene {
    triangles: Vec<Triangle>,
    cells: Vec<Vec<usize>>,
    bounds: [f64; 4],
    grid_size: usize,
}

impl Scene {
    pub fn new(triangles: Vec<Triangle>) -> Self {
        let grid_size = 48;
        let mut bounds = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for tri in &triangles {
            bounds[0] = bounds[0].min(tri.bounds[0]);
            bounds[1] = bounds[1].min(tri.bounds[1]);
            bounds[2] = bounds[2].max(tri.bounds[2]);
            bounds[3] = bounds[3].max(tri.bounds[3]);
        }
        if triangles.is_empty() {
            bounds = [0.0, 0.0, 1.0, 1.0];
        }
        let mut scene = Self {
            triangles,
            cells: vec![Vec::new(); grid_size * grid_size],
            bounds,
            grid_size,
        };
        for i in 0..scene.triangles.len() {
            let [x0, y0, x1, y1] = scene.triangles[i].bounds;
            let (cx0, cy0) = scene.cell(x0, y0);
            let (cx1, cy1) = scene.cell(x1, y1);
            for cy in cy0..=cy1 {
                for cx in cx0..=cx1 {
                    scene.cells[cy * grid_size + cx].push(i);
                }
            }
        }
        scene
    }

    fn cell(&self, x: f64, y: f64) -> (usize, usize) {
        let map = |v: f64, lo: f64, hi: f64| {
            (((v - lo) / (hi - lo).max(1e-12) * self.grid_size as f64).floor() as isize)
                .clamp(0, self.grid_size as isize - 1) as usize
        };
        (
            map(x, self.bounds[0], self.bounds[2]),
            map(y, self.bounds[1], self.bounds[3]),
        )
    }

    pub fn split(&self, a: Point, b: Point, depth_eps: f64) -> Vec<Fragment> {
        if (a.x - b.x).abs() + (a.y - b.y).abs() <= 1e-12 {
            return Vec::new();
        }
        let [x0, y0, x1, y1] = segment_bounds(a, b);
        let (cx0, cy0) = self.cell(x0, y0);
        let (cx1, cy1) = self.cell(x1, y1);
        let mut candidates: Vec<usize> = Vec::new();
        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                candidates.extend(self.cells[cy * self.grid_size + cx].iter().copied());
            }
        }
        candidates.sort_unstable();
        candidates.dedup();
        let mut hidden = Vec::new();
        for id in candidates {
            if let Some(interval) = self.triangles[id].hidden_interval(a, b, depth_eps) {
                hidden.push(interval);
            }
        }
        hidden.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut merged: Vec<(f64, f64)> = Vec::new();
        for (lo, hi) in hidden {
            if let Some(last) = merged.last_mut()
                && lo <= last.1 + 1e-12
            {
                last.1 = last.1.max(hi);
                continue;
            }
            merged.push((lo, hi));
        }
        let mut result = Vec::new();
        let mut cursor = 0.0;
        for (lo, hi) in merged {
            if lo > cursor + 1e-12 {
                result.push(Fragment {
                    a: a.lerp(b, cursor),
                    b: a.lerp(b, lo),
                    hidden: false,
                });
            }
            if hi > lo + 1e-12 {
                result.push(Fragment {
                    a: a.lerp(b, lo),
                    b: a.lerp(b, hi),
                    hidden: true,
                });
            }
            cursor = hi;
        }
        if cursor < 1.0 - 1e-12 {
            result.push(Fragment {
                a: a.lerp(b, cursor),
                b,
                hidden: false,
            });
        }
        result
    }

    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64, depth: f64) -> Point {
        Point { x, y, depth }
    }

    #[test]
    fn triangle_clips_only_middle_of_rear_line() {
        let tri = Triangle::new([p(-1.0, -1.0, 1.0), p(1.0, -1.0, 1.0), p(0.0, 1.0, 1.0)]).unwrap();
        let parts = Scene::new(vec![tri]).split(p(-2.0, 0.0, 0.0), p(2.0, 0.0, 0.0), 1e-6);
        assert_eq!(parts.len(), 3);
        assert_eq!(
            parts.iter().map(|f| f.hidden).collect::<Vec<_>>(),
            vec![false, true, false]
        );
        assert!((parts[1].a.x + 0.5).abs() < 1e-9);
        assert!((parts[1].b.x - 0.5).abs() < 1e-9);
    }

    #[test]
    fn rear_triangle_does_not_hide_front_line() {
        let tri =
            Triangle::new([p(-1.0, -1.0, -1.0), p(1.0, -1.0, -1.0), p(0.0, 1.0, -1.0)]).unwrap();
        let parts = Scene::new(vec![tri]).split(p(-2.0, 0.0, 0.0), p(2.0, 0.0, 0.0), 1e-6);
        assert_eq!(parts.len(), 1);
        assert!(!parts[0].hidden);
    }
}
