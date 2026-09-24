//! Vector display of polynomial and rational NURBS Bézier spans.
//! Polynomial degrees 1–3 become exact display cubics (up to coordinate rounding).
//! Rational and higher-degree spans use adaptive cubic approximation.

use brepkit_math::nurbs::curve::NurbsCurve;
use brepkit_math::vec::Point3;

use crate::camera::Camera;
use cetz_nurbs::nurbs::{BezierSpan, NurbsInput, extract_spans};

pub use cetz_nurbs::display::edge_cubics;
use cetz_nurbs::display::{DisplaySegment, append_span};

fn project(h: [f64; 4]) -> Point3 {
    Point3::new(h[0] / h[3], h[1] / h[3], h[2] / h[3])
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
    fn noncircular_rational_quadratic_uses_cubics() {
        let mut input: NurbsInput =
            serde_json::from_str(include_str!("../../examples/nurbs-rational.json")).unwrap();
        input.weights = Some(vec![1., 0.5, 1.]);
        let svg = render_nurbs(&input).unwrap();
        assert!(svg.contains("C"));
        assert!(!svg.contains("A70."));
    }
}
