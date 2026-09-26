// Standalone CeTZ extension. The WASM contains only NURBS geometry.
#import "@preview/cetz:0.5.2" as cetz
#let engine = plugin("cetz_nurbs.wasm")

/// Generate full-knot NURBS data with unit weights from a control polygon.
#let from-control-points(points, degree: 3, close: false, periodic: false) = {
  let request = (points: points, degree: degree, close: close, periodic: periodic)
  json(engine.nurbs_from_controls(bytes(json.encode(request))))
}

/// Interpolate every point using uniform clamped (or cyclic) Greville parameters.
#let from-interpolation-points(points, degree: 3, close: false, periodic: false) = {
  let request = (points: points, degree: degree, close: close, periodic: periodic)
  json(engine.nurbs_from_interpolation(bytes(json.encode(request))))
}

/// Model-space cubic controls. Rational/high-degree spans are approximated.
#let native-cubics(spec) = {
  assert(not ("close" in spec) and not ("closed" in spec) and not ("periodic" in spec),
    message: "Supply explicit knots, weights and control_points; close/periodic belong to the point constructors, not NURBS data")
  let request = (
    degree: spec.at("degree", default: none),
    knot_format: spec.at("knot_format", default: "full"),
    knots: spec.at("knots", default: ()),
    control_points: spec.control_points,
    weights: spec.at("weights", default: none),
    tolerance: spec.at("tolerance", default: 0.01),
  )
  json(engine.nurbs_native_cubics(bytes(json.encode(request))))
}

/// A CeTZ drawing element: no canvas, projection, or automatic rescaling.
/// Use inside cetz.canvas; apply draw.ortho for 3D. Styles are CeTZ styles.
/// Named groups expose curve and p0, p1, ... child anchors.
#let nurbs(spec, name: none, control-points: false, control-polygon: false,
  control-labels: false, point-radius: 0.06,
  point-fill: rgb("#df7145"), polygon-stroke: (paint: gray, dash: "dashed"),
  ..style) = {
  let cubics = native-cubics(spec)
  cetz.draw.group(name: name, {
    if control-polygon {
      cetz.draw.line(..spec.control_points, stroke: polygon-stroke)
    }
    cetz.draw.merge-path(name: "curve", join: false, {
      for c in cubics {
        cetz.draw.bezier(c.at(0), c.at(3), c.at(1), c.at(2))
      }
    }, ..style)
    for (i, p) in spec.control_points.enumerate() {
      cetz.draw.anchor("p" + str(i), p)
      if control-points {
        cetz.draw.circle(p, radius: point-radius, fill: point-fill, stroke: none)
      }
      if control-labels {
        cetz.draw.content(p, text(size: 8pt, "P" + str(i)), anchor: "south-west", padding: 3pt)
      }
    }
  })
}

// CeTZ draw.ortho uses Rx(x) * Ry(y) * Rz(z), with its Y rotation
// mapping (x, z) to (cos(y)*x - sin(y)*z, sin(y)*x + cos(y)*z).
#let project(point, view) = {
  let px = point.at(0)
  let py = point.at(1)
  let pz = point.at(2, default: 0)
  let cx = calc.cos(view.x)
  let sx = calc.sin(view.x)
  let cy = calc.cos(view.y)
  let sy = calc.sin(view.y)
  let cz = calc.cos(view.z)
  let sz = calc.sin(view.z)
  let rz-x = cz * px - sz * py
  let rz-y = sz * px + cz * py
  let ry-x = cy * rz-x - sy * pz
  let ry-z = sy * rz-x + cy * pz
  (ry-x, cx * rz-y - sx * ry-z)
}

#let paint(value) = if type(value) == str { rgb(value) } else { value }

