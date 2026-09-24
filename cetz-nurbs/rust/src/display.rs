use crate::bezier::Cubic3;
use crate::nurbs::{BezierSpan, NurbsInput, extract_spans};
use brepkit_math::nurbs::curve::NurbsCurve;
use brepkit_math::vec::Point3;

#[derive(Clone, Copy)]
pub struct DisplaySegment {
    pub cubic: Cubic3,
    pub range: (f64, f64),
}

fn project(h: [f64; 4]) -> Point3 {
    Point3::new(h[0] / h[3], h[1] / h[3], h[2] / h[3])
}

fn exact_polynomial_cubic(span: &BezierSpan) -> Option<Cubic3> {
    if !span.is_polynomial() || span.degree() > 3 {
        return None;
    }
    let p: Vec<Point3> = span.controls.iter().copied().map(project).collect();
    let points = match p.as_slice() {
        [a, b] => [
            *a,
            *a + (*b - *a) * (1.0 / 3.0),
            *a + (*b - *a) * (2.0 / 3.0),
            *b,
        ],
        [a, b, c] => [
            *a,
            *a + (*b - *a) * (2.0 / 3.0),
            *c + (*b - *c) * (2.0 / 3.0),
            *c,
        ],
        [a, b, c, d] => [*a, *b, *c, *d],
        _ => return None,
    };
    Some(Cubic3 { points })
}

fn fit_cubic(span: &BezierSpan) -> Cubic3 {
    let p = span.degree() as f64;
    let controls = &span.controls;
    let a = project(controls[0]);
    let b = project(controls[controls.len() - 1]);
    let left = project(controls[1]);
    let right = project(controls[controls.len() - 2]);
    let d0 = (left - a) * (p * controls[1][3] / controls[0][3]);
    let d1 = (b - right) * (p * controls[controls.len() - 2][3] / controls[controls.len() - 1][3]);
    Cubic3 {
        points: [a, a + d0 * (1.0 / 3.0), b - d1 * (1.0 / 3.0), b],
    }
}

pub fn append_span(
    span: BezierSpan,
    tolerance: f64,
    depth: u8,
    output: &mut Vec<DisplaySegment>,
) -> Result<(), String> {
    if let Some(cubic) = exact_polynomial_cubic(&span) {
        output.push(DisplaySegment {
            cubic,
            range: span.range,
        });
        return Ok(());
    }
    let cubic = fit_cubic(&span);
    let error = [0.125, 0.25, 0.5, 0.75, 0.875]
        .into_iter()
        .map(|t| (span.evaluate(t) - cubic.evaluate(t)).length())
        .fold(0.0_f64, f64::max);
    if error <= tolerance {
        output.push(DisplaySegment {
            cubic,
            range: span.range,
        });
        return Ok(());
    }
    if depth == 20 || output.len() >= 65536 {
        return Err("NURBS cubic approximation exceeded subdivision budget".into());
    }
    let (left, right) = span.split();
    append_span(left, tolerance, depth + 1, output)?;
    append_span(right, tolerance, depth + 1, output)?;
    Ok(())
}

pub fn display_segments(curve: &NurbsCurve, tolerance: f64) -> Result<Vec<DisplaySegment>, String> {
    let mut output = Vec::new();
    for span in extract_spans(curve)? {
        append_span(span, tolerance, 0, &mut output)?;
    }
    Ok(output)
}

/// Cubic control points in world coordinates for Typst's native `curve.cubic`.
/// Affine orthographic projection is applied in Typst after this conversion.
pub fn native_cubics(input: &NurbsInput) -> Result<Vec<[[f64; 3]; 4]>, String> {
    let curve = input.curve()?;
    Ok(display_segments(&curve, input.tolerance)?
        .into_iter()
        .map(|segment| segment.cubic.points.map(|p| [p.x(), p.y(), p.z()]))
        .collect())
}

