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

// Interpolate explicit parameter samples with prescribed knots and optional
// endpoint derivatives dC/du. Weights are fixed inputs, not fitted unknowns.
#let interpolate-at-parameters(points,parameters,knots:(),weights:none,knot-format:"full",
  start-derivative:none,end-derivative:none)={
  assert(points.len() >= 2 and parameters.len()==points.len(),message:"One parameter per interpolation point is required")
  assert(parameters.windows(2).all(pair=>pair.first() < pair.last()),message:"Interpolation parameters must be strictly increasing")
  let dim=points.first().len()
  assert(dim in (2,3) and points.all(p=>p.len()==dim),message:"Expected consistent 2D or 3D points")
  let count=points.len()+int(start-derivative!=none)+int(end-derivative!=none)
  assert(count <= 64,message:"Explicit-knot interpolation supports at most 64 constraints")
  let template=normalize((control_points:range(count).map(_=>(0,0)),knots:knots,
    weights:if weights==none {range(count).map(_=>1)} else {weights},knot_format:knot-format))
  let (a,b)=domain(template)
  assert(parameters.first() >= a and parameters.last() <= b,message:"Interpolation parameter outside curve domain")
  let conditions=parameters.map(u=>(u,0))
  let rhs=points
  for (u,value) in ((a,start-derivative),(b,end-derivative)) {
    if value!=none {
      assert(value.len()==dim,message:"Endpoint derivative dimension must match points")
      conditions.push((u,1));rhs.push(value)
    }
  }
  let columns=range(count).map(j=>{
    let basis=template
    basis.control_points=range(count).map(i=>(if i==j {1} else {0},0))
    conditions.map(((u,order))=>if order==0 {evaluate(basis,u).first()} else {jet(basis,u).first.first()})
  })
  let original=range(count).map(i=>columns.map(c=>c.at(i)))
  let matrix=original
  let result=rhs
  // Scale rows before partial pivoting so changing the parameter domain does
  // not make derivative constraints artificially dominate point constraints.
  for i in range(count) {
    let scale=calc.max(..matrix.at(i).map(calc.abs))
    assert(scale > 0,message:"Singular interpolation constraints")
    matrix.at(i)=matrix.at(i).map(v=>v/scale)
    result.at(i)=mul(result.at(i),1/scale)
  }
  for col in range(count) {
    let pivot=col
    for row in range(col+1,count) {
      if calc.abs(matrix.at(row).at(col)) > calc.abs(matrix.at(pivot).at(col)) {pivot=row}
    }
    assert(calc.abs(matrix.at(pivot).at(col)) > 1e-12,message:"Singular interpolation constraints")
    let temp=matrix.at(col);matrix.at(col)=matrix.at(pivot);matrix.at(pivot)=temp
    temp=result.at(col);result.at(col)=result.at(pivot);result.at(pivot)=temp
    let divisor=matrix.at(col).at(col)
    matrix.at(col)=matrix.at(col).map(v=>v/divisor)
    result.at(col)=mul(result.at(col),1/divisor)
    for row in range(count) {
      if row!=col {
        let factor=matrix.at(row).at(col)
        matrix.at(row)=sub(matrix.at(row),mul(matrix.at(col),factor))
        result.at(row)=sub(result.at(row),mul(result.at(col),factor))
      }
    }
  }
  for (i,row) in original.enumerate() {
    let actual=range(dim).map(axis=>row.zip(result).map(((v,p))=>v*p.at(axis)).sum())
    assert(norm(sub(actual,rhs.at(i))) < 1e-8*calc.max(1,norm(rhs.at(i))),message:"Interpolation residual exceeds tolerance")
  }
  template.control_points=result
  template
}
