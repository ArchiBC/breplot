//! Polygonal UV trimming. Ear clipping + convex clipping/subtraction, no CAD kernel.
use crate::evaluate::{cross, norm, sub};
use crate::{DataError, Mesh, MeshOptions, MeshVertex, NurbsSurface};
pub(crate) type P = [f64; 2];
const EPS: f64 = 1e-12;
pub(crate) fn orient(a: P, b: P, c: P) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
fn mix(a: P, b: P, t: f64) -> P {
    std::array::from_fn(|i| a[i] + t * (b[i] - a[i]))
}
fn area(p: &[P]) -> f64 {
    (1..p.len().saturating_sub(1))
        .map(|i| orient(p[0], p[i], p[i + 1]))
        .sum::<f64>()
        / 2.
}
fn on(a: P, b: P, p: P) -> bool {
    orient(a, b, p).abs() <= EPS
        && (0..2).all(|i| p[i] >= a[i].min(b[i]) - EPS && p[i] <= a[i].max(b[i]) + EPS)
}
fn intersects(a: P, b: P, c: P, d: P) -> bool {
    on(a, b, c)
        || on(a, b, d)
        || on(c, d, a)
        || on(c, d, b)
        || (orient(a, b, c) * orient(a, b, d) < 0. && orient(c, d, a) * orient(c, d, b) < 0.)
}
fn location(ring: &[P], p: P) -> i8 {
    let mut inside = false;
    for i in 0..ring.len() {
        let a = ring[i];
        let b = ring[(i + 1) % ring.len()];
        if on(a, b, p) {
            return 0;
        }
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
    }
    if inside { 1 } else { -1 }
}
fn validate_ring(ring: &mut Vec<P>) -> Result<(), DataError> {
    if ring.iter().flatten().any(|x| !x.is_finite()) {
        return Err(DataError::new("trim", "nonfinite normalized ring"));
    }
    if ring.len() > 1 && ring.first() == ring.last() {
        ring.pop();
    }
    if ring.len() < 3 || ring.len() > 256 {
        return Err(DataError::new("trim", "each ring requires 3..256 vertices"));
    }
    for i in 0..ring.len() {
        let a = ring[i];
        let b = ring[(i + 1) % ring.len()];
        if (a[0] - b[0]).hypot(a[1] - b[1]) <= EPS {
            return Err(DataError::new("trim", "zero-length ring edge"));
        }
        for j in i + 1..ring.len() {
            if j == i + 1 || (i == 0 && j == ring.len() - 1) {
                continue;
            }
            if intersects(a, b, ring[j], ring[(j + 1) % ring.len()]) {
                return Err(DataError::new("trim", "self-intersecting or touching ring"));
            }
        }
    }
    if area(ring).abs() <= EPS {
        return Err(DataError::new("trim", "zero-area ring"));
    }
    if area(ring) < 0. {
        ring.reverse();
    }
    Ok(())
}
fn triangulate(ring: &[P]) -> Result<Vec<[P; 3]>, DataError> {
    let mut p = ring.to_vec();
    let mut result = Vec::new();
    while p.len() > 3 {
        let ear = (0..p.len()).find(|&i| {
            let a = p[(i + p.len() - 1) % p.len()];
            let b = p[i];
            let c = p[(i + 1) % p.len()];
            orient(a, b, c) > EPS
                && !(0..p.len())
                    .filter(|&j| {
                        j != i && j != (i + p.len() - 1) % p.len() && j != (i + 1) % p.len()
                    })
                    .any(|j| {
                        orient(a, b, p[j]) >= -EPS
                            && orient(b, c, p[j]) >= -EPS
                            && orient(c, a, p[j]) >= -EPS
                    })
        });
        if let Some(i) = ear {
            result.push([p[(i + p.len() - 1) % p.len()], p[i], p[(i + 1) % p.len()]]);
            p.remove(i);
        } else {
            // Remove redundant collinear vertices without changing the boundary.
            if let Some(i) = (0..p.len())
                .find(|&i| on(p[(i + p.len() - 1) % p.len()], p[(i + 1) % p.len()], p[i]))
            {
                p.remove(i);
            } else {
                return Err(DataError::new(
                    "trim",
                    "ring triangulation failed at numeric tolerance",
                ));
            }
        }
    }
    if p.len() == 3 && area(&p) > EPS {
        result.push([p[0], p[1], p[2]]);
    }
    Ok(result)
}
fn halfplane(poly: &[P], a: P, b: P, inside: bool) -> Vec<P> {
    let mut out = Vec::new();
    if poly.is_empty() {
        return out;
    }
    let sign = if inside { 1. } else { -1. };
    for i in 0..poly.len() {
        let p = poly[i];
        let q = poly[(i + 1) % poly.len()];
        let dp = sign * orient(a, b, p);
        let dq = sign * orient(a, b, q);
        if dp >= 0. {
            out.push(p);
        }
        if (dp >= 0.) != (dq >= 0.) {
            out.push(mix(p, q, (dp / (dp - dq)).clamp(0., 1.)));
        }
    }
    out.dedup_by(|a, b| (a[0] - b[0]).hypot(a[1] - b[1]) < EPS);
    if out.len() > 1
        && (out[0][0] - out.last().unwrap()[0]).hypot(out[0][1] - out.last().unwrap()[1]) < EPS
    {
        out.pop();
    }
    out
}

