// Teaching figures only. Production geometry stays in the plugin.
#import "@preview/cetz:0.5.2" as cetz
#import "../package/lib.typ": nurbs, native-cubics
#let palette = (rgb("#176caa"), rgb("#c96825"), rgb("#43885f"), rgb("#8856ac"), rgb("#bb465d"))
#let mix(a, b, t) = a.zip(b).map(((x, y)) => (1-t)*x + t*y)
#let evaluate(controls, t) = {
  let row = controls
  while row.len() > 1 { row = range(row.len()-1).map(i => mix(row.at(i), row.at(i+1), t)) }
  row.first()
}
#let split(controls, t) = {
  let row = controls
  let left = (row.first(),)
  let right = (row.last(),)
  while row.len() > 1 {
    row = range(row.len()-1).map(i => mix(row.at(i), row.at(i+1), t))
    left.push(row.first()); right.push(row.last())
  }
  (left, right.rev())
}
#let bezier-spec(points, weights: none, tolerance: 0.001) = (
  control_points: points, weights: weights,
  knots: (0,) * points.len() + (1,) * points.len(), tolerance: tolerance,
)
#let dot(p, color: rgb("#c96825"), label: none) = {
  cetz.draw.content(p, circle(radius: 1.5pt, fill: color, stroke: none), padding: 0pt)
  if label != none {
    cetz.draw.content(p, text(font: "Arial", size: 8pt, label), anchor: "south-west", padding: 3pt)
  }
}
#let colored(spec, controls: false) = {
  for (i, c) in native-cubics(spec).enumerate() {
    let color = palette.at(calc.rem(i, palette.len()))
    if controls {
      cetz.draw.line(..c, stroke: (paint: color.lighten(40%), thickness: 0.5pt, dash: "dashed"))
      for p in c { dot(p, color: color) }
    }
    cetz.draw.bezier(c.at(0), c.at(3), c.at(1), c.at(2), stroke: color + 1.3pt)
  }
}
#let plot(spec, unit: 13mm, polygon: true, labels: false, pieces: false, local-controls: false) = cetz.canvas(length: unit, {
  if polygon {
    cetz.draw.line(..spec.control_points, stroke: (paint: gray.lighten(25%), thickness: 0.55pt, dash: "dashed"))
    for (i, p) in spec.control_points.enumerate() { dot(p, label: if labels { "P" + str(i) }) }
  }
  if pieces { colored(spec, controls: local-controls) }
  else { nurbs(spec, stroke: palette.first() + 1.2pt) }
})
#let knot-strip(knots, unit: 12mm) = cetz.canvas(length: unit, {
  let lo = knots.first(); let hi = knots.last()
  cetz.draw.line((0, 0), (6, 0), stroke: gray + 0.6pt)
  let unique = ()
  for u in knots { if unique.len() == 0 or unique.last() != u { unique.push(u) } }
  for (i, u) in unique.enumerate() {
    let x = (u - lo)/(hi - lo)*6
    let mult = knots.filter(v => v == u).len()
    cetz.draw.line((x,-0.08),(x,0.08),stroke: black + 0.7pt)
    cetz.draw.content((x, if calc.rem(i,2)==0 {-0.23} else {-0.5}),
      text(font:"Arial",size:7.5pt,str(u) + if mult>1 {" x"+str(mult)} else {""}))
  }
})

// Cox-de Boor basis for the explanatory plot; not used to draw production curves.
#let basis(i,p,u,knots) = {
  if p == 0 { return if knots.at(i) <= u and u < knots.at(i+1) {1.0} else {0.0} }
  let a = knots.at(i+p)-knots.at(i)
  let b = knots.at(i+p+1)-knots.at(i+1)
  (if a==0 {0} else {(u - knots.at(i))/a*basis(i,p - 1,u,knots)}) + (if b==0 {0} else {(knots.at(i+p+1)-u)/b*basis(i+1,p - 1,u,knots)})
}
#let basis-plot(knots, p: 3) = cetz.canvas(length: 14mm, {
  let n = knots.len()-p - 1
  let a = knots.at(p); let b = knots.at(n)
  cetz.draw.line((0,0),(6.3,0),stroke: gray)
  cetz.draw.line((0,0),(0,1.8),stroke: gray)
  for i in range(n) {
    let points = range(81).map(j => {
      let u = a+(b - a)*j/80
      let value = if j==80 { if i==n - 1 {1} else {0} } else {basis(i,p,u,knots)}
      (j/80*6, value*1.5)
    })
    cetz.draw.line(..points,stroke: palette.at(calc.rem(i,palette.len()))+0.8pt)
  }
  cetz.draw.content((-0.15,1.5),text(size:8pt,"1"))
  cetz.draw.content((0,-0.2),text(size:8pt,str(a)))
  cetz.draw.content((6,-0.2),text(size:8pt,str(b)))
  cetz.draw.content((6.4,0),[$u$])
})
