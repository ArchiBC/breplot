// STEP linework is drawn with Typst-native curves. The shaded surface is PNG.
#let engine = plugin("breplot.wasm")

#let native-lines(data, width: 100%, hidden: false, surface: true, surface-mode: "raster") = {
  let json-size = data.at(0) + 256 * data.at(1) + 65536 * data.at(2) + 16777216 * data.at(3)
  let scene = json(data.slice(4, 4 + json-size))
  let png = data.slice(4 + json-size)
  let view = scene.view
  layout(available => {
    let drawing-width = if type(width) == ratio { available.width * width } else { width }
    let unit = drawing-width / view.at(2)
    let drawing-height = view.at(3) * unit
    let xy(p) = ((p.at(0) - view.at(0)) * unit, (p.at(1) - view.at(1)) * unit)
    let stroke = calc.max(view.at(2), view.at(3)) * 0.0018 * unit
    let layers = []

    if surface and surface-mode == "raster" and png.len() > 0 {
      layers += place(top + left, image(png, format: "png", width: drawing-width,
        height: drawing-height, fit: "stretch"))
    }
    if surface and surface-mode == "flat" {
      for facet in scene.facets {
        let color = rgb(..facet.color)
        let a = xy(facet.points.at(0))
        let b = xy(facet.points.at(1))
        let c = xy(facet.points.at(2))
        layers += place(top + left, curve(
          fill: color,
          stroke: (paint: color, thickness: stroke * 0.25),
          curve.move(a), curve.line(b), curve.line(c), curve.line(a),
        ))
      }
    }

    let draw-cubics(curves, color, thickness, dash: none) = {
      if curves.len() == 0 { return none }
      let path = ()
      for cubic in curves {
        path.push(curve.move(xy(cubic.at(0))))
        path.push(curve.cubic(xy(cubic.at(1)), xy(cubic.at(2)), xy(cubic.at(3))))
      }
      place(top + left, curve(stroke: (paint: color, thickness: thickness,
        cap: "butt", join: "round", dash: dash), ..path))
    }
    let draw-lines(lines, color, thickness) = {
      if lines.len() == 0 { return none }
      let path = ()
      for segment in lines {
        path.push(curve.move(xy(segment.at(0))))
        path.push(curve.line(xy(segment.at(1))))
      }
      place(top + left, curve(stroke: (paint: color, thickness: thickness,
        cap: "butt", join: "round"), ..path))
    }

    if hidden {
      layers += draw-cubics(scene.hidden, rgb("#61758c").transparentize(if surface { 50% } else { 0% }),
        stroke * 0.6, dash: (stroke * 2.4, stroke * 1.8))
    }
    layers += draw-cubics(scene.visible,
      rgb(if surface { "#31475e" } else { "#172232" }).transparentize(if surface { 28% } else { 0% }),
      stroke * if surface { 0.72 } else { 1.0 })
    layers += draw-lines(scene.silhouette, rgb(if surface { "#1b2c40" } else { "#111827" }),
      stroke * if surface { 1.0 } else { 1.35 })
    box(width: drawing-width, height: drawing-height, layers)
  })
}

/// Draw a precomputed STEP payload. Refresh it when the STEP or camera changes.
#let step-view-data(data, width: 100%, hidden: false, surface: true, surface-mode: "raster") = {
  native-lines(data, width: width, hidden: hidden, surface: surface, surface-mode: surface-mode)
}

/// Render STEP data with native Typst linework and a direct PNG surface layer.
#let step-view(
  data,
  width: 100%,
  view: none,
  direction: (1, 1, -1),
  up: (0, 0, 1),
  deflection: 0.005,
  visibility-deflection: 0.001,
  surface-deflection: 0.02,
  max-segment-length: 0.5,
  hidden: false,
  silhouette: false,
  surface: true,
  surface-mode: "raster",
  surface-pixels: 1600,
  specular: 0.18,
) = {
  let settings = (
    view: if view == none { none } else {
      (view.x / 1deg, view.y / 1deg, view.z / 1deg)
    },
    direction: direction,
    up: up,
    deflection: deflection,
    visibility_deflection: visibility-deflection,
    surface_deflection: surface-deflection,
    max_segment_length: max-segment-length,
    hidden: hidden,
    silhouette: silhouette,
    surface: surface,
    surface_mode: surface-mode,
    surface_pixels: surface-pixels,
    specular: specular,
  )
  native-lines(engine.render_step_native(data, bytes(json.encode(settings))),
    width: width, hidden: hidden, surface: surface, surface-mode: surface-mode)
}
