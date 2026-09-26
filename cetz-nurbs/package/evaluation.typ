// Exact rational de Boor evaluation for annotations and analytic endpoint jets.
// No sampling of the display cubics; supports the same 2D/3D full/Rhino data.
#let add(a,b)=a.zip(b).map(((x,y))=>x+y)
#let mul(a,s)=a.map(x=>x*s)
#let sub(a,b)=add(a,mul(b,-1))
#let mix(a,b,t)=add(mul(a,1-t),mul(b,t))
#let norm(a)=calc.sqrt(a.map(x=>x*x).sum())
#let normalize(s)={
  let q=s
  let format=s.at("knot_format",default:"full")
  assert(format in ("full","rhino"),message:"Unknown knot format")
  let n=s.control_points.len()
  assert(n >= 2,message:"At least two control points are required")
  if format=="rhino" {q.knots=(s.knots.first(),)+s.knots+(s.knots.last(),)}
  let p=q.knots.len()-n - 1
  assert(p >= 1 and p <= 8 and n > p,message:"Invalid NURBS degree/control count: " + repr((p,n,q.knots,format)))
  assert(s.at("degree",default:p)==p,message:"Degree does not match knots")
  assert(q.knots.windows(2).all(pair=>pair.first() <= pair.last()),message:"Knots must be nondecreasing")
  assert(q.knots.at(p) < q.knots.at(n),message:"Empty NURBS domain")
  let dim=s.control_points.first().len()
  assert(dim in (2,3) and s.control_points.all(v=>v.len()==dim),message:"Expected consistent 2D or 3D points")
  q.insert("weights",s.at("weights",default:range(n).map(_=>1)))
  assert(q.weights.len()==n and q.weights.all(w=>w > 0),message:"Expected one positive weight per control point")
  q.insert("knot_format","full")
  q
}
#let degree(s)=s.knots.len()-s.control_points.len()-1
#let domain(s)=(s.knots.at(degree(s)),s.knots.at(s.control_points.len()))
#let homogeneous(s)=s.control_points.zip(s.weights).map(((p,w))=>mul(p,w)+(w,))
#let deboor(points,knots,p,u)={
  let n=points.len()
  let k=if u>=knots.at(n) {n - 1} else {
    range(p,n).filter(i=>knots.at(i) <=u and u < knots.at(i+1)).first()
  }
  let d=range(p+1).map(j=>points.at(k - p+j))
  for r in range(1,p+1) {
    for j in range(r,p+1).rev() {
      let i=k - p+j
      let a=(u - knots.at(i))/(knots.at(i+p+1 - r)-knots.at(i))
      d.at(j)=mix(d.at(j - 1),d.at(j),a)
    }
  }
  d.last()
}
#let evaluate(s,u)={
  let s=normalize(s)
  let (a,b)=domain(s)
  assert(u >= a and u <= b,message:"Parameter outside the active NURBS domain")
  let h=deboor(homogeneous(s),s.knots,degree(s),u)
  mul(h.slice(0,-1),1/h.last())
}
#let at(s,t)={let (a,b)=domain(s); evaluate(s,a+(b - a)*t)}
// Analytic rational derivatives, including one-sided derivatives at endpoints.
#let jet(s,u)={
  let s=normalize(s)
  let (a,b)=domain(s)
  assert(u >= a and u <= b,message:"Parameter outside the active NURBS domain")
  let p=degree(s); let knots=s.knots; let h=homogeneous(s)
  let values=(deboor(h,knots,p,u),)
  for order in range(1,3) {
    if p==0 {values.push(h.first().map(_=>0))} else {
      h=range(h.len()-1).map(i=>{
        let den=knots.at(i+p+1)-knots.at(i+1)
        if den==0 {h.at(i).map(_=>0)} else {mul(sub(h.at(i+1),h.at(i)),p/den)}
      })
      knots=knots.slice(1,-1); p -= 1
      values.push(deboor(h,knots,p,u))
    }
  }
  let (h0,h1,h2)=values
  let c=mul(h0.slice(0,-1),1/h0.last())
  let d=mul(sub(h1.slice(0,-1),mul(c,h1.last())),1/h0.last())
  let dd=mul(sub(sub(h2.slice(0,-1),mul(d,2*h1.last())),mul(c,h2.last())),1/h0.last())
  (point:c,first:d,second:dd)
}
#let curve-curvature(s,u)={
  let j=jet(s,u)
  let speed=norm(j.first)
  assert(speed > 0,message:"Curvature is undefined at a zero-speed point")
  let tangent=mul(j.first,1/speed)
  let parallel=j.second.zip(tangent).map(((x,y))=>x*y).sum()
  let vector=mul(sub(j.second,mul(tangent,parallel)),1/(speed*speed))
  (point:j.point,vector:vector,magnitude:norm(vector))
}
