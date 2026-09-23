#import "@local/breplot:0.1.0": nurbs-curve

#set page(margin: 18mm)
#set text(font: "Arial", size: 10pt)

= 本地 Typst 包导入

通过 `typst.toml` 的 `lib.typ` 入口加载 WASM，并渲染有理 NURBS 曲线。

#let arc = (
  degree: 2,
  knots: (0, 0, 0, 1, 1, 1),
  control_points: ((70, 0), (70, 70), (0, 70)),
  weights: (1, calc.sqrt(2) / 2, 1),
  tolerance: 0.01,
)
#nurbs-curve(arc, width: 65%)
