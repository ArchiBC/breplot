#import "../package/lib.typ": step-view, step-view-data
#let camera = json("step-preview.json").view

#set page(margin: 18mm)
#set text(font: "Arial", size: 10pt)

= breplot STEP 演示

同一 STEP 模型由 Typst 在编译时传给 Rust/WASM；平滑曲面与 Typst 原生 Bézier 边线共同组成插图。

#figure(
  if sys.inputs.at("quality", default: "final") == "cached" {
    step-view-data(read("../target/preview-step.bin", encoding: none), width: 90%)
  } else {
    step-view(
      read("../testdemo/demo.stp", encoding: none),
      width: 90%,
      view: (x: camera.at(0) * 1deg, y: camera.at(1) * 1deg, z: camera.at(2) * 1deg),
    )
  },
  caption: [Rhino STEP 模型的正交视图。],
)
