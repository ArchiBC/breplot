#import "../package/lib.typ": step-view, step-view-data
#let camera = json("step-preview.json").view

#set page(margin: 18mm)
#set text(font: "Arial", size: 10pt)

= breplot 隐藏线演示

灰色虚线标出按正交视角被曲面遮挡的 B-Rep 边段。

#figure(
  if sys.inputs.at("quality", default: "final") == "cached" {
    step-view-data(read("../target/preview-step.bin", encoding: none),
      width: 90%, hidden: true)
  } else {
    step-view(
      read("../testdemo/demo.stp", encoding: none),
      width: 90%,
      view: (x: camera.at(0) * 1deg, y: camera.at(1) * 1deg, z: camera.at(2) * 1deg),
      hidden: true,
    )
  },
  caption: [同一视角的可见边与隐藏边。],
)
