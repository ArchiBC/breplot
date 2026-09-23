//! Vector display of polynomial and rational NURBS Bézier spans.
//! Polynomial degrees 1–3 become exact display cubics (up to coordinate rounding).
//! Rational and higher-degree spans use adaptive cubic approximation.

use brepkit_math::nurbs::curve::NurbsCurve;
use brepkit_math::vec::Point3;

use crate::bezier::Cubic3;
use crate::camera::Camera;
use crate::nurbs::{BezierSpan, NurbsInput, extract_spans};

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

fn append_span(
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

struct CircleArc {
    start: Point3,
    end: Point3,
    radius: f64,
    large: u8,
    sweep: u8,
}

/// SVG A can represent a circular rational quadratic exactly in 2D.
/// A circle equation composed with a rational quadratic has degree at most
/// four, so checking five parameter values detects the identity numerically.
fn circular_arc(span: &BezierSpan) -> Option<CircleArc> {
    if span.degree() != 2 || span.is_polynomial() {
        return None;
    }
    let a = span.evaluate(0.0);
    let m = span.evaluate(0.5);
    let b = span.evaluate(1.0);
    let (ax, ay, mx, my, bx, by) = (a.x(), a.y(), m.x(), m.y(), b.x(), b.y());
    let det = 2.0 * (ax * (my - by) + mx * (by - ay) + bx * (ay - my));
    let scale = (a - m).length().max((m - b).length());
    if det.abs() < scale * scale * 1e-12 {
        return None;
    }
    let a2 = ax * ax + ay * ay;
    let m2 = mx * mx + my * my;
    let b2 = bx * bx + by * by;
    let cx = (a2 * (my - by) + m2 * (by - ay) + b2 * (ay - my)) / det;
    let cy = (a2 * (bx - mx) + m2 * (ax - bx) + b2 * (mx - ax)) / det;
    let radius = ((ax - cx).powi(2) + (ay - cy).powi(2)).sqrt();
    if !radius.is_finite() || radius <= 0.0 {
        return None;
    }
    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let p = span.evaluate(t);
        let r = ((p.x() - cx).powi(2) + (p.y() - cy).powi(2)).sqrt();
        if (r - radius).abs() > 1e-10 * radius.max(1.0) {
            return None;
        }
    }
    let first_control = project(span.controls[1]);
    let tangent_x = first_control.x() - ax;
    let tangent_y = first_control.y() - ay;
    let sweep = u8::from((ax - cx) * tangent_y - (ay - cy) * tangent_x > 0.0);
    let start_angle = (ay - cy).atan2(ax - cx);
    let end_angle = (by - cy).atan2(bx - cx);
    let delta = if sweep == 1 {
        (end_angle - start_angle).rem_euclid(std::f64::consts::TAU)
    } else {
        (start_angle - end_angle).rem_euclid(std::f64::consts::TAU)
    };
    if delta < 1e-10 {
        return None;
    }
    Some(CircleArc {
        start: a,
        end: b,
        radius,
        large: u8::from(delta > std::f64::consts::PI),
        sweep,
    })
}

enum VectorPart {
    Cubic(DisplaySegment),
    Arc(CircleArc),
}

fn project_xy(camera: &Camera, point: Point3) -> Point3 {
    let projected = camera.project(point);
    Point3::new(projected.x, projected.y, 0.0)
}

