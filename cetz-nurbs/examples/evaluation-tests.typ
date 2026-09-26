#import "../package/lib.typ": *
#let near(a,b,eps:0.00000001)=assert(calc.abs(a - b) < eps,message:"Unexpected numeric result")
#let vector-near(a,b)={assert(a.len()==b.len());for (x,y) in a.zip(b) {near(x,y)}}
#let line=(control_points:((1,2,3),(4,6,3)),knots:(0,0,1,1))
#vector-near(evaluate-point(line,0.25),(1.75,3,3))
#vector-near(evaluate-derivatives(line,0).first,(3,4,0))
#vector-near(evaluate-derivatives(line,1).second,(0,0,0))
#near(curve-curvature(line,0.5).magnitude,0)
#let arc=(control_points:((1,0),(1,1),(0,1)),knots:(0,0,0,1,1,1),weights:(1,calc.sqrt(0.5),1))
#vector-near(evaluate-point(arc,0.5),(calc.sqrt(0.5),calc.sqrt(0.5)))
#for u in (0,0.2,0.5,0.8,1) {
  let p=evaluate-point(arc,u)
  near(p.at(0)*p.at(0)+p.at(1)*p.at(1),1)
  near(curve-curvature(arc,u).magnitude,1)
  vector-near(curve-curvature(arc,u).vector,p.map(x=>-x))
}
#let rhino=arc
#{rhino.knots=arc.knots.slice(1,-1)}
#rhino.insert("knot_format","rhino")
#vector-near(evaluate-point(rhino,0.5),evaluate-point(arc,0.5))
#assert(curve-domain(rhino)==(0,1))
#assert(curve-degree(rhino)==2)
#let scaled=arc
#{scaled.knots=arc.knots.map(u=>2+5*u)}
#vector-near(evaluate-point(scaled,4.5),evaluate-point(arc,0.5))
#vector-near(evaluate-derivatives(scaled,4.5).first,evaluate-derivatives(arc,0.5).first.map(x=>x/5))
#vector-near(evaluate-derivatives(scaled,4.5).second,evaluate-derivatives(arc,0.5).second.map(x=>x/25))
#near(curve-curvature(scaled,4.5).magnitude,1)
#let arc3=arc
#{arc3.control_points=arc.control_points.map(p=>(p.at(0),0,p.at(1)))}
#near(curve-curvature(arc3,0.5).magnitude,1)
#vector-near(curve-curvature(arc3,0.5).vector,(-calc.sqrt(0.5),0,-calc.sqrt(0.5)))
#for degree in (2,3,5,8) {
  let s=from-control-points(range(10).map(i=>(calc.cos(i*36deg),calc.sin(i*36deg))),degree:degree,periodic:true)
  let (a,b)=curve-domain(s)
  let start=evaluate-derivatives(s,a);let end=evaluate-derivatives(s,b)
  vector-near(start.point,end.point)
  vector-near(start.first,end.first)
  if degree > 2 {vector-near(start.second,end.second)}
}
// Interior repeated knots use the right-hand jet; the final endpoint uses left.
#let corner=(control_points:((0,0),(1,0),(1,1)),knots:(0,0,1,2,2))
#vector-near(evaluate-derivatives(corner,1).first,(0,1))
#vector-near(evaluate-derivatives(corner,2).first,(0,1))
#let broken=(control_points:((0,0),(1,0),(3,1),(3,2)),knots:(0,0,1,1,2,2))
#vector-near(evaluate-point(broken,1),(3,1))
#vector-near(evaluate-derivatives(broken,1).first,(0,1))
Evaluation and derivative checks passed.

#import "../package/lib.typ": interpolate-at-parameters
// Recover rational 3D control points from parameters and endpoint derivatives.
#let source=(control_points:((0,0,0),(1,2,1),(3,-1,2),(4,0,3)),knots:(0,0,0,0,1,1,1,1),weights:(1,0.8,1.4,1))
#let recovered=interpolate-at-parameters((evaluate-point(source,0),evaluate-point(source,1)),(0,1),
  knots:source.knots,weights:source.weights,
  start-derivative:evaluate-derivatives(source,0).first,end-derivative:evaluate-derivatives(source,1).first)
#for (a,b) in source.control_points.zip(recovered.control_points) {vector-near(a,b)}
#for i in range(21) {vector-near(evaluate-point(source,i/20),evaluate-point(recovered,i/20))}
// Arbitrary knot spacing and a Rhino-format knot vector.
#let points=((0,0),(1,2),(3,1),(4,0))
#let interpolated=interpolate-at-parameters(points,(0,0.2,0.8,1),knots:(0,0,0,1,1,1),knot-format:"rhino")
#for (p,u) in points.zip((0,0.2,0.8,1)) {vector-near(evaluate-point(interpolated,u),p)}

// Endpoint dC/du scales inversely when the parameter interval is rescaled.
#let stretch=1000000
#let stretched=interpolate-at-parameters((evaluate-point(source,0),evaluate-point(source,1)),(0,stretch),
  knots:source.knots.map(u=>u*stretch),weights:source.weights,
  start-derivative:evaluate-derivatives(source,0).first.map(v=>v/stretch),
  end-derivative:evaluate-derivatives(source,1).first.map(v=>v/stretch))
#for i in range(21) {vector-near(evaluate-point(source,i/20),evaluate-point(stretched,stretch*i/20))}