// doc:trim:start
/// One simple outer polygon and disjoint simple holes, in original UV coordinates.
/// Loops close implicitly. Orientation is normalized; periodic seam wrapping is not inferred.
#[derive(Clone, Debug)]
pub struct TrimRegion {
    origin: P,
    scale: P,
    rings: Vec<Vec<P>>,
    outer_triangles: Vec<[P; 3]>,
    hole_triangles: Vec<[P; 3]>,
}
// doc:trim:end
impl TrimRegion {
    pub fn new(outer: Vec<P>, holes: Vec<Vec<P>>) -> Result<Self, DataError> {
        if outer.len() < 3
            || holes.len() > 32
            || outer
                .iter()
                .chain(holes.iter().flatten())
                .flatten()
                .any(|v| !v.is_finite())
        {
            return Err(DataError::new(
                "trim",
                "finite coordinates, outer ring and at most 32 holes required",
            ));
        }
        let origin =
            std::array::from_fn(|i| outer.iter().map(|p| p[i]).fold(f64::INFINITY, f64::min));
        let scale: P = std::array::from_fn(|i| {
            outer.iter().map(|p| p[i]).fold(f64::NEG_INFINITY, f64::max) - origin[i]
        });
        if scale.iter().any(|s| !s.is_finite() || *s <= 0.) {
            return Err(DataError::new("trim", "invalid UV extent"));
        }
        let mut rings: Vec<Vec<P>> = std::iter::once(outer)
            .chain(holes)
            .map(|r| {
                r.into_iter()
                    .map(|p| std::array::from_fn(|i| (p[i] - origin[i]) / scale[i]))
                    .collect()
            })
            .collect();
        for r in &mut rings {
            validate_ring(r)?;
        }
        for i in 1..rings.len() {
            if location(&rings[0], rings[i][0]) != 1 {
                return Err(DataError::new(
                    "trim",
                    "holes must lie strictly inside outer ring",
                ));
            }
            for j in 0..i {
                for a in 0..rings[i].len() {
                    for b in 0..rings[j].len() {
                        if intersects(
                            rings[i][a],
                            rings[i][(a + 1) % rings[i].len()],
                            rings[j][b],
                            rings[j][(b + 1) % rings[j].len()],
                        ) {
                            return Err(DataError::new("trim", "rings may not intersect or touch"));
                        }
                    }
                }
                if j > 0
                    && (location(&rings[j], rings[i][0]) >= 0
                        || location(&rings[i], rings[j][0]) >= 0)
                {
                    return Err(DataError::new(
                        "trim",
                        "nested or overlapping holes are unsupported",
                    ));
                }
            }
        }
        let outer_triangles = triangulate(&rings[0])?;
        let hole_triangles = rings[1..]
            .iter()
            .map(|r| triangulate(r))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect();
        Ok(Self {
            origin,
            scale,
            rings,
            outer_triangles,
            hole_triangles,
        })
    }
    fn normalized(&self, p: P) -> P {
        std::array::from_fn(|i| (p[i] - self.origin[i]) / self.scale[i])
    }
    fn original(&self, p: P) -> P {
        std::array::from_fn(|i| self.origin[i] + p[i] * self.scale[i])
    }
    pub fn contains(&self, uv: P) -> bool {
        if uv.iter().any(|v| !v.is_finite()) {
            return false;
        }
        let p = self.normalized(uv);
        location(&self.rings[0], p) >= 0 && self.rings[1..].iter().all(|r| location(r, p) <= 0)
    }
    pub fn rings(&self) -> Vec<Vec<P>> {
        self.rings
            .iter()
            .map(|r| r.iter().map(|&p| self.original(p)).collect())
            .collect()
    }
    pub(crate) fn check_domain(&self, s: &NurbsSurface) -> Result<(), DataError> {
        let d = s.domain();
        if self
            .rings()
            .iter()
            .flatten()
            .any(|p| (0..2).any(|i| p[i] < d[i][0] || p[i] > d[i][1]))
        {
            return Err(DataError::new(
                "trim",
                "ring lies outside surface domain; unwrap periodic seams explicitly",
            ));
        }
        Ok(())
    }
    /// Parameter intervals on a UV segment retained by the polygonal region.
    pub fn segment_intervals(&self, a: P, b: P) -> Vec<[f64; 2]> {
        let (p, q) = (self.normalized(a), self.normalized(b));
        let d = [q[0] - p[0], q[1] - p[1]];
        let mut cuts = vec![0., 1.];
        for r in &self.rings {
            for j in 0..r.len() {
                let c = r[j];
                let e = r[(j + 1) % r.len()];
                let v = [e[0] - c[0], e[1] - c[1]];
                let det = d[0] * v[1] - d[1] * v[0];
                if det.abs() > EPS {
                    let cp = [c[0] - p[0], c[1] - p[1]];
                    let t = (cp[0] * v[1] - cp[1] * v[0]) / det;
                    let u = (cp[0] * d[1] - cp[1] * d[0]) / det;
                    if (0.0..=1.0).contains(&t) && (-EPS..=1. + EPS).contains(&u) {
                        cuts.push(t);
                    }
                } else if on(p, q, c) || on(p, q, e) {
                    let axis = if d[0].abs() > d[1].abs() { 0 } else { 1 };
                    if d[axis] != 0. {
                        for x in [c, e] {
                            let t = (x[axis] - p[axis]) / d[axis];
                            if t > 0. && t < 1. {
                                cuts.push(t);
                            }
                        }
                    }
                }
            }
        }
        cuts.sort_by(f64::total_cmp);
        cuts.dedup_by(|a, b| (*a - *b).abs() < EPS);
        cuts.windows(2)
            .filter(|w| self.contains(mix(a, b, (w[0] + w[1]) / 2.)))
            .map(|w| [w[0], w[1]])
            .collect()
    }
    fn clip(&self, triangle: [P; 3]) -> Vec<Vec<P>> {
        let triangle = triangle.map(|p| self.normalized(p));
        let mut fragments = Vec::new();
        for outer in &self.outer_triangles {
            let mut p = triangle.to_vec();
            for i in 0..3 {
                p = halfplane(&p, outer[i], outer[(i + 1) % 3], true);
            }
            if area(&p) > EPS {
                fragments.push(p);
            }
        }
        for hole in &self.hole_triangles {
            let mut remaining = Vec::new();
            for mut p in fragments {
                for i in 0..3 {
                    let outside = halfplane(&p, hole[i], hole[(i + 1) % 3], false);
                    if area(&outside) > EPS {
                        remaining.push(outside);
                    }
                    p = halfplane(&p, hole[i], hole[(i + 1) % 3], true);
                    if p.len() < 3 {
                        break;
                    }
                }
            }
            fragments = remaining;
        }
        fragments
            .into_iter()
            .map(|r| r.into_iter().map(|p| self.original(p)).collect())
            .collect()
    }
}

