// NURBS data is converted to world-space cubic controls by Rust/WASM.
// All drawing below uses Typst's native curve, circle, and text elements.
#let engine = plugin("breplot.wasm")

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

/// Render 2D or 3D NURBS with Typst-native curves.
/// `spec` contains degree, full knots, control_points, optional weights,
/// style, and a CeTZ-compatible `view: (x: ..., y: ..., z: ...)`.
#let nurbs-curve(spec, width: 100%) = {
  let request = (
    degree: spec.degree,
    knots: spec.knots,
    control_points: spec.control_points,
    weights: spec.at("weights", default: none),
    tolerance: spec.at("tolerance", default: 0.01),
  )
  let cubics = json(engine.nurbs_native_cubics(bytes(json.encode(request))))
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
  let view-height = max-y - min-y + 2 * pad

  layout(size => {
    let drawing-width = if type(width) == ratio { size.width * width } else { width }
    let unit = drawing-width / view-width
    let xy(p) = ((p.at(0) - min-x + pad) * unit, (max-y - p.at(1) + pad) * unit)
    let layers = []

    for axis in axes {
      let a = xy(project((0, 0, 0), view))
      let b = xy(axis.end)
      layers += place(top + left, curve(
        stroke: (paint: axis.color, thickness: 0.65pt, cap: "round"),
        curve.move(a), curve.line(b),
      ))
      layers += place(top + left, dx: b.at(0) + 2pt, dy: b.at(1) - 5pt,
        text(fill: axis.color, size: 8pt, weight: "bold", axis.label))
    }

    if style.at("show_control_polygon", default: false) {
      let polygon = (curve.move(xy(controls.first())),)
      for p in controls.slice(1) { polygon.push(curve.line(xy(p))) }
      layers += place(top + left, curve(
        stroke: (paint: paint(style.at("control_polygon_stroke", default: "#8795a5")),
          thickness: 0.55pt, dash: "dashed"),
        ..polygon,
      ))
    }

    let path = ()
    for segment in segments {
      path.push(curve.move(xy(segment.at(0))))
      path.push(curve.cubic(
        xy(segment.at(1)), xy(segment.at(2)), xy(segment.at(3)),
      ))
    }
    let stroke-color = paint(style.at("stroke", default: "#244b75"))
    let opacity = style.at("opacity", default: 1.0)
    let dash = style.at("dash", default: ())
    layers += place(top + left, curve(
      stroke: (paint: stroke-color.transparentize((1 - opacity) * 100%),
        thickness: style.at("stroke_width", default: extent * 0.0025) * unit,
        cap: "round", join: "round",
        dash: if dash.len() == 0 { none } else { dash.map(v => v * unit) }),
      ..path,
    ))

    for (i, p) in controls.enumerate() {
      let at = xy(p)
      let radius = style.at("control_point_radius", default: extent * 0.006) * unit
      if style.at("show_control_points", default: false) {
        layers += place(top + left, dx: at.at(0) - radius, dy: at.at(1) - radius,
          circle(radius: radius, fill: paint(style.at("control_point_fill", default: "#df7145"))))
      }
      if style.at("show_control_labels", default: false) {
        layers += place(top + left, dx: at.at(0) + radius + 2pt, dy: at.at(1) - radius - 5pt,
          text(fill: rgb("#39485a"), size: 8pt, "P" + str(i)))
      }
    }
    box(width: drawing-width, height: view-height * unit, layers)
  })
}
