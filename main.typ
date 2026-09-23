#import "package/lib.typ": step-view, step-view-data, nurbs-curve

#let preview-config = json("examples/step-preview.json")
#let step-camera = (
  x: preview-config.view.at(0) * 1deg,
  y: preview-config.view.at(1) * 1deg,
  z: preview-config.view.at(2) * 1deg,
)
#let quality = sys.inputs.at("quality", default: "final")

#let cubic-3d = (
  degree: 3,
  knots: (0, 0, 0, 0, 0.45, 1, 1, 1, 1),
  control_points: (
    (0, 0, 0),
    (25, 38, 8),
    (43, -5, 18),
    (63, 48, -6),
    (90, 18, 12),
  ),
  view: (x: 35.264deg, y: 45deg, z: 0deg),
  tolerance: 0.001,
  style: (
    stroke: rgb("#195b8a"),
    stroke_width: 0.4,
    show_control_points: true,
    show_control_polygon: true,
    show_control_labels: true,
    show_axes: true,
    axis_length: 22,
    control_point_fill: rgb("#d76839"),
    control_point_radius: 0.9,
  ),
)

#let circle-2d = (
  degree: 2,
  knots: (0, 0, 0, 1, 1, 1),
  control_points: ((70, 0), (70, 70), (0, 70)),
  weights: (1, calc.sqrt(2) / 2, 1),
  tolerance: 0.01,
  style: (
    stroke: rgb("#7250a4"),
    stroke_width: 0.5,
    show_control_points: true,
    show_control_polygon: true,
    control_point_fill: rgb("#d76839"),
    control_point_radius: 1,
  ),
)

#set page(margin: 18mm)
#set text(font: "Arial", size: 10pt)

= breplot

Typst 插件将 STEP 模型交给 Rust/WASM，生成带平滑曲面和 Typst 原生曲线边线的插图。

#figure(
  if quality == "cached" {
    step-view-data(read("target/preview-step.bin", encoding: none), width: 90%)
  } else {
    step-view(
      read("testdemo/demo.stp", encoding: none),
      width: 90%,
      view: step-camera,
    )
  },
  caption: [STEP 模型的正交视图。],
)

= NURBS 曲线

#grid(
  columns: (1fr, 1fr),
  gutter: 8mm,
  [
    *3D 非有理三次曲线*：投影后按节点区间转换为 Bézier。

    #nurbs-curve(cubic-3d, width: 100%)

    #text(size: 8pt)[
      完整节点向量：#raw("[" + cubic-3d.knots.map(str).join(", ") + "]")

      控制点（世界坐标 X, Y, Z）：
      #for i in range(cubic-3d.control_points.len()) [
        #raw("P" + str(i) + " = (" + cubic-3d.control_points.at(i).map(str).join(", ") + ")") \
      ]
    ]
  ],
  [
    *2D 有理二次曲线*：四分之一圆，按容差转换为三次 Bézier。

    #nurbs-curve(circle-2d, width: 100%)
  ],
)