impl NurbsSurface {
    pub fn mesh_trimmed(
        &self,
        region: &TrimRegion,
        subdivisions: usize,
        max_vertices: usize,
    ) -> Result<Mesh, DataError> {
        region.check_domain(self)?;
        let base = self.mesh_uniform(subdivisions, max_vertices)?;
        let mut result = Mesh {
            vertices: Vec::new(),
            triangles: Vec::new(),
            subdivisions,
            sampled_error: 0.,
            degenerate_triangles: 0,
        };
        for t in base.triangles {
            let spans = base.vertices[t[0]].spans;
            for polygon in region.clip(t.map(|i| base.vertices[i].uv)) {
                for j in 1..polygon.len() - 1 {
                    let uv = [polygon[0], polygon[j], polygon[j + 1]];
                    if orient(uv[0], uv[1], uv[2]) <= 0. {
                        continue;
                    }
                    if result.vertices.len() + 3 > max_vertices {
                        return Err(DataError::new(
                            "max_vertices",
                            "trimmed mesh budget exceeded",
                        ));
                    }
                    let start = result.vertices.len();
                    for p in uv {
                        let sample = self.evaluate_in_spans(p, spans)?;
                        result.vertices.push(MeshVertex {
                            uv: p,
                            point: sample.point,
                            normal: sample.normal,
                            spans,
                        });
                    }
                    let points: [_; 3] = std::array::from_fn(|i| result.vertices[start + i].point);
                    if norm(cross(sub(points[1], points[0]), sub(points[2], points[0]))) == 0. {
                        result.degenerate_triangles += 1;
                        continue;
                    }
                    result.triangles.push([start, start + 1, start + 2]);
                    for weights in [[1. / 3.; 3], [0.5, 0.5, 0.], [0., 0.5, 0.5], [0.5, 0., 0.5]] {
                        let p =
                            std::array::from_fn(|c| (0..3).map(|i| weights[i] * uv[i][c]).sum());
                        let q = std::array::from_fn(|c| {
                            (0..3).map(|i| weights[i] * points[i][c]).sum()
                        });
                        let error = norm(sub(self.evaluate_in_spans(p, spans)?.point, q));
                        if !error.is_finite() {
                            return Err(DataError::new("trim", "nonfinite approximation error"));
                        }
                        result.sampled_error = result.sampled_error.max(error);
                    }
                }
            }
        }
        Ok(result)
    }
    pub fn tessellate_trimmed(
        &self,
        region: &TrimRegion,
        options: MeshOptions,
    ) -> Result<Mesh, DataError> {
        if !options.tolerance.is_finite()
            || options.tolerance <= 0.
            || options.initial_subdivisions == 0
            || options.initial_subdivisions > options.max_subdivisions
        {
            return Err(DataError::new(
                "mesh_options",
                "invalid trim refinement options",
            ));
        }
        let mut n = options.initial_subdivisions;
        loop {
            let mesh = self.mesh_trimmed(region, n, options.max_vertices)?;
            if mesh.sampled_error <= options.tolerance {
                return Ok(mesh);
            }
            if n == options.max_subdivisions {
                return Err(DataError::new(
                    "max_subdivisions",
                    "trimmed sampled error exceeds tolerance",
                ));
            }
            n = n.saturating_mul(2).min(options.max_subdivisions);
        }
    }
}
