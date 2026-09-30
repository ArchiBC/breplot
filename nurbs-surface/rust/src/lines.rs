use crate::{DataError, NurbsSurface, TrimRegion};

// doc:lines:start
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    IsoU,
    IsoV,
    TrimBoundary,
    Boundary,
    Silhouette,
    ControlNet,
    ControlPoint,
}
#[derive(Clone, Debug)]
pub struct SurfaceLine {
    pub kind: LineKind,
    pub uv: Vec<[f64; 2]>,
    pub points: Vec<[f64; 3]>,
}
// doc:lines:end
fn mix(a: [f64; 2], b: [f64; 2], t: f64) -> [f64; 2] {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}
impl NurbsSurface {
    /// Explicit U/V isovalues; lines are independent of mesh edges. If supplied,
    /// the polygon region clips isocurves and its boundary is included separately.
    pub fn structure_lines(
        &self,
        region: Option<&TrimRegion>,
        u_values: &[f64],
        v_values: &[f64],
        samples_per_span: usize,
    ) -> Result<Vec<SurfaceLine>, DataError> {
        if samples_per_span == 0 || samples_per_span > 1024 || u_values.len() + v_values.len() > 256
        {
            return Err(DataError::new(
                "structure_lines",
                "requires 1..1024 samples per span and at most 256 isovalues",
            ));
        }
        if let Some(r) = region {
            r.check_domain(self)?;
        }
        let d = self.domain();
        let mut segments = Vec::new();
        for &u in u_values {
            self.u().span_at(u)?;
            segments.push((LineKind::IsoU, [u, d[1][0]], [u, d[1][1]]));
        }
        for &v in v_values {
            self.v().span_at(v)?;
            segments.push((LineKind::IsoV, [d[0][0], v], [d[0][1], v]));
        }
        if let Some(r) = region {
            for ring in r.rings() {
                for i in 0..ring.len() {
                    segments.push((LineKind::TrimBoundary, ring[i], ring[(i + 1) % ring.len()]));
                }
            }
        }
        let mut result = Vec::new();
        let mut total = 0;
        for (kind, a, b) in segments {
            let intervals = if kind == LineKind::TrimBoundary {
                vec![[0., 1.]]
            } else {
                region.map_or_else(|| vec![[0., 1.]], |r| r.segment_intervals(a, b))
            };
            for [start, end] in intervals {
                let mut cuts = vec![start, end];
                for (axis, knots) in [self.u().knots(), self.v().knots()].iter().enumerate() {
                    if a[axis] != b[axis] {
                        for &k in *knots {
                            let t = (k - a[axis]) / (b[axis] - a[axis]);
                            if t > start && t < end {
                                cuts.push(t);
                            }
                        }
                    }
                }
                cuts.sort_by(f64::total_cmp);
                cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
                for interval in cuts.windows(2) {
                    let mid = mix(a, b, (interval[0] + interval[1]) / 2.);
                    let spans = [self.u().span_at(mid[0])?, self.v().span_at(mid[1])?];
                    let mut line = SurfaceLine {
                        kind,
                        uv: Vec::new(),
                        points: Vec::new(),
                    };
                    for i in 0..=samples_per_span {
                        total += 1;
                        if total > 200_000 {
                            return Err(DataError::new("structure_lines", "point budget exceeded"));
                        }
                        let uv = mix(
                            a,
                            b,
                            interval[0]
                                + (interval[1] - interval[0]) * i as f64 / samples_per_span as f64,
                        );
                        let sample = self.evaluate_in_spans(uv, spans)?;
                        line.uv.push(uv);
                        line.points.push(sample.point);
                    }
                    result.push(line);
                }
            }
        }
        Ok(result)
    }
}

/// Occlusion interval of a projected 3D chord by a projected triangle.
/// Larger depth is closer to the viewer. All projections use the same frame.
pub(crate) fn hidden_interval(
    a: [f64; 3],
    b: [f64; 3],
    mut t: [[f64; 3]; 3],
    bias: f64,
) -> Option<[f64; 2]> {
    for axis in 0..2 {
        let lo = t.iter().map(|p| p[axis]).fold(f64::INFINITY, f64::min);
        let hi = t.iter().map(|p| p[axis]).fold(f64::NEG_INFINITY, f64::max);
        if a[axis].max(b[axis]) < lo || a[axis].min(b[axis]) > hi {
            return None;
        }
    }
    let xy = |p: [f64; 3]| [p[0], p[1]];
    let orient = crate::trim::orient;
    let mut area = orient(xy(t[0]), xy(t[1]), xy(t[2]));
    if area.abs() < 1e-14 {
        return None;
    }
    if area < 0. {
        t.swap(1, 2);
        area = -area;
    }
    let mut lo = 0.0_f64;
    let mut hi = 1.0_f64;
    for i in 0..3 {
        let x = orient(xy(t[i]), xy(t[(i + 1) % 3]), xy(a));
        let y = orient(xy(t[i]), xy(t[(i + 1) % 3]), xy(b));
        if x < 0. && y < 0. {
            return None;
        }
        if (x < 0.) != (y < 0.) {
            let hit = x / (x - y);
            if x < 0. {
                lo = lo.max(hit);
            } else {
                hi = hi.min(hit);
            }
        }
    }
    if lo >= hi {
        return None;
    }
    let delta = |s: f64| {
        let p: [f64; 3] = std::array::from_fn(|i| a[i] + s * (b[i] - a[i]));
        let w0 = orient(xy(t[1]), xy(t[2]), xy(p)) / area;
        let w1 = orient(xy(t[2]), xy(t[0]), xy(p)) / area;
        w0 * t[0][2] + w1 * t[1][2] + (1. - w0 - w1) * t[2][2] - p[2] - bias
    };
    let x = delta(lo);
    let y = delta(hi);
    if x <= 0. && y <= 0. {
        return None;
    }
    if (x <= 0.) != (y <= 0.) {
        let hit = lo + (hi - lo) * x / (x - y);
        if x <= 0. {
            lo = hit;
        } else {
            hi = hit;
        }
    }
    (lo < hi).then_some([lo, hi])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn triangle_hides_only_rear_middle_interval() {
        let t = [[0., -1., 1.], [2., -1., 1.], [1., 1., 1.]];
        let interval = hidden_interval([-1., 0., 0.], [3., 0., 0.], t, 0.).unwrap();
        assert!((interval[0] - 0.375).abs() < 1e-12);
        assert!((interval[1] - 0.625).abs() < 1e-12);
        assert!(hidden_interval([-1., 0., 2.], [3., 0., 2.], t, 0.).is_none());
        assert!(hidden_interval([-1., 3., 0.], [3., 3., 0.], t, 0.).is_none());
    }

    #[test]
    fn depth_crossing_and_coplanar_lines() {
        let t = [[0., 0., 0.], [2., 0., 0.], [0., 2., 0.]];
        let h = hidden_interval([0.1, 0.1, -1.], [0.9, 0.1, 1.], t, 0.).unwrap();
        assert!((h[0]).abs() < 1e-12 && (h[1] - 0.5).abs() < 1e-12);
        assert!(hidden_interval([0.1, 0.1, 0.], [0.9, 0.1, 0.], t, 1e-8).is_none());
    }
}
