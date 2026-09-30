// Conversion of exact archive geometry to display polylines is separate from decoding.
use super::{Curve, Model, Result, error};
use crate::evaluate::{norm, sub};
use crate::{Brep, BrepEdge, BrepFace, NurbsSurface, TrimRegion};
use std::collections::BTreeSet;
fn mix(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    std::array::from_fn(|i| a[i] * (1. - t) + b[i] * t)
}
fn sample(
    c: &Curve,
    domain: [f64; 2],
    reverse: bool,
    surface: Option<&NurbsSurface>,
    tol: f64,
) -> Result<Vec<[f64; 3]>> {
    c.check_display_continuity()?;
    let mut knots = vec![domain[0]];
    knots.extend(
        c.breaks()
            .into_iter()
            .filter(|&t| t > domain[0] && t < domain[1]),
    );
    knots.push(domain[1]);
    let at = |t: f64| -> Result<[f64; 3]> {
        let mut p = c.evaluate(t)?;
        if let Some(s) = surface {
            let d = s.domain();
            for i in 0..2 {
                let eps = 1e-10
                    * (d[i][1] - d[i][0])
                        .max(d[i][0].abs())
                        .max(d[i][1].abs())
                        .max(1.);
                if p[i] < d[i][0] - eps || p[i] > d[i][1] + eps {
                    return Err(error(&format!("trim UV {:?} outside surface {:?}", p, d)));
                }
                p[i] = p[i].clamp(d[i][0], d[i][1]);
            }
        }
        Ok(p)
    };
    let world = |p: [f64; 3]| -> Result<[f64; 3]> {
        surface.map_or(Ok(p), |s| {
            let d = s.domain();
            Ok(
                s.evaluate(p[0].clamp(d[0][0], d[0][1]), p[1].clamp(d[1][0], d[1][1]))?
                    .point,
            )
        })
    };
    let metric = |p: [f64; 3], q: [f64; 3]| -> f64 {
        surface.map_or(0., |s| {
            let d = s.domain();
            ((p[0] - q[0]) / (d[0][1] - d[0][0])).hypot((p[1] - q[1]) / (d[1][1] - d[1][0]))
        })
    };
    let mut out = vec![at(domain[0])?];
    for ab in knots.windows(2) {
        let a = ab[0];
        let b = ab[1];
        let mut stack = vec![(a, b, at(a)?, at(b)?, 0)];
        while let Some((a, b, pa, pb, depth)) = stack.pop() {
            let mut split = false;
            for f in [0.25, 0.5, 0.75] {
                let p = at(a + (b - a) * f)?;
                let q = mix(pa, pb, f);
                split |= norm(sub(world(p)?, world(q)?)) > tol || metric(p, q) > 0.002;
            }
            if split {
                if depth >= 18 {
                    return Err(error("curve sampling depth exceeded"));
                }
                let m = (a + b) / 2.;
                let pm = at(m)?;
                stack.push((m, b, pm, pb, depth + 1));
                stack.push((a, m, pa, pm, depth + 1));
            } else {
                out.push(pb);
                if out.len() > 65536 {
                    return Err(error("curve sample budget"));
                }
            }
        }
    }
    if reverse {
        out.reverse();
    }
    Ok(out)
}
impl Model {
    /// Chord tests at quarter/mid points are a display criterion, not a global error proof.
    /// Unsupported objects remain visible in `skipped`; no supported face is silently dropped.
    pub fn to_brep(&self, tolerance: f64) -> Result<Brep> {
        if !tolerance.is_finite() || tolerance <= 0. {
            return Err(error("tolerance must be positive"));
        }
        let mut out = Brep::default();
        let mut points = 0usize;
        for o in &self.objects {
            let b = &o.brep;
            b.validate()?;
            let offset = out.edges.len();
            for e in &b.edges {
                let samples = sample(
                    b.curves3[e.curve as usize].as_ref().unwrap(),
                    e.proxy_domain,
                    e.reversed,
                    None,
                    tolerance,
                )?;
                points += samples.len();
                if points > 500_000 {
                    return Err(error("combined edge sample budget"));
                }
                out.edges.push(BrepEdge { points: samples });
            }
            for (fi, f) in b.faces.iter().enumerate() {
                let surface = b.surfaces[f.surface as usize].as_ref().unwrap().clone();
                let mut outer = None;
                let mut holes = vec![];
                let mut edges = BTreeSet::new();
                for &li in &f.loops {
                    let l = &b.loops[li as usize];
                    if !matches!(l.kind, 1 | 2) {
                        return Err(error("unsupported loop kind"));
                    }
                    let mut ring: Vec<[f64; 3]> = vec![];
                    for &ti in &l.trims {
                        let t = &b.trims[ti as usize];
                        if t.edge >= 0 {
                            edges.insert(offset + t.edge as usize);
                        }
                        let samples = sample(
                            b.curves2[t.curve as usize].as_ref().unwrap(),
                            t.proxy_domain,
                            t.reversed,
                            Some(&surface),
                            tolerance,
                        )
                        .map_err(|e| e.prefix(&format!("object{}.face{fi}.trim{ti}", o.record)))?;
                        if let Some(&last) = ring.last() {
                            if norm(sub(last, samples[0])) > 1e-7 {
                                return Err(error("disconnected trim chain"));
                            }
                        }
                        let skip = usize::from(!ring.is_empty());
                        ring.extend(samples.into_iter().skip(skip));
                    }
                    if ring.len() < 4 || norm(sub(ring[0], *ring.last().unwrap())) > 1e-7 {
                        return Err(error("open trim loop"));
                    }
                    ring.pop();
                    let ring = ring.into_iter().map(|p| [p[0], p[1]]).collect();
                    if l.kind == 1 {
                        if outer.replace(ring).is_some() {
                            return Err(error("multiple outer loops unsupported"));
                        }
                    } else {
                        holes.push(ring);
                    }
                }
                let trim =
                    TrimRegion::new(outer.ok_or_else(|| error("missing outer loop"))?, holes)
                        .map_err(|e| e.prefix(&format!("object{}.face{fi}", o.record)))?;
                out.faces.push(BrepFace {
                    surface,
                    trim: Some(trim),
                    reversed: f.reversed,
                    boundary_edges: edges.into_iter().collect(),
                });
                if out.faces.len() > 1024 {
                    return Err(error("combined face budget"));
                }
            }
        }
        Ok(out)
    }
}
