//! Convenience constructors. The core NURBS input never modifies control points.
use brepkit_math::nurbs::basis::{basis_funs_into, find_span};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PointInput {
    pub points: Vec<Vec<f64>>,
    #[serde(default = "default_degree")]
    pub degree: usize,
    #[serde(default)]
    pub close: bool,
    #[serde(default)]
    pub periodic: bool,
}

fn default_degree() -> usize {
    3
}

/// Standard full-knot NURBS data, directly accepted by the core renderer.
#[derive(Clone, Debug, Serialize)]
pub struct CurveData {
    pub knots: Vec<f64>,
    pub control_points: Vec<Vec<f64>>,
    pub weights: Vec<f64>,
}

fn prepare(input: &PointInput) -> Result<Vec<Vec<f64>>, String> {
    if !(1..=8).contains(&input.degree) {
        return Err("degree must be between 1 and 8".into());
    }
    let dim = input.points.first().map_or(0, Vec::len);
    if !(2..=3).contains(&dim)
        || input
            .points
            .iter()
            .any(|p| p.len() != dim || p.iter().any(|v| !v.is_finite()))
    {
        return Err(
            "points must have a consistent dimension of 2 or 3 and finite coordinates".into(),
        );
    }
    let mut points = input.points.clone();
    if input.periodic && points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    if points.len() < input.degree + 1 {
        return Err(
            "provide at least degree + 1 points (excluding the repeated periodic endpoint)".into(),
        );
    }
    if input.close && !input.periodic && points.first() != points.last() {
        points.push(points[0].clone());
    }
    Ok(points)
}

fn knots(count: usize, degree: usize, periodic: bool) -> Vec<f64> {
    if periodic {
        (0..count + 2 * degree + 1)
            .map(|i| i as f64 - degree as f64)
            .collect()
    } else {
        (0..count + degree + 1)
            .map(|i| {
                if i <= degree {
                    0.
                } else if i >= count {
                    (count - degree) as f64
                } else {
                    (i - degree) as f64
                }
            })
            .collect()
    }
}

fn data(mut points: Vec<Vec<f64>>, knots: Vec<f64>, input: &PointInput) -> CurveData {
    if input.periodic {
        for i in 0..input.degree {
            points.push(points[i].clone());
        }
    }
    CurveData {
        weights: vec![1.; points.len()],
        control_points: points,
        knots,
    }
}

pub fn from_controls(input: &PointInput) -> Result<CurveData, String> {
    let points = prepare(input)?;
    let knots = knots(points.len(), input.degree, input.periodic);
    Ok(data(points, knots, input))
}

fn parameters(count: usize, degree: usize, knots: &[f64], periodic: bool) -> Vec<f64> {
    if periodic {
        // A cyclic Greville grid. The half step for even degrees avoids the
        // singular even-point quadratic collocation at integer knots.
        let offset = if degree % 2 == 0 { 0.5 } else { 0. };
        (0..count).map(|i| i as f64 + offset).collect()
    } else {
        (0..count)
            .map(|i| knots[i + 1..=i + degree].iter().sum::<f64>() / degree as f64)
            .collect()
    }
}

pub fn from_interpolation(input: &PointInput) -> Result<CurveData, String> {
    let points = prepare(input)?;
    let count = points.len();
    if count > 512 {
        return Err("interpolation supports at most 512 points".into());
    }
    let knots = knots(count, input.degree, input.periodic);
    let parameters = parameters(count, input.degree, &knots, input.periodic);
    let expanded = count + if input.periodic { input.degree } else { 0 };
    let mut matrix = vec![vec![0.; count]; count];
    for (row, u) in parameters.into_iter().enumerate() {
        let span = find_span(expanded, input.degree, u, &knots);
        let mut basis = vec![0.; input.degree + 1];
        basis_funs_into(span, u, input.degree, &knots, &mut basis);
        for (j, value) in basis.into_iter().enumerate() {
            matrix[row][(span - input.degree + j) % count] += value;
        }
    }
    let controls = solve(matrix.clone(), points.clone())?;
    let scale = points.iter().flatten().map(|v| v.abs()).fold(1., f64::max);
    for (row, point) in matrix.iter().zip(&points) {
        for (axis, expected) in point.iter().enumerate() {
            let actual: f64 = row.iter().zip(&controls).map(|(a, p)| a * p[axis]).sum();
            if !actual.is_finite() || (actual - expected).abs() > 1e-9 * scale {
                return Err("interpolation residual exceeds numerical tolerance".into());
            }
        }
    }
    Ok(data(controls, knots, input))
}

