#import "../package/lib.typ": nurbs-curve

#set page(margin: 18mm)
#set text(font: "Arial", size: 10pt)

= NURBS 曲线模块

三维非有理三次 NURBS：先投影控制点，再按节点区间精确转换成三次 Bézier。

#let cubic-3d = (
  degree: 3,
  knots: (0, 0, 0, 0, 0.45, 1, 1, 1, 1),
  control_points: ((0, 0, 0), (25, 38, 8), (43, -5, 18),
    (63, 48, -6), (90, 18, 12)),
  view: (x: 35.264deg, y: 45deg, z: 0deg),
  style: (show_control_points: true, show_control_polygon: true,
    show_control_labels: true, show_axes: true),
)

#nurbs-curve(cubic-3d, width: 70%)

二维有理二次 NURBS：四分之一圆，按容差转换为 Typst 原生三次 Bézier。

#let circle-2d = (
  degree: 2,
  knots: (0, 0, 0, 1, 1, 1),
  control_points: ((70, 0), (70, 70), (0, 70)),
  weights: (1, calc.sqrt(2) / 2, 1),
  tolerance: 0.01,
  style: (show_control_points: true, show_control_polygon: true),
)

#nurbs-curve(circle-2d, width: 43%)
