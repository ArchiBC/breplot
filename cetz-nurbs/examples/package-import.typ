#import "@preview/cetz:0.5.2" as cetz
#import "@local/cetz-nurbs:0.1.0": nurbs
#set page(width: auto, height: auto, margin: 10mm)
#cetz.canvas({
  nurbs((degree: 2, knots: (0, 0, 0, 1, 1, 1),
    control_points: ((0, 0), (1, 2), (3, 0))),
    stroke: blue + 1pt, control-points: true, control-polygon: true)
})