fn solve(mut a: Vec<Vec<f64>>, mut b: Vec<Vec<f64>>) -> Result<Vec<Vec<f64>>, String> {
    let n = a.len();
    for col in 0..n {
        let pivot = (col..n)
            .max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))
            .unwrap();
        if a[pivot][col].abs() < 1e-12 {
            return Err("singular interpolation system".into());
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        let diagonal = a[col][col];
        for j in col..n {
            a[col][j] /= diagonal;
        }
        for v in &mut b[col] {
            *v /= diagonal;
        }
        for row in 0..n {
            if row == col {
                continue;
            }
            let factor = a[row][col];
            for j in col..n {
                a[row][j] -= factor * a[col][j];
            }
            for axis in 0..b[row].len() {
                b[row][axis] -= factor * b[col][axis];
            }
        }
    }
    if b.iter().flatten().any(|v| !v.is_finite()) {
        return Err("nonfinite interpolation controls".into());
    }
    Ok(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nurbs::{NurbsInput, extract_spans};

    fn input(degree: usize, close: bool, periodic: bool) -> PointInput {
        PointInput {
            points: (0..10)
                .map(|i| {
                    let t = i as f64 * 0.57;
                    vec![t.cos() * 2., t.sin(), (2. * t).sin()]
                })
                .collect(),
            degree,
            close,
            periodic,
        }
    }

    fn curve(data: &CurveData) -> brepkit_math::nurbs::curve::NurbsCurve {
        serde_json::from_value::<NurbsInput>(serde_json::to_value(data).unwrap())
            .unwrap()
            .curve()
            .unwrap()
    }

    #[test]
    fn uniform_clamped_controls_preserve_input_and_endpoints() {
        let input = input(3, false, false);
        let result = from_controls(&input).unwrap();
        assert_eq!(result.control_points, input.points);
        assert_eq!(&result.knots[..4], &[0.; 4]);
        assert_eq!(&result.knots[10..], &[7.; 4]);
        let curve = curve(&result);
        assert_eq!(curve.evaluate(0.), curve.control_points()[0]);
        assert_eq!(curve.evaluate(7.), *curve.control_points().last().unwrap());
    }

    #[test]
    fn interpolation_passes_all_points_for_all_degrees_and_modes() {
        for degree in 1..=8 {
            for (close, periodic) in [(false, false), (true, false), (false, true), (true, true)] {
                let input = input(degree, close, periodic);
                let points = prepare(&input).unwrap();
                let result = from_interpolation(&input).unwrap();
                let curve = curve(&result);
                let params = parameters(points.len(), degree, &result.knots, periodic);
                for (point, u) in points.iter().zip(params) {
                    let actual = curve.evaluate(u);
                    for (expected, actual) in point.iter().zip([actual.x(), actual.y(), actual.z()])
                    {
                        assert!(
                            (expected - actual).abs() < 1e-9,
                            "degree={degree}, close={close}, periodic={periodic}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn constructors_close_and_periodic_seams_are_distinct() {
        for build in [from_controls, from_interpolation] {
            for periodic in [false, true] {
                let input = input(3, true, periodic);
                let result = build(&input).unwrap();
                let curve = curve(&result);
                let (a, b) = curve.domain();
                assert!((curve.evaluate(a) - curve.evaluate(b)).length() < 1e-10);
                if periodic {
                    let spans = extract_spans(&curve).unwrap();
                    let first = &spans[0];
                    let last = spans.last().unwrap();
                    let jet = |s: &crate::nurbs::BezierSpan, end: bool| {
                        let p: Vec<_> = s
                            .controls
                            .iter()
                            .map(|h| brepkit_math::vec::Vec3::new(h[0], h[1], h[2]))
                            .collect();
                        let dt = s.range.1 - s.range.0;
                        if end {
                            [
                                (p[3] - p[2]) * (3. / dt),
                                (p[3] - p[2] * 2. + p[1]) * (6. / dt.powi(2)),
                            ]
                        } else {
                            [
                                (p[1] - p[0]) * (3. / dt),
                                (p[2] - p[1] * 2. + p[0]) * (6. / dt.powi(2)),
                            ]
                        }
                    };
                    for (a, b) in jet(first, false).into_iter().zip(jet(last, true)) {
                        assert!((a - b).length() < 1e-9);
                    }
                }
            }
        }
    }

    #[test]
    fn validates_inputs_and_accepts_one_repeated_periodic_endpoint() {
        let mut input = input(3, false, true);
        let expected = from_interpolation(&input).unwrap();
        input.points.push(input.points[0].clone());
        assert_eq!(
            from_interpolation(&input).unwrap().control_points,
            expected.control_points
        );
        input.points[0][0] = f64::NAN;
        assert!(from_controls(&input).is_err());
        input.points = vec![vec![0., 0.]; 3];
        assert!(from_interpolation(&input).is_err());
    }
}