/// Return NURBS edge cubics over the topological edge's parameter interval.
/// Reversed trims preserve the same orientation as the old edge evaluator.
pub fn edge_cubics(
    curve: &NurbsCurve,
    interval: (f64, f64),
    tolerance: f64,
) -> Result<Vec<Cubic3>, String> {
    let reverse = interval.0 > interval.1;
    let low = interval.0.min(interval.1);
    let high = interval.0.max(interval.1);
    let mut result = Vec::new();
    for segment in display_segments(curve, tolerance)? {
        let (a, b) = segment.range;
        let start = low.max(a);
        let end = high.min(b);
        if end <= start {
            continue;
        }
        result.push(
            segment
                .cubic
                .portion((start - a) / (b - a), (end - a) / (b - a)),
        );
    }
    if reverse {
        result.reverse();
        for cubic in &mut result {
            cubic.points.reverse();
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> NurbsInput {
        serde_json::from_str(
            r#"{
            "degree": 2, "knots": [0,0,0,1,1,1],
            "control_points": [[1,0,2],[1,1,3],[0,1,5]]
        }"#,
        )
        .unwrap()
    }

    #[test]
    fn eight_independent_controls_form_a_repeated_knot_circle() {
        let mut input: NurbsInput = serde_json::from_str(
            r#"{
            "knot_format":"rhino",
            "knots":[0,0,1,1,2,2,3,3,4,4],
            "control_points":[[2,0],[2,2],[0,2],[-2,2],[-2,0],[-2,-2],[0,-2],[2,-2],[2,0]],
            "tolerance":0.001
        }"#,
        )
        .unwrap();
        input.weights = Some(
            (0..9)
                .map(|i| {
                    if i % 2 == 0 {
                        1.
                    } else {
                        std::f64::consts::FRAC_1_SQRT_2
                    }
                })
                .collect(),
        );
        let curve = input.curve().unwrap();
        assert_eq!(curve.control_points().len(), 9);
        assert_eq!(extract_spans(&curve).unwrap().len(), 4);
        for i in 0..=400 {
            let p = curve.evaluate(i as f64 / 100.);
            assert!((p.x().hypot(p.y()) - 2.).abs() < 1e-12);
        }
        let display = display_segments(&curve, input.tolerance).unwrap();
        for segment in &display {
            for i in 0..=20 {
                let t = i as f64 / 20.;
                let u = segment.range.0 + (segment.range.1 - segment.range.0) * t;
                assert!((segment.cubic.evaluate(t) - curve.evaluate(u)).length() < input.tolerance);
            }
        }
        assert!(
            (display[0].cubic.points[0] - display.last().unwrap().cubic.points[3]).length() < 1e-12
        );
    }

    // Analytic rational endpoint derivatives from homogeneous Bernstein controls.
    // brepkit-math 3.4.18 derivatives(u, 2) returns nonfinite values on this
    // non-clamped seam; rendering uses basis evaluation, not that API.
    fn endpoint_jet(span: &BezierSpan, end: bool) -> [brepkit_math::vec::Vec3; 3] {
        use brepkit_math::vec::Vec3;
        let p = span.degree();
        let h = &span.controls;
        let (a, b, c, sign) = if end {
            (h[p], h[p - 1], h[p - 2], -1.)
        } else {
            (h[0], h[1], h[2], 1.)
        };
        let dt = span.range.1 - span.range.0;
        let d1: [f64; 4] = std::array::from_fn(|i| sign * p as f64 * (b[i] - a[i]) / dt);
        let d2: [f64; 4] =
            std::array::from_fn(|i| (p * (p - 1)) as f64 * (c[i] - 2. * b[i] + a[i]) / (dt * dt));
        let xyz = |v: [f64; 4]| Vec3::new(v[0], v[1], v[2]);
        let c0 = xyz(a) * (1. / a[3]);
        let c1 = (xyz(d1) - c0 * d1[3]) * (1. / a[3]);
        let c2 = (xyz(d2) - c1 * (2. * d1[3]) - c0 * d2[3]) * (1. / a[3]);
        [c0, c1, c2]
    }

    fn periodic_input() -> NurbsInput {
        serde_json::from_str(r#"{
            "knots":[-3,-2,-1,0,1,2,3,4,5,6,7,8,9,10],
            "control_points":[[2,0,0],[2,1.5,1],[0.4,2.5,2],[-1.8,1.5,0],[-2,-0.8,-1],[-0.6,-2,0],[1.8,-1.5,1],[2,0,0],[2,1.5,1],[0.4,2.5,2]],
            "tolerance":0.001
        }"#).unwrap()
    }

    #[test]
    fn periodic_curve_preserves_position_and_two_derivatives_at_seam() {
        for rational in [false, true] {
            let mut input = periodic_input();
            if rational {
                input.weights = Some(vec![1., 0.8, 1.2, 1., 0.7, 1.1, 1., 1., 0.8, 1.2]);
            }
            let curve = input.curve().unwrap();
            assert_eq!(curve.control_points().len(), 10);
            assert_eq!(curve.domain(), (0., 7.));
            assert!((curve.evaluate(0.) - curve.evaluate(7.)).length() < 1e-12);
            let spans = extract_spans(&curve).unwrap();
            let a = endpoint_jet(&spans[0], false);
            let b = endpoint_jet(spans.last().unwrap(), true);
            for i in 0..=2 {
                assert!(
                    (a[i] - b[i]).length() < 1e-10,
                    "rational={rational}, derivative={i}"
                );
            }
            let display = display_segments(&curve, input.tolerance).unwrap();
            assert!(
                (display[0].cubic.points[0] - display.last().unwrap().cubic.points[3]).length()
                    < 1e-12
            );
            for segment in display {
                for i in 0..=20 {
                    let t = i as f64 / 20.;
                    let u = segment.range.0 + (segment.range.1 - segment.range.0) * t;
                    assert!(
                        (segment.cubic.evaluate(t) - curve.evaluate(u)).length() < input.tolerance
                    );
                }
            }
        }
    }

    #[test]
    fn periodic_nonuniform_full_and_rhino_match() {
        let mut input = periodic_input();
        // One nonuniform period, extended by three knots on either side.
        input.knots = vec![
            -3., -2.5, -1., 0., 0.5, 1.5, 2., 4., 4.5, 6., 7., 7.5, 8.5, 9.,
        ];
        let curve = input.curve().unwrap();
        let spans = extract_spans(&curve).unwrap();
        for (a, b) in endpoint_jet(&spans[0], false)
            .iter()
            .zip(endpoint_jet(spans.last().unwrap(), true))
        {
            assert!((*a - b).length() < 1e-10);
        }
        input.knot_format = crate::nurbs::KnotFormat::Rhino;
        input.knots = input.knots[1..input.knots.len() - 1].to_vec();
        let rhino = input.curve().unwrap();
        assert_eq!(rhino.control_points(), curve.control_points());
        for i in 0..=70 {
            let u = i as f64 / 10.;
            assert!((curve.evaluate(u) - rhino.evaluate(u)).length() < 1e-12);
        }
    }

    #[test]
    fn native_cubics_keep_world_coordinates_for_cetz_projection() {
        let input = input();
        let output = native_cubics(&input).unwrap();
        assert_eq!(output.len(), 1);
        assert_eq!(output[0][0], [1., 0., 2.]);
        assert_eq!(output[0][3], [0., 1., 5.]);
        let curve = input.curve().unwrap();
        let cubic = Cubic3 {
            points: output[0].map(|p| Point3::new(p[0], p[1], p[2])),
        };
        for t in [0., 0.25, 0.5, 0.75, 1.] {
            assert!((cubic.evaluate(t) - curve.evaluate(t)).length() < 1e-12);
        }
    }

    #[test]
    fn standalone_input_rejects_invalid_geometry() {
        let mut bad = input();
        bad.knots.pop();
        assert!(native_cubics(&bad).unwrap_err().contains("knots"));
        let mut bad = input();
        bad.weights = Some(vec![1., 0., 1.]);
        assert!(native_cubics(&bad).unwrap_err().contains("weights"));
        let mut bad = input();
        bad.control_points[0] = vec![1.];
        assert!(native_cubics(&bad).is_err());
        let mut bad = input();
        bad.tolerance = 0.;
        assert!(native_cubics(&bad).unwrap_err().contains("tolerance"));
    }
    fn curve(degree: usize, knots: Vec<f64>, points: &[[f64; 2]], weights: Vec<f64>) -> NurbsCurve {
        NurbsCurve::new(
            degree,
            knots,
            points
                .iter()
                .map(|p| Point3::new(p[0], p[1], 0.0))
                .collect(),
            weights,
        )
        .unwrap()
    }

    #[test]
    fn polynomial_multi_span_is_exact() {
        let c = curve(
            3,
            vec![0., 0., 0., 0., 0.4, 1., 1., 1., 1.],
            &[[0., 0.], [1., 3.], [2., 2.], [4., 3.], [5., 0.]],
            vec![1.; 5],
        );
        let display = display_segments(&c, 1e-6).unwrap();
        assert_eq!(display.len(), 2);
        for segment in display {
            for t in [0., 0.1, 0.25, 0.5, 0.75, 0.9, 1.] {
                let u = segment.range.0 + (segment.range.1 - segment.range.0) * t;
                assert!((segment.cubic.evaluate(t) - c.evaluate(u)).length() < 1e-9);
            }
        }
    }

    #[test]
    fn rational_quarter_circle_stays_within_tolerance() {
        let c = curve(
            2,
            vec![0., 0., 0., 1., 1., 1.],
            &[[1., 0.], [1., 1.], [0., 1.]],
            vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
        );
        let display = display_segments(&c, 0.001).unwrap();
        assert!(display.len() < 16);
        for segment in display {
            for i in 0..=20 {
                let t = i as f64 / 20.;
                let u = segment.range.0 + (segment.range.1 - segment.range.0) * t;
                assert!((segment.cubic.evaluate(t) - c.evaluate(u)).length() < 0.001);
            }
        }
    }

    #[test]
    fn reversed_trim_has_reversed_endpoints() {
        let c = curve(
            2,
            vec![0., 0., 0., 1., 1., 1.],
            &[[0., 0.], [1., 2.], [2., 0.]],
            vec![1.; 3],
        );
        let pieces = edge_cubics(&c, (0.8, 0.2), 0.001).unwrap();
        assert!((pieces.first().unwrap().points[0] - c.evaluate(0.8)).length() < 1e-10);
        assert!((pieces.last().unwrap().points[3] - c.evaluate(0.2)).length() < 1e-10);
    }

    #[test]
    fn nonclamped_spans_match_curve() {
        let c = curve(
            3,
            vec![0., 0.1, 0.2, 0.3, 0.5, 0.7, 0.8, 0.9, 1.],
            &[[0., 0.], [1., 2.], [2., -1.], [3., 3.], [4., 0.]],
            vec![1.; 5],
        );
        let spans = display_segments(&c, 1e-8).unwrap();
        assert_eq!(spans.len(), 2);
        for span in spans {
            for i in 0..=10 {
                let t = i as f64 / 10.;
                let u = span.range.0 + (span.range.1 - span.range.0) * t;
                assert!((span.cubic.evaluate(t) - c.evaluate(u)).length() < 1e-9);
            }
        }
    }

    #[test]
    fn fully_repeated_interior_knot_keeps_disconnected_spans() {
        let c = curve(
            2,
            vec![0., 0., 0., 0.5, 0.5, 0.5, 1., 1., 1.],
            &[[0., 0.], [1., 1.], [2., 0.], [4., 0.], [5., 1.], [6., 0.]],
            vec![1.; 6],
        );
        let display = display_segments(&c, 1e-8).unwrap();
        assert_eq!(display.len(), 2);
        assert!((display[0].cubic.points[3] - Point3::new(2., 0., 0.)).length() < 1e-9);
        assert!((display[1].cubic.points[0] - Point3::new(4., 0., 0.)).length() < 1e-9);
    }

    #[test]
    fn high_degree_polynomial_is_displayed_with_cubic_tolerance() {
        let points = [
            [0., 0.],
            [1., 2.],
            [2., -1.],
            [3., 3.],
            [4., 0.],
            [5., 2.],
            [6., -2.],
            [7., 1.],
            [8., 0.],
        ];
        let c = curve(8, [vec![0.; 9], vec![1.; 9]].concat(), &points, vec![1.; 9]);
        let display = display_segments(&c, 0.001).unwrap();
        assert!(!display.is_empty());
        for segment in display {
            for i in 0..=20 {
                let t = i as f64 / 20.;
                let u = segment.range.0 + (segment.range.1 - segment.range.0) * t;
                assert!((segment.cubic.evaluate(t) - c.evaluate(u)).length() < 0.001);
            }
        }
    }
}
