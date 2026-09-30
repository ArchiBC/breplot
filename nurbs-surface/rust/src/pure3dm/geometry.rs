use super::{
    Result,
    archive::{ANON, Reader, class},
    error,
};
use crate::{KnotAxis, KnotFormat, NurbsSurface};
#[derive(Clone, Debug)]
pub struct NurbsCurve {
    pub axis: KnotAxis,
    pub dimension: usize,
    pub points: Vec<[f64; 3]>,
    pub weights: Vec<f64>,
}
#[derive(Clone, Debug)]
pub enum Curve {
    Nurbs(NurbsCurve),
    Poly {
        parameters: Vec<f64>,
        segments: Vec<Curve>,
    },
}
impl Curve {
    pub(crate) fn check_display_continuity(&self) -> Result<()> {
        match self {
            Self::Nurbs(c) => {
                let d = c.axis.domain();
                let mut run = 0;
                let mut previous = f64::NAN;
                for &k in c.axis.knots() {
                    run = if k == previous { run + 1 } else { 1 };
                    previous = k;
                    if k > d[0] && k < d[1] && run > c.axis.degree() {
                        return Err(error(
                            "fully repeated internal curve knot requires separate display pieces",
                        ));
                    }
                }
            }
            Self::Poly { segments, .. } => {
                for c in segments {
                    c.check_display_continuity()?;
                }
                for pair in segments.windows(2) {
                    let a = pair[0].evaluate(pair[0].domain()[1])?;
                    let b = pair[1].evaluate(pair[1].domain()[0])?;
                    if crate::evaluate::norm(crate::evaluate::sub(a, b)) > 1e-9 {
                        return Err(error("disconnected polycurve segments"));
                    }
                }
            }
        }
        Ok(())
    }
    pub fn dimension(&self) -> usize {
        match self {
            Self::Nurbs(c) => c.dimension,
            Self::Poly { segments, .. } => segments[0].dimension(),
        }
    }
    pub fn domain(&self) -> [f64; 2] {
        match self {
            Self::Nurbs(c) => c.axis.domain(),
            Self::Poly { parameters, .. } => [parameters[0], *parameters.last().unwrap()],
        }
    }
    pub fn breaks(&self) -> Vec<f64> {
        match self {
            Self::Nurbs(c) => {
                let d = c.axis.domain();
                let mut k: Vec<_> = c
                    .axis
                    .knots()
                    .iter()
                    .copied()
                    .filter(|&x| x >= d[0] && x <= d[1])
                    .collect();
                k.dedup();
                k
            }
            Self::Poly {
                parameters,
                segments,
            } => {
                let mut out = vec![];
                for (i, c) in segments.iter().enumerate() {
                    let d = c.domain();
                    out.extend(c.breaks().iter().map(|t| {
                        parameters[i]
                            + (parameters[i + 1] - parameters[i]) * (t - d[0]) / (d[1] - d[0])
                    }));
                }
                out.dedup();
                out
            }
        }
    }
    pub fn evaluate(&self, t: f64) -> Result<[f64; 3]> {
        match self {
            Self::Nurbs(c) => c.evaluate(t),
            Self::Poly {
                parameters,
                segments,
            } => {
                let d = self.domain();
                if !t.is_finite() || t < d[0] || t > d[1] {
                    return Err(error("polycurve parameter outside domain"));
                }
                let i = parameters
                    .partition_point(|&x| x <= t)
                    .saturating_sub(1)
                    .min(segments.len() - 1);
                let c = &segments[i];
                let cd = c.domain();
                let u = cd[0]
                    + (cd[1] - cd[0]) * (t - parameters[i]) / (parameters[i + 1] - parameters[i]);
                c.evaluate(u.clamp(cd[0], cd[1]))
            }
        }
    }
}
impl NurbsCurve {
    pub fn evaluate(&self, t: f64) -> Result<[f64; 3]> {
        let span = self.axis.span_at(t)?;
        let b = self.axis.basis(span, t);
        let first = span - self.axis.degree();
        let scale = self.weights[first..=span]
            .iter()
            .copied()
            .fold(0., f64::max);
        let mut p = [0.; 3];
        let mut w = 0.;
        for (j, v) in b.iter().enumerate() {
            let k = first + j;
            let a = v[0] * self.weights[k] / scale;
            w += a;
            for d in 0..3 {
                p[d] += a * self.points[k][d];
            }
        }
        if w <= 0. || !w.is_finite() {
            return Err(error("invalid rational curve denominator"));
        }
        Ok(p.map(|x| x / w))
    }
}
fn axis(r: &mut Reader, n: usize, order: usize) -> Result<KnotAxis> {
    let count = r.count(1_000_000)?;
    if count != n + order - 2 {
        return Err(error("knot count mismatch"));
    }
    let knots = (0..count).map(|_| r.f64()).collect::<Result<Vec<_>>>()?;
    KnotAxis::new(knots, n, KnotFormat::Rhino)
}
fn pole(r: &mut Reader, dim: usize, rat: bool) -> Result<([f64; 3], f64)> {
    let mut p = [0.; 3];
    for x in &mut p[..dim] {
        *x = r.f64()?;
    }
    let w = if rat { r.f64()? } else { 1. };
    if w <= 0. {
        return Err(error("non-positive weight"));
    }
    p = p.map(|x| x / w);
    if p.iter().any(|x| !x.is_finite()) {
        return Err(error("invalid control point"));
    }
    Ok((p, w))
}
fn header(r: &mut Reader) -> Result<(usize, bool)> {
    let d = r.i32()?;
    let rat = r.i32()?;
    if !matches!(d, 2 | 3) || !matches!(rat, 0 | 1) {
        return Err(error("invalid dimension or rational flag"));
    }
    Ok((d as usize, rat == 1))
}
fn order(r: &mut Reader) -> Result<usize> {
    let n = r.i32()?;
    if !(2..=33).contains(&n) {
        return Err(error("unsupported order"));
    }
    Ok(n as usize)
}
fn curve(mut r: Reader) -> Result<Curve> {
    let minor = r.version(1, 1)?;
    let (dimension, rat) = header(&mut r)?;
    let order = order(&mut r)?;
    let n = r.count(1_000_000)?;
    r.take(8 + 48)?;
    let axis = axis(&mut r, n, order)?;
    if r.count(1_000_000)? != n {
        return Err(error("curve CV count mismatch"));
    }
    let mut points = vec![];
    let mut weights = vec![];
    for _ in 0..n {
        let (p, w) = pole(&mut r, dimension, rat)?;
        points.push(p);
        weights.push(w);
    }
    if minor >= 1 {
        r.u8()?;
    }
    r.finish()?;
    Ok(Curve::Nurbs(NurbsCurve {
        axis,
        dimension,
        points,
        weights,
    }))
}
fn line(mut r: Reader) -> Result<Curve> {
    r.version(1, 0)?;
    let a = r.point::<3>()?;
    let b = r.point::<3>()?;
    let d = r.point::<2>()?;
    let dimension = r.i32()?;
    if !matches!(dimension, 2 | 3) {
        return Err(error("invalid line dimension"));
    }
    r.finish()?;
    Ok(Curve::Nurbs(NurbsCurve {
        axis: KnotAxis::new(vec![d[0], d[0], d[1], d[1]], 2, KnotFormat::Full)?,
        dimension: dimension as usize,
        points: vec![a, b],
        weights: vec![1., 1.],
    }))
}
fn read_curve(
    r: &mut Reader,
    unsupported: &mut Vec<String>,
    depth: usize,
) -> Result<Option<Curve>> {
    if depth > 16 {
        return Err(error("polycurve nesting budget"));
    }
    let (id, mut data) = class(r)?;
    match id.as_str() {
        "4ED7D4DD-E947-11D3-BFE5-0010830122F0" | "5EAF1119-0B51-11D4-BFFE-0010830122F0" => {
            Ok(Some(curve(data)?))
        }
        "4ED7D4DB-E947-11D3-BFE5-0010830122F0" => Ok(Some(line(data)?)),
        "4ED7D4E0-E947-11D3-BFE5-0010830122F0" => {
            data.version(1, 0)?;
            let n = data.count(100_000)?;
            if n == 0 {
                return Err(error("empty polycurve"));
            }
            data.take(8 + 48)?;
            if data.count(100_001)? != n + 1 {
                return Err(error("polycurve parameters count"));
            }
            let parameters = (0..=n).map(|_| data.f64()).collect::<Result<Vec<_>>>()?;
            if parameters.windows(2).any(|p| p[0] >= p[1]) {
                return Err(error("polycurve parameters not increasing"));
            }
            let mut segments = vec![];
            for _ in 0..n {
                if let Some(c) = read_curve(&mut data, unsupported, depth + 1)? {
                    segments.push(c);
                }
            }
            data.finish()?;
            if segments.len() != n {
                return Ok(None);
            }
            if segments
                .iter()
                .any(|c| c.dimension() != segments[0].dimension())
            {
                return Err(error("mixed polycurve dimensions"));
            }
            Ok(Some(Curve::Poly {
                parameters,
                segments,
            }))
        }
        _ => {
            unsupported.push(format!("curve class {id}"));
            Ok(None)
        }
    }
}
fn surface(mut r: Reader) -> Result<NurbsSurface> {
    r.version(1, 0)?;
    let (dim, rat) = header(&mut r)?;
    if dim != 3 {
        return Err(error("surface dimension must be 3"));
    }
    let ou = order(&mut r)?;
    let ov = order(&mut r)?;
    let nu = r.count(1_000_000)?;
    let nv = r.count(1_000_000)?;
    if nu.checked_mul(nv).is_none_or(|n| n > 1_000_000) {
        return Err(error("surface CV budget"));
    }
    r.take(8 + 48)?;
    let u = axis(&mut r, nu, ou)?;
    let v = axis(&mut r, nv, ov)?;
    if r.count(1_000_000)? != nu * nv {
        return Err(error("surface CV count mismatch"));
    }
    let mut points = vec![];
    let mut weights = vec![];
    for _ in 0..nu {
        let mut row = vec![];
        let mut wr = vec![];
        for _ in 0..nv {
            let (p, w) = pole(&mut r, dim, rat)?;
            row.push(p);
            wr.push(w);
        }
        points.push(row);
        weights.push(wr);
    }
    r.finish()?;
    NurbsSurface::new(u, v, points, Some(weights))
}
pub fn curves(r: &mut Reader, unsupported: &mut Vec<String>) -> Result<Vec<Option<Curve>>> {
    let mut r = r.expect(ANON)?;
    r.version(1, 0)?;
    let n = r.count(100_000)?;
    let mut out = vec![];
    for _ in 0..n {
        let flag = r.i32()?;
        if flag == 0 {
            out.push(None);
            continue;
        }
        if flag != 1 {
            return Err(error("invalid presence flag"));
        }
        out.push(read_curve(&mut r, unsupported, 0)?);
    }
    r.finish()?;
    Ok(out)
}
pub fn surfaces(
    r: &mut Reader,
    unsupported: &mut Vec<String>,
) -> Result<Vec<Option<NurbsSurface>>> {
    let mut r = r.expect(ANON)?;
    r.version(1, 0)?;
    let n = r.count(1024)?;
    let mut out = vec![];
    for _ in 0..n {
        let flag = r.i32()?;
        if flag == 0 {
            out.push(None);
            continue;
        }
        if flag != 1 {
            return Err(error("invalid presence flag"));
        }
        let (id, data) = class(&mut r)?;
        out.push(
            if matches!(
                id.as_str(),
                "4ED7D4DE-E947-11D3-BFE5-0010830122F0" | "4760C817-0BE3-11D4-BFFE-0010830122F0"
            ) {
                Some(surface(data)?)
            } else {
                unsupported.push(format!("surface class {id}"));
                None
            },
        );
    }
    r.finish()?;
    Ok(out)
}
