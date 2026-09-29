use crate::{DataError, KnotAxis, NurbsSurface};

// doc:sample:start
#[derive(Clone, Copy, Debug)]
pub struct SurfaceSample {
    pub point: [f64; 3],
    /// Derivatives in the original knot parameters, not normalized parameters.
    pub du: [f64; 3],
    pub dv: [f64; 3],
    /// Oriented as du cross dv; None at a singular or numerically parallel pair.
    pub normal: Option<[f64; 3]>,
}
// doc:sample:end

pub(crate) fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
pub(crate) fn norm(a: [f64; 3]) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}
pub(crate) fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn unit(a: [f64; 3]) -> Option<[f64; 3]> {
    let scale = a.iter().fold(0.0_f64, |s, x| s.max(x.abs()));
    if scale == 0.0 || !scale.is_finite() {
        return None;
    }
    let a = a.map(|x| x / scale);
    let n = norm(a);
    Some(a.map(|x| x / n))
}

impl KnotAxis {
    pub(crate) fn span_at(&self, t: f64) -> Result<usize, DataError> {
        let [a, b] = self.domain();
        if !t.is_finite() || t < a || t > b {
            return Err(DataError::new(
                "parameter",
                "must be finite and inside the active domain",
            ));
        }
        if t == b {
            return Ok((self.degree()..self.pole_count())
                .rev()
                .find(|&i| self.knots()[i] < b)
                .unwrap());
        }
        Ok(self.knots().partition_point(|&k| k <= t) - 1)
    }

    /// Local Cox-de Boor recurrence with derivative propagated by the product rule.
    fn basis(&self, span: usize, t: f64) -> Vec<[f64; 2]> {
        let p = self.degree();
        let k = self.knots();
        let mut values = vec![[0.0; 2]; p + 1];
        values[0][0] = 1.0;
        for j in 1..=p {
            let mut saved = [0.0; 2];
            for r in 0..j {
                let left = t - k[span + 1 - j + r];
                let right = k[span + r + 1] - t;
                let denominator = k[span + r + 1] - k[span + 1 - j + r];
                let temp = if denominator == 0.0 {
                    [0.0; 2]
                } else {
                    values[r].map(|x| x / denominator)
                };
                values[r] = [
                    saved[0] + right * temp[0],
                    saved[1] - temp[0] + right * temp[1],
                ];
                saved = [left * temp[0], temp[0] + left * temp[1]];
            }
            values[j] = saved;
        }
        values
    }
}

impl NurbsSurface {
    /// Point and analytic first derivatives. Interior knots use the right-hand
    /// limit, domain endpoints the inward limit. No extrapolation or wrapping.
    pub fn evaluate(&self, u: f64, v: f64) -> Result<SurfaceSample, DataError> {
        let su = self.u().span_at(u).map_err(|e| e.prefix("u"))?;
        let sv = self.v().span_at(v).map_err(|e| e.prefix("v"))?;
        self.evaluate_in_spans([u, v], [su, sv])
    }

    /// Internal one-sided evaluation keeps discontinuities and crease normals intact.
    pub(crate) fn evaluate_in_spans(
        &self,
        uv: [f64; 2],
        spans: [usize; 2],
    ) -> Result<SurfaceSample, DataError> {
        let bu = self.u().basis(spans[0], uv[0]);
        let bv = self.v().basis(spans[1], uv[1]);
        let first_u = spans[0] - self.u().degree();
        let first_v = spans[1] - self.v().degree();
        let mut scale = 0.0_f64;
        for i in 0..bu.len() {
            for j in 0..bv.len() {
                scale = scale.max(self.weight(first_u + i, first_v + j).unwrap());
            }
        }
        let mut h = [[0.0; 4]; 3];
        for (i, a) in bu.iter().enumerate() {
            for (j, b) in bv.iter().enumerate() {
                let point = self.control_point(first_u + i, first_v + j).unwrap();
                let w = self.weight(first_u + i, first_v + j).unwrap() / scale;
                let coefficients = [a[0] * b[0] * w, a[1] * b[0] * w, a[0] * b[1] * w];
                for d in 0..3 {
                    for c in 0..3 {
                        h[d][c] += coefficients[d] * point[c];
                    }
                    h[d][3] += coefficients[d];
                }
            }
        }
        if h[0][3] <= 0.0 || h.iter().flatten().any(|x| !x.is_finite()) {
            return Err(DataError::new(
                "evaluation",
                "nonfinite homogeneous value or vanished denominator",
            ));
        }
        // doc:quotient:start
        let point = std::array::from_fn(|c| h[0][c] / h[0][3]);
        let du = std::array::from_fn(|c| (h[1][c] - point[c] * h[1][3]) / h[0][3]);
        let dv = std::array::from_fn(|c| (h[2][c] - point[c] * h[2][3]) / h[0][3]);
        // doc:quotient:end
        if [point, du, dv].iter().flatten().any(|x| !x.is_finite()) {
            return Err(DataError::new("evaluation", "point or derivative overflow"));
        }
        let normal = unit(du).zip(unit(dv)).and_then(|(a, b)| {
            let n = cross(a, b);
            (norm(n) > 64.0 * f64::EPSILON).then(|| unit(n)).flatten()
        });
        Ok(SurfaceSample {
            point,
            du,
            dv,
            normal,
        })
    }
}
