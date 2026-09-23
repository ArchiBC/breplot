#import "../package/lib.typ": step-view

#set page(margin: 18mm)
#set text(font: "Arial", size: 10pt)

= 原生平涂曲面对照

平涂模式以 Typst 原生三角形绘制曲面；此例降低网格密度，仅用于比较渲染方式。

#figure(
  step-view(
    read("../testdemo/demo.stp", encoding: none),
    width: 80%,
    view: (x: 35.264deg, y: 45deg, z: 0deg),
    visibility-deflection: 0.05,
    surface-deflection: 0.2,
    max-segment-length: 2,
    surface-mode: "flat",
  ),
  caption: [原生三角面片与原生 Bézier 边线。],
)