pub fn render_nurbs(input: &NurbsInput) -> Result<String, String> {
    let world_curve = input.curve()?;
    let camera = Camera::new(input.direction, input.up)?;
    // Orthographic projection is affine. Projecting the controls while keeping
    // their knots and weights produces the exact 2D NURBS projection.
    let projected = world_curve
        .control_points()
        .iter()
        .map(|&p| project_xy(&camera, p))
        .collect();
    let curve = NurbsCurve::new(
        world_curve.degree(),
        world_curve.knots().to_vec(),
        projected,
        world_curve.weights().to_vec(),
    )
    .map_err(|e| format!("projected NURBS: {e}"))?;
    let mut parts = Vec::new();
    for span in extract_spans(&curve)? {
        if let Some(arc) = circular_arc(&span) {
            parts.push(VectorPart::Arc(arc));
        } else {
            let mut cubics = Vec::new();
            append_span(span, input.tolerance, 0, &mut cubics)?;
            parts.extend(cubics.into_iter().map(VectorPart::Cubic));
        }
    }
    let points = curve.control_points();
    let world_points = world_curve.control_points();
    let world_extent = (0..3)
        .map(|coordinate| {
            let value = |p: &Point3| match coordinate {
                0 => p.x(),
                1 => p.y(),
                _ => p.z(),
            };
            world_points
                .iter()
                .map(value)
                .fold(f64::NEG_INFINITY, f64::max)
                - world_points.iter().map(value).fold(f64::INFINITY, f64::min)
        })
        .fold(0.0_f64, f64::max)
        .max(1e-6);
    let axis_length = input.style.axis_length.unwrap_or(world_extent * 0.25);
    let axis_origin = project_xy(&camera, Point3::new(0.0, 0.0, 0.0));
    let axes = if input.style.show_axes {
        vec![
            (
                "X",
                project_xy(&camera, Point3::new(axis_length, 0.0, 0.0)),
                "#bf514a",
            ),
            (
                "Y",
                project_xy(&camera, Point3::new(0.0, axis_length, 0.0)),
                "#47845f",
            ),
            (
                "Z",
                project_xy(&camera, Point3::new(0.0, 0.0, axis_length)),
                "#4678b5",
            ),
        ]
    } else {
        Vec::new()
    };
    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for point in points
        .iter()
        .copied()
        .chain(axes.iter().flat_map(|(_, end, _)| [axis_origin, *end]))
    {
        bounds[0] = bounds[0].min(point.x());
        bounds[1] = bounds[1].min(point.y());
        bounds[2] = bounds[2].max(point.x());
        bounds[3] = bounds[3].max(point.y());
    }
    let width = (bounds[2] - bounds[0]).max(1e-6);
    let height = (bounds[3] - bounds[1]).max(1e-6);
    let extent = width.max(height);
    let pad = extent * 0.08;
    let stroke = input.style.stroke_width.unwrap_or(extent * 0.0025);
    let radius = input.style.control_point_radius.unwrap_or(extent * 0.006);
    let font_size = extent * 0.035;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{:.8} {:.8} {:.8} {:.8}\" width=\"800\" height=\"{:.4}\">\n",
        bounds[0] - pad,
        bounds[1] - pad,
        width + 2.0 * pad,
        height + 2.0 * pad,
        800.0 * (height + 2.0 * pad) / (width + 2.0 * pad)
    );
    if !axes.is_empty() {
        svg.push_str("<g id=\"world-axes\">\n");
        for (label, end, color) in &axes {
            let dx = end.x() - axis_origin.x();
            let dy = end.y() - axis_origin.y();
            let length = dx.hypot(dy);
            if length < 1e-8 {
                continue;
            }
            let (ux, uy) = (dx / length, dy / length);
            let head = (extent * 0.025).min(length * 0.3);
            let half_head = head * 0.4;
            let base_x = end.x() - ux * head;
            let base_y = end.y() - uy * head;
            svg.push_str(&format!(
                "<line id=\"axis-{label}\" x1=\"{:.8}\" y1=\"{:.8}\" x2=\"{:.8}\" y2=\"{:.8}\" stroke=\"{color}\" stroke-width=\"{:.8}\"/>\n",
                axis_origin.x(), axis_origin.y(), end.x(), end.y(), stroke * 1.2
            ));
            svg.push_str(&format!(
                "<polygon fill=\"{color}\" points=\"{:.8},{:.8} {:.8},{:.8} {:.8},{:.8}\"/>\n",
                end.x(),
                end.y(),
                base_x - uy * half_head,
                base_y + ux * half_head,
                base_x + uy * half_head,
                base_y - ux * half_head
            ));
            svg.push_str(&format!(
                "<text x=\"{:.8}\" y=\"{:.8}\" fill=\"{color}\" font-family=\"Arial,sans-serif\" font-size=\"{font_size:.8}\" font-weight=\"bold\">{label}</text>\n",
                end.x() + ux * font_size * 0.3,
                end.y() + uy * font_size * 0.3
            ));
        }
        svg.push_str("</g>\n");
    }
    if input.style.show_control_polygon {
        svg.push_str(&format!(
            "<polyline id=\"control-polygon\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.8}\" stroke-dasharray=\"{:.8} {:.8}\" points=\"",
            input.style.control_polygon_stroke, stroke * 0.65, stroke * 3.0, stroke * 2.0
        ));
        for p in points {
            svg.push_str(&format!("{:.8},{:.8} ", p.x(), p.y()));
        }
        svg.push_str("\"/>\n");
    }
    let dash = if input.style.dash.is_empty() {
        String::new()
    } else {
        format!(
            " stroke-dasharray=\"{}\"",
            input
                .style
                .dash
                .iter()
                .map(|v| format!("{v:.8}"))
                .collect::<Vec<_>>()
                .join(" ")
        )
    };
    svg.push_str(&format!(
        "<path id=\"curve\" fill=\"none\" stroke=\"{}\" stroke-width=\"{stroke:.8}\" stroke-opacity=\"{:.4}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"{dash} d=\"",
        input.style.stroke, input.style.opacity
    ));
    let mut last: Option<Point3> = None;
    for part in &parts {
        let (a, end) = match part {
            VectorPart::Cubic(segment) => (segment.cubic.points[0], segment.cubic.points[3]),
            VectorPart::Arc(arc) => (arc.start, arc.end),
        };
        if !last.is_some_and(|prev| (prev - a).length() < 1e-8) {
            svg.push_str(&format!("M{:.8},{:.8}", a.x(), a.y()));
        }
        match part {
            VectorPart::Cubic(segment) => {
                let [_, b, c, d] = segment.cubic.points;
                svg.push_str(&format!(
                    "C{:.8},{:.8} {:.8},{:.8} {:.8},{:.8}",
                    b.x(),
                    b.y(),
                    c.x(),
                    c.y(),
                    d.x(),
                    d.y()
                ));
            }
            VectorPart::Arc(arc) => {
                svg.push_str(&format!(
                    "A{:.8},{:.8} 0 {} {} {:.8},{:.8}",
                    arc.radius,
                    arc.radius,
                    arc.large,
                    arc.sweep,
                    arc.end.x(),
                    arc.end.y()
                ));
            }
        }
        last = Some(end);
    }
    svg.push_str("\"/>\n");
    if input.style.show_control_points {
        svg.push_str("<g id=\"control-points\">\n");
        for (index, p) in points.iter().enumerate() {
            svg.push_str(&format!(
                "<circle data-index=\"{index}\" cx=\"{:.8}\" cy=\"{:.8}\" r=\"{radius:.8}\" fill=\"{}\"/>\n",
                p.x(), p.y(), input.style.control_point_fill
            ));
        }
        svg.push_str("</g>\n");
    }
    if input.style.show_control_labels {
        svg.push_str(
            "<g id=\"control-labels\" font-family=\"Arial,sans-serif\" fill=\"#39485a\">\n",
        );
        for (index, p) in points.iter().enumerate() {
            svg.push_str(&format!(
                "<text x=\"{:.8}\" y=\"{:.8}\" font-size=\"{font_size:.8}\">P{index}</text>\n",
                p.x() + radius * 1.4,
                p.y() - radius * 1.4
            ));
        }
        svg.push_str("</g>\n");
    }
    svg.push_str("</svg>\n");
    Ok(svg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use brepkit_math::vec::Point3;

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
    fn projection_of_3d_rational_nurbs_is_a_2d_nurbs() {
        let world = NurbsCurve::new(
            2,
            vec![0., 0., 0., 1., 1., 1.],
            vec![
                Point3::new(1., 0., 2.),
                Point3::new(1., 1., 3.),
                Point3::new(0., 1., 5.),
            ],
            vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
        )
        .unwrap();
        let camera = Camera::new([1., 2., -3.], [0., 0., 1.]).unwrap();
        let screen = NurbsCurve::new(
            2,
            world.knots().to_vec(),
            world
                .control_points()
                .iter()
                .map(|&p| {
                    let q = camera.project(p);
                    Point3::new(q.x, q.y, 0.)
                })
                .collect(),
            world.weights().to_vec(),
        )
        .unwrap();
        for i in 0..=20 {
            let u = i as f64 / 20.;
            let a = camera.project(world.evaluate(u));
            let b = screen.evaluate(u);
            assert!((a.x - b.x()).abs() < 1e-12);
            assert!((a.y - b.y()).abs() < 1e-12);
        }
    }

    #[test]
    fn world_axes_and_labels_follow_the_3d_camera() {
        let input: NurbsInput =
            serde_json::from_str(include_str!("../../examples/nurbs-polynomial.json")).unwrap();
        assert!(input.control_points.iter().all(|point| point.len() == 3));
        let svg = render_nurbs(&input).unwrap();
        let camera = Camera::new(input.direction, input.up).unwrap();
        let length = input.style.axis_length.unwrap();
        for (label, end) in [
            ("X", Point3::new(length, 0.0, 0.0)),
            ("Y", Point3::new(0.0, length, 0.0)),
            ("Z", Point3::new(0.0, 0.0, length)),
        ] {
            let projected = camera.project(end);
            let line = svg
                .lines()
                .find(|line| line.contains(&format!("id=\"axis-{label}\"")))
                .unwrap();
            assert!(line.contains(&format!(
                "x2=\"{:.8}\" y2=\"{:.8}\"",
                projected.x, projected.y
            )));
        }
        assert!(svg.contains("id=\"control-labels\""));
        assert!(svg.contains(">P4</text>"));
    }

    #[test]
    fn rational_circle_uses_one_svg_arc() {
        let input: NurbsInput =
            serde_json::from_str(include_str!("../../examples/nurbs-rational.json")).unwrap();
        let svg = render_nurbs(&input).unwrap();
        assert!(svg.contains("A70.00000000,70.00000000 0 0 0"));
        assert_eq!(svg.matches("<circle ").count(), 3);
        assert!(svg.contains("id=\"control-polygon\""));
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

    #[test]
    fn noncircular_rational_quadratic_uses_cubics() {
        let mut input: NurbsInput =
            serde_json::from_str(include_str!("../../examples/nurbs-rational.json")).unwrap();
        input.weights = Some(vec![1., 0.5, 1.]);
        let svg = render_nurbs(&input).unwrap();
        assert!(svg.contains("C"));
        assert!(!svg.contains("A70."));
    }
}