/// Convenience content wrapper retaining the former spec/width interface.
/// New composed diagrams should use nurbs directly inside a CeTZ canvas.
#let nurbs-curve(spec, width: 100%) = {
  let cubics = native-cubics(spec)
  let is-3d = spec.control_points.first().len() == 3
  let view = spec.at("view", default: if is-3d {
    (x: 35.264deg, y: 45deg, z: 0deg)
  } else {
    (x: 0deg, y: 0deg, z: 0deg)
  })
  let style = spec.at("style", default: (:))
  let controls = spec.control_points.map(p => project(p, view))
  let segments = cubics.map(c => c.map(p => project(p, view)))
  let all-points = controls
  for segment in segments { all-points += segment }

  let axes = ()
  if is-3d and style.at("show_axes", default: false) {
    let world-min = spec.control_points.first()
    let world-max = spec.control_points.first()
    for p in spec.control_points {
      world-min = (
        calc.min(world-min.at(0), p.at(0)),
        calc.min(world-min.at(1), p.at(1)),
        calc.min(world-min.at(2), p.at(2)),
      )
      world-max = (
        calc.max(world-max.at(0), p.at(0)),
        calc.max(world-max.at(1), p.at(1)),
        calc.max(world-max.at(2), p.at(2)),
      )
    }
    let extent = calc.max(
      world-max.at(0) - world-min.at(0),
      world-max.at(1) - world-min.at(1),
      world-max.at(2) - world-min.at(2),
    )
    let axis-length = style.at("axis_length", default: extent * 0.25)
    axes = (
      (label: "X", end: project((axis-length, 0, 0), view), color: rgb("#bf514a")),
      (label: "Y", end: project((0, axis-length, 0), view), color: rgb("#47845f")),
      (label: "Z", end: project((0, 0, axis-length), view), color: rgb("#4678b5")),
    )
    all-points.push(project((0, 0, 0), view))
    for axis in axes { all-points.push(axis.end) }
  }

  let min-x = all-points.first().at(0)
  let max-x = min-x
  let min-y = all-points.first().at(1)
  let max-y = min-y
  for p in all-points {
    min-x = calc.min(min-x, p.at(0))
    max-x = calc.max(max-x, p.at(0))
    min-y = calc.min(min-y, p.at(1))
    max-y = calc.max(max-y, p.at(1))
  }
  let extent = calc.max(max-x - min-x, max-y - min-y, 0.000001)
  let pad = extent * 0.11
  let view-width = max-x - min-x + 2 * pad

  layout(size => {
    let drawing-width = if type(width) == ratio { size.width * width } else { width }
    let unit = drawing-width / view-width
    let stroke-color = paint(style.at("stroke", default: "#244b75"))
    let opacity = style.at("opacity", default: 1.0)
    assert(opacity >= 0 and opacity <= 1, message: "opacity must be between 0 and 1")
    let dash = style.at("dash", default: ())
    cetz.canvas(length: unit, padding: 0pt, {
      // Reserve the original model/control bounds for width-based layout.
      cetz.draw.hide(bounds: true, {
        cetz.draw.rect((min-x - pad, min-y - pad), (max-x + pad, max-y + pad))
      })
      cetz.draw.ortho(..view, sorted: false, {
        if axes.len() > 0 {
          let axis-length = style.at("axis_length", default: {
            let points = spec.control_points
            calc.max(..range(3).map(i => {
              let values = points.map(p => p.at(i))
              calc.max(..values) - calc.min(..values)
            })) * 0.25
          })
          for (i, label, color) in ((0, "X", rgb("#bf514a")), (1, "Y", rgb("#47845f")), (2, "Z", rgb("#4678b5"))) {
            let end = range(3).map(j => if i == j { axis-length } else { 0 })
            cetz.draw.line((0, 0, 0), end, stroke: (paint: color, thickness: 0.65pt))
            cetz.draw.content(end, text(fill: color, size: 8pt, label), anchor: "south-west", padding: 2pt)
          }
        }
        nurbs(spec,
          control-points: style.at("show_control_points", default: false),
          control-polygon: style.at("show_control_polygon", default: false),
          control-labels: style.at("show_control_labels", default: false),
          point-radius: style.at("control_point_radius", default: extent * 0.006),
          point-fill: paint(style.at("control_point_fill", default: "#df7145")),
          polygon-stroke: (paint: paint(style.at("control_polygon_stroke", default: "#8795a5")), thickness: 0.55pt, dash: "dashed"),
          stroke: (paint: stroke-color.transparentize((1 - opacity) * 100%),
            thickness: style.at("stroke_width", default: extent * 0.0025) * unit,
            cap: "round", join: "round",
            dash: if dash.len() == 0 { none } else { dash.map(v => v * unit) }),
        )
      })
    })
  })
}

// Annotation/analysis API evaluates the original rational curve, not its display.
#import "evaluation.typ" as evaluation
#let curve-domain(spec)=evaluation.domain(evaluation.normalize(spec))
#let curve-degree(spec)=evaluation.degree(evaluation.normalize(spec))
#let evaluate-point(spec,u)=evaluation.evaluate(spec,u)
#let evaluate-derivatives(spec,u)=evaluation.jet(spec,u)
#let curve-curvature(spec,u)=evaluation.curve-curvature(spec,u)

#let interpolate-at-parameters=evaluation.interpolate-at-parameters
