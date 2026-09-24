#import "@preview/cetz:0.5.2" as cetz
#import "package/lib.typ": nurbs, nurbs-curve, native-cubics, from-control-points, from-interpolation-points
#import "article/figures.typ": *
#set page(paper: "a4", margin: (x: 20mm, y: 18mm), numbering: "1", header: align(right, text(size: 8pt, fill: gray)[cetz-nurbs · 曲线、表示与绘制]))
#set text(font: ("Noto Serif SC", "Arial"), size: 10pt, lang: "zh")
#set par(justify: true, leading: 0.7em)
#set heading(numbering: "1.1")
#show heading.where(level: 1): set text(fill: rgb("#176caa"), size: 19pt)
#show raw: set text(font: ("Consolas", "Noto Sans SC"), size: 8pt)
#let note(body) = block(fill: rgb("#eff5f8"), inset: 9pt, width: 100%, radius: 3pt, body)
#let cap(body) = align(center, text(size: 9pt, fill: rgb("#666666"), body))

= Bézier 与 NURBS 曲线的数学表示
本文讨论 Bézier 曲线、B 样条及非均匀有理 B 样条（NURBS）的表示方法，推导齐次表示、节点细化与 Bézier 分段之间的关系，并说明其在 Typst/CeTZ 中的矢量绘制方法。文中区分参数曲线的等价变换与面向绘制的数值逼近；算例由随文程序生成。

== 表示层次与符号约定
#table(columns: (28mm, 1fr), inset: 7pt, stroke: 0.4pt + gray,
  [*参数表示*], [由次数、节点向量、控制点及权重确定参数函数。],
  [*等价变换*], [通过节点插入、升阶或细分改变表示，保持对应参数处的几何位置。],
  [*绘制表示*], [以非有理三次 Bézier 段输出矢量路径，必要时引入数值逼近。])

记控制点数为 $N$，曲线次数为 $p$，控制点序列为 $P_0, dots, P_(N-1)$。对单段 Bézier 曲线有 $N=p+1$；对 B 样条曲线，控制点数还取决于节点区间的数量及其重数。

== Bernstein 基与 Bézier 曲线
令参数 $t in [0,1]$，$p$ 次 Bézier 曲线由 $p+1$ 个控制点定义：
$ B(t) = sum_(i=0)^p b_(i,p)(t) P_i, quad b_(i,p)(t) = binom(p,i) (1-t)^(p-i) t^i. $
Bernstein 基函数在 $[0,1]$ 上非负且满足单位分解，因而 $B(t)$ 位于控制点的凸包内。曲线具有端点插值性：$B(0)=P_0$、$B(1)=P_p$；相应端部导数为 $p(P_1-P_0)$ 与 $p(P_p-P_(p-1))$。内部控制点一般不满足插值条件。

#let cubic = ((0,0),(1,3),(4,3),(5,0))
#align(center, plot(bezier-spec(cubic), labels: true, unit: 19mm))
#cap[三次 Bézier：虚线是控制多边形，蓝线是曲线。]

由单位分解可得 Bézier 表示的仿射不变性：对任意仿射映射 $A$，有 $A(B(t))=sum_(i=0)^p b_(i,p)(t)A(P_i)$。因此，平移、旋转、缩放及正交投影均可通过变换控制点实现。

#pagebreak()
= de Casteljau 算法与次数变换
给定参数 $t in [0,1]$，de Casteljau 算法通过递归线性插值计算曲线点：
$ P_i^((0)) = P_i, quad P_i^((r)) = (1-t)P_i^((r-1)) + t P_(i+1)^((r-1)). $
递推终值满足 $P_0^((p))=B(t)$。插值三角形的左边界与逆序右边界分别构成参数区间 $[0,t]$ 和 $[t,1]$ 上子曲线的控制点序列。

#let t = 0.4
#let parts = split(cubic, t)
#align(center, cetz.canvas(length: 20mm, {
  cetz.draw.line(..cubic, stroke: gray + 0.6pt)
  let row = cubic
  for r in range(1,4) {
    row = range(row.len()-1).map(i => mix(row.at(i),row.at(i+1),t))
    if row.len()>1 { cetz.draw.line(..row,stroke: palette.at(r)+0.8pt) }
    for q in row { dot(q, color: palette.at(r)) }
  }
  nurbs(bezier-spec(parts.at(0)), stroke: palette.at(0)+1.5pt)
  nurbs(bezier-spec(parts.at(1)), stroke: palette.at(1)+1.5pt)
  dot(evaluate(cubic,t),label: "t=0.4")
}))
#cap[一条三次曲线被精确分成两条三次曲线；插值网格给出新的控制点。]

两段以 $s in [0,1]$ 为局部参数，分别表示 $B(t s)$ 与 $B(t+(1-t)s)$。由链式法则，子段的局部导数包含相应区间长度因子。该细分保持几何位置及表示次数，每个子段仍由 $p+1$ 个控制点定义。

== 升阶及精确降阶的条件
利用 Bernstein 基的递推恒等式，可将 $p$ 次表示升为 $p+1$ 次表示。其中 $Q_0=P_0$、$Q_(p+1)=P_p$，对 $i=1,dots,p$ 有
$ Q_i = i/(p+1) P_(i-1) + (1-i/(p+1)) P_i. $
升阶前后的参数函数恒等。反之，精确降阶要求曲线的实际多项式次数低于其表示次数，即最高次幂的系数向量为零。升阶后控制点之间存在相应的线性约束，一般扰动不保持该约束。

#note[*性质.* 有限次细分保持各非退化子区间上的实际多项式次数；升阶仅改变表示次数。因此，一般高次多项式曲线不能通过有限次细分精确表示为分段三次曲线。]

例如，$C(t)=(t,t^4)$ 在任意非零长度区间上经仿射重参数化后，第二坐标的四次项系数仍非零，故不存在与之恒等的三次多项式表示。针对高次输入的分段三次逼近见第 7 节。

#pagebreak()
= 有理 Bézier 曲线与齐次表示
给定控制点 $P_i$ 及权重 $w_i$，有理 Bézier 曲线定义为
$ R(t) = frac(sum_(i=0)^p b_(i,p)(t) w_i P_i, sum_(i=0)^p b_(i,p)(t) w_i). $
该表达式一般为有理函数。若所有权重相等且非零，则退化为非有理 Bézier 表示；权重的整体非零比例变换不改变曲线。本文实现限定 $w_i>0$，从而保证参数域内分母为正。

#let tri = ((0,0),(2,3),(4,0))
#align(center, cetz.canvas(length: 23mm, {
  cetz.draw.line(..tri,stroke: gray+0.6pt)
  for (i,w) in (0.3,1,3).enumerate() {
    nurbs(bezier-spec(tri,weights:(1,w,1)),stroke:palette.at(i)+1.2pt)
    cetz.draw.content((4.5,2.6-i*0.6),text(size:9pt,fill:palette.at(i),"w1="+str(w)))
  }
  for q in tri { dot(q) }
}))
#cap[固定控制点、改变中间权重的二次有理 Bézier 曲线。]

定义有理基函数 $r_i(t)=b_(i,p)(t)w_i / (sum_j b_(j,p)(t)w_j)$。正权重条件下，$r_i(t)>=0$ 且 $sum_i r_i(t)=1$，故有理曲线同样具有凸包性与仿射不变性。

固定其他权重，对 $w_i$ 求偏导可得
$ frac(∂ R(t),∂ w_i)=frac(b_(i,p)(t),sum_j b_(j,p)(t)w_j)(P_i-R(t)). $
该式表明，在给定参数处，增大单个权重所引起的瞬时位移方向指向对应控制点。端部导数还包含相邻权重的比值，例如 $R'(0)=p frac(w_1,w_0)(P_1-P_0)$。

非有理与有理表示之间的关系可由中心投影建立。下一节先讨论三维有理曲线到归一化平面的投影，再以单位空间权重为特例导出通常的多项式齐次表示。

#pagebreak()
== 空间有理曲线与平面投影的等价表示
设三维有理 Bézier 曲线的控制点为 $V_i=(x_i,y_i,z_i)$，权重为 $a_i>0$，并假设 $z_i>0$。记 $D(t)=sum_i b_(i,p)(t)a_i$，则空间曲线为
$ V(t)=frac(sum_i b_(i,p)(t)a_i V_i,D(t)). $
以原点为投影中心，将曲线投影至归一化平面 $Pi: z=1$。投影映射为 $pi(x,y,z)=(x/z,y/z,1)$。令
$ P_i=(x_i/z_i,y_i/z_i,1), quad w_i=a_i z_i, $
直接代入并约去公共分母 $D(t)$，可得
$ pi(V(t))=frac(sum_i b_(i,p)(t)a_i z_i P_i,sum_i b_(i,p)(t)a_i z_i)=frac(sum_i b_(i,p)(t)w_i P_i,sum_i b_(i,p)(t)w_i). $
因此，空间曲线的中心投影等于平面内由控制点 $P_i$、权重 $w_i$ 定义的有理 Bézier 曲线；两者在相同参数处对应。

#let projective-controls = ((0,0,1.5),(1,3,3),(4,1,2))
#let projective-weights = (1,0.7,1.3)
#let plane-controls = projective-controls.map(v => (v.at(0)/v.at(2),v.at(1)/v.at(2),1))
#let plane-weights = projective-controls.enumerate().map(((i,v)) => projective-weights.at(i)*v.at(2))
#align(center,cetz.canvas(length:22mm,{
  cetz.draw.ortho(x:-65deg,y:0deg,z:-25deg,sorted:false,{
    cetz.draw.line((-0.3,-0.3,1),(2.4,-0.3,1),(2.4,1.4,1),(-0.3,1.4,1),close:true,stroke:gray+0.6pt)
    cetz.draw.content((2.4,1.4,1),[$Pi: z=1$],padding:4pt)
    for (i,v) in projective-controls.enumerate() {
      cetz.draw.line((0,0,0),v,stroke:(paint:rgb("#9a9a9a"),thickness:0.5pt,dash:"dashed"))
      dot(v,color:palette.at(1),label:"V"+str(i))
      dot(plane-controls.at(i),color:palette.at(0),label:"P"+str(i))
    }
    cetz.draw.line(..projective-controls,stroke:palette.at(1).lighten(35%)+0.6pt)
    cetz.draw.line(..plane-controls,stroke:palette.at(0).lighten(35%)+0.6pt)
    nurbs(bezier-spec(projective-controls,weights:projective-weights),stroke:palette.at(1)+1.5pt)
    nurbs(bezier-spec(plane-controls,weights:plane-weights),stroke:palette.at(0)+1.5pt)
    let h = projective-controls.enumerate().map(((i,v)) => (..v.map(x=>x*projective-weights.at(i)),projective-weights.at(i)))
    let q = evaluate(h,0.5)
    let v = (q.at(0)/q.at(3),q.at(1)/q.at(3),q.at(2)/q.at(3))
    let r = (v.at(0)/v.at(2),v.at(1)/v.at(2),1)
    cetz.draw.line((0,0,0),v,stroke:palette.at(2)+0.8pt)
    dot(v,color:palette.at(2),label:"V(0.5)")
    dot(r,color:palette.at(2),label:"R(0.5)")
    dot((0,0,0),color:black,label:"O")
  })
}))
#cap[橙色为空间有理 Bézier，蓝色为平面内的等价投影表示。绿色射线连接同一参数处的对应点；灰色射线连接对应控制点。]

本例取 $V_0=(0,0,1.5)$、$V_1=(1,3,3)$、$V_2=(4,1,2)$，空间权重为 $(1,0.7,1.3)$。投影控制点为 $(0,0,1)$、$(1/3,1,1)$、$(2,1/2,1)$，相应权重为 $(1.5,2.1,2.6)$。

在齐次几何中，非零向量的比例类表示同一点，$z=1$ 是其中一个归一化截面。特别地，若 $a_i=1$，空间曲线为非有理 Bézier，投影权重即为 $z_i$。反向构造取 $V_i=(w_i P_(i,x),w_i P_(i,y),w_i)$，即恢复二维有理曲线的多项式齐次提升。三维有理曲线本身的多项式齐次提升则位于四维空间。

== 有理二次圆弧
取 $P_0=(1,0)$、$P_1=(1,1)$、$P_2=(0,1)$，权重为 $(1,sqrt(2)/2,1)$。记齐次坐标为 $(X,Y,W)$，代入 Bernstein 展开式可验证 $X^2+Y^2=W^2$。因此，去齐次化曲线满足 $x^2+y^2=1$，其像为第一象限内的单位圆弧。
#align(center, plot(bezier-spec(((1,0),(1,1),(0,1)),weights:(1,calc.sqrt(2)/2,1)),unit:20mm))
#cap[输入是精确有理圆弧；PDF 中的普通三次显示是近似。]

#pagebreak()
= B 样条基函数与节点向量
设 $U=(u_0,dots,u_(N+p))$ 为非递减节点向量，其长度为 $N+p+1$，有效参数域为 $[u_p,u_N]$。节点划分基函数的多项式区间；同一节点值的出现次数称为节点重数。

零次基函数在 $[u_i,u_(i+1))$ 内为 1，其他位置为 0；高次按 Cox–de Boor 递推：
$ N_(i,p)(u) = frac(u-u_i,u_(i+p)-u_i) N_(i,p-1)(u) + frac(u_(i+p+1)-u,u_(i+p+1)-u_(i+1)) N_(i+1,p-1)(u). $
分母为零的项约定为零。非有理 B 样条定义为 $C(u)=sum_(i=0)^(N-1) N_(i,p)(u)P_i$；相应有理形式为 $R(u)=frac(sum_i N_(i,p)(u)w_i P_i,sum_i N_(i,p)(u)w_i)$。各基函数在非零节点区间内为次数不超过 $p$ 的多项式。

#let controls = ((0,0),(1,2),(2,-1),(3,2),(4,-1),(5,2),(6,0))
#let uniform = (control_points:controls,knots:(0,0,0,0,1,2,3,4,4,4,4))
#let nonuniform = (control_points:controls,knots:(0,0,0,0,0.3,1,3.5,4,4,4,4))
== 均匀与非均匀节点的比较
#grid(columns:(1fr,1fr),gutter:8mm,
  [*均匀内部间距*\ #plot(uniform,unit:11mm)\ #knot-strip(uniform.knots,unit:10mm)],
  [*非均匀内部间距*\ #plot(nonuniform,unit:11mm)\ #knot-strip(nonuniform.knots,unit:10mm)])
两例的端节点重数均为 $p+1=4$，故曲线两端夹紧并插值首尾控制点。均匀夹紧节点向量的非零区间等长，端部保留重复节点；非均匀情形则允许不同的区间长度。

#align(center,basis-plot(nonuniform.knots))
#cap[非均匀案例的七个三次基函数。改变节点也会改变控制点影响的参数范围。]

在控制点固定时，节点位置的变化一般改变曲线形状。保持曲线不变的节点插入须同步更新控制点。此外，均匀节点仅规定参数区间长度，通常不产生等弧长参数化。

#pagebreak()
= 重节点与连续性
对内部重数为 $r$ 的节点，若 $1<=r<=p$，则 $p$ 次 B 样条至少具有 $C^(p-r)$ 连续性；特殊控制点配置可提高实际连续阶数。三次曲线在简单、二重和三重节点处分别保证 $C^2$、$C^1$ 和 $C^0$ 连续；四重节点允许位置不连续。

#let repeat-case(r) = {
  let pts = ((0,0),(1,2),(2,-1),(3,2),(4,0),(5,2),(6,-1),(7,1)).slice(0,4+r)
  (control_points:pts,knots:(0,)*4+(1,)*r+(2,)*4)
}
#grid(columns:(1fr,1fr),gutter:8mm,
  [*三次 / 简单节点 / C²*\ #plot(repeat-case(1),unit:10mm,pieces:true)],
  [*三次 / 二重节点 / C¹*\ #plot(repeat-case(2),unit:10mm,pieces:true)],
  [*三次 / 三重节点 / C⁰*\ #plot(repeat-case(3),unit:10mm,pieces:true)],
  [*三次 / 四重节点 / 可断开*\ #plot(repeat-case(4),unit:10mm,pieces:true)])
#cap[每幅图由两个非零区间组成，分别着色。四幅图是独立的数据示例。]

各例采用节点向量 `(0,0,0,0, 1×r, 2,2,2,2)`，虚线表示控制多边形。四重节点处的左右子段分别绘制；零长度节点区间不参与子段提取。

== 节点细化与 Bézier 分解
对两端夹紧的连续 B 样条，将各内部节点的重数提高至 $p$，即可获得逐区间的 Bézier 表示。每个控制块包含 $p+1$ 个控制点，相邻块共享端点。对于非夹紧输入，需先提取有效参数域；已有 $p+1$ 重节点的左右子段独立处理。

单次插入节点 $v$ 时，受影响的新控制点是相邻旧控制点的仿射组合，系数包含 $(v-u_i)/(u_(i+p)-u_i)$；完整算法还需要按插入区间确定复制与更新的索引范围。有理曲线对齐次控制点执行同一算法。

#note[*性质.* Bézier 分解保持表示次数及参数函数。有理 B 样条的子段仍为有理 Bézier，非有理 B 样条的子段仍为非有理 Bézier。]

#pagebreak()
= 基于 Bernstein 基变换的子段提取
除节点插入外，Bézier 分解也可通过逐区间的多项式基变换实现。本文程序在每个非零节点区间内，将齐次 B 样条表示转换为 Bernstein 表示。

把区间 $[a,b]$ 写成 $u=a+(b-a)t$，待求齐次 Bézier 控制点为 $Q_j$。在 $p+1$ 个不同局部参数处计算原齐次曲线，再求解
$ H(a+(b-a)t_k)=sum_(j=0)^p b_(j,p)(t_k)Q_j, quad t_k=k/p. $
由多项式插值的唯一性，$p+1$ 个互异参数处的取值确定至多 $p$ 次多项式，因此该线性系统在精确算术下给出等价表示。浮点求解引入舍入误差。计算中固定当前节点区间，并分别取断点的左右极限。

#let two = (control_points:((0,0),(1,3),(2,-1),(3,3),(4,0)),knots:(0,0,0,0,1,2,2,2,2))
#align(center,plot(two,unit:25mm,pieces:true,local-controls:true))
#cap[简单内节点的一条三次 B 样条，精确提取成两段三次 Bézier；彩色虚线是各段控制多边形。]

#let segments = native-cubics(two)
#table(columns:(16mm,1fr),inset:6pt,stroke:0.4pt+gray,
 [*区间*],[*提取后的四个控制点（显示到两位小数）*],
 ..segments.enumerate().map(((i,c)) => ([#i → #(i+1)], [#c.map(p=>"("+p.map(x=>str(calc.round(x,digits:2))).join(", ")+")").join(" · ")])).flatten())

此例所有权重相同且次数为 3，所以提取结果直接成为普通三次绘图段。线段和二次非有理段则先精确升阶到三次。节点区间数与最终显示段数并不总相同：一般有理段和高次段还要继续细分、近似。

== 齐次基变换的适用性
上述唯一性论证适用于齐次多项式的各坐标分量。去齐次化后的有理函数一般不属于同次多项式空间，直接对其欧氏采样点进行多项式插值不具有等价性。因此，程序在子段提取阶段保留分子与分母，随后计算欧氏位置及导数。

#pagebreak()
= 面向矢量绘制的分段三次逼近
输出图元采用非有理三次 Bézier 表示。一次及二次非有理段可经升阶精确表示，三次非有理段可直接输出；其余输入采用分段三次逼近。

对局部参数域 $[0,1]$，由原曲线端点位置及一阶导数构造三次 Hermite 插值段，其 Bézier 控制点为
$ Q_0=C(0), quad Q_1=C(0)+frac(C'(0),3), quad Q_2=C(1)-frac(C'(1),3), quad Q_3=C(1). $
程序在 $t=1/8,1/4,1/2,3/4,7/8$ 处计算模型坐标中的位置误差。若任一采样误差超过容差，则对原齐次段进行中点细分，并递归构造三次插值段；超过细分深度或段数限制时返回错误。

#let highpoints = ((0,0),(1,5),(2,-4),(4,5),(5,-3),(6,0))
#let coarse = bezier-spec(highpoints,tolerance:0.2)
#let fine = bezier-spec(highpoints,tolerance:0.002)
#grid(columns:(1fr,1fr),gutter:8mm,
  [*五次 / 容差 0.2*\ #plot(coarse,unit:11mm,polygon:false,pieces:true)\ 显示段数：#native-cubics(coarse).len()],
  [*同一五次 / 容差 0.002*\ #plot(fine,unit:11mm,polygon:false,pieces:true)\ 显示段数：#native-cubics(fine).len()])
#cap[五次曲线在两组容差下的分段三次逼近；颜色表示不同输出子段。]

== 等价变换与逼近的适用范围
#table(columns:(1fr,1fr),inset:7pt,stroke:0.4pt+gray,
 [*操作*],[*结果*],
 [任意次数 Bézier 细分],[次数不变，曲线不变；有理情形用齐次坐标。],
 [B 样条插节点／提取子段],[次数不变，曲线不变；控制点和节点一起更新。],
 [低次曲线升阶],[几何不变，增加冗余表示。],
 [一般高次曲线的三次表示],[一般只能近似；原表示可约降时例外。],
 [有理圆弧转普通三次],[一般只能近似；保留有理二次则可精确。])

#note[*误差判据.* 容差约束有限采样点处的位置误差，尚未构成全参数域上的误差上界。其单位与模型坐标一致；页面缩放相应改变输出尺度。]

#pagebreak()
= 投影变换与绘制实现
去齐次化映射 $(w P,w) mapsto P$ 是有理表示中的中心投影。绘制阶段的相机正交投影则为欧氏空间到二维平面的仿射映射。前者涉及坐标比值，后者满足仿射不变性。

对任意仿射映射 $A$，归一化有理系数和为 1，因此
$ A(R(u))=frac(sum_i N_(i,p)(u)w_i A(P_i),sum_i N_(i,p)(u)w_i). $
因此，正交投影与曲线求值可交换次序；投影后的控制点保留原次数、节点向量及权重。透视投影应在齐次坐标中处理，其去齐次化通常改变权重。

#let spatial = (knots:(0,0,0,0,0.45,1,1,1,1),control_points:((0,0,0),(1,2,0.5),(2,-0.2,1),(3,2,-0.5),(4,1,1)))
#grid(columns:(1fr,1fr),gutter:8mm,
 [#align(center,cetz.canvas(length:19mm,{
   cetz.draw.ortho(x:25deg,y:40deg,z:10deg,sorted:false,{
     let h = ((0,0,1),(2,2,2),(2,0,1))
     cetz.draw.line((0,0,1),(2.3,0,1),(2.3,1.3,1),(0,1.3,1),close:true,stroke:gray+0.5pt)
     nurbs(bezier-spec(h),stroke:palette.at(1)+1.2pt)
     nurbs(bezier-spec(((0,0,1),(1,1,1),(2,0,1)),weights:(1,2,1)),stroke:palette.at(0)+1.2pt)
     for t in (0.25,0.5,0.75) {
       let q = evaluate(h,t)
       let r = (q.at(0)/q.at(2),q.at(1)/q.at(2),1)
       cetz.draw.line((0,0,0),r,q,stroke:(paint:gray,thickness:0.5pt,dash:"dashed"))
     }
     dot((0,0,0),label:"O")
     dot(evaluate(h,0.5),color:palette.at(1),label:"H(t)")
     cetz.draw.content((2.3,1.3,1),text(size:8pt,"W=1"),padding:3pt)
   })
 }))\ #cap[橙：齐次多项式曲线；蓝：投影到 W=1。虚线经过原点，是中心投影。]],
 [#align(center,cetz.canvas(length:19mm,{
  cetz.draw.ortho(x:25deg,y:40deg,z:10deg,sorted:false,{
    nurbs(spatial,control-points:true,control-polygon:true,stroke:palette.at(0)+1.2pt)
    for q in ((2,0,0),(0,2,0),(0,0,2)) { cetz.draw.line((0,0,0),q,stroke:gray+0.6pt) }
  })
}))\ #cap[三维欧氏控制点经相机正交投影。PDF 最终仍是二维矢量路径。]])

== 程序结构与接口
输入节点格式先归一化为完整向量，推导次数并校验。Rust/WASM 提取齐次子段，生成三次显示控制点；Typst 层把这些控制点交给 CeTZ 的 `bezier`。CeTZ 负责画布、坐标变换、线型、标注与原生图元组合。

`nurbs(spec)` 返回 CeTZ 绘图元素，供调用方组织画布及坐标变换；`nurbs-curve(spec)` 提供独立插图的排版封装。本包处理二维或三维参数曲线，不涉及 STEP 解析、曲面渲染及隐藏线计算。

```typ
#cetz.canvas({
  cetz.draw.ortho(x: 25deg, y: 40deg, z: 10deg, {
    nurbs(spatial, name: "c", control-points: true)
    // c.p0、c.p1 等是原始控制点的命名锚点。
  })
})
```

#pagebreak()
= 有理整圆与周期 B 样条算例

== 具有八个不同控制点位置的有理二次整圆

采用 Rhino 紧凑节点形式，控制点序列含 9 项，其中 $P_8=P_0$，故有 8 个不同的空间位置。由节点数与控制点数可得 $p=2$。内部节点 1、2、3 的重数均为 2，曲线由四段有理二次圆弧组成。

#let w = calc.sqrt(2) / 2
#let full-circle = (
  knot_format: "rhino",
  knots: (0, 0, 1, 1, 2, 2, 3, 3, 4, 4),
  control_points: ((2, 0), (2, 2), (0, 2), (-2, 2),
    (-2, 0), (-2, -2), (0, -2), (2, -2), (2, 0)),
  weights: (1, w, 1, w, 1, w, 1, w, 1), tolerance: 0.001,
)
#align(center, cetz.canvas(length: 12mm, {
  nurbs(full-circle, control-points: true, control-polygon: true,
    stroke: blue + 1pt)
  for (i, p) in full-circle.control_points.slice(0, 8).enumerate() {
    cetz.draw.content(p, text(size: 8pt, if i == 0 { "P0=P8" } else { "P" + str(i) }), anchor: "south-west", padding: 3pt)
  }
  for (p, label) in (((2, 0), "u=0/4"), ((0, 2), "u=1 (×2)"),
    ((-2, 0), "u=2 (×2)"), ((0, -2), "u=3 (×2)")) {
    cetz.draw.content(p.map(v => v * 0.6), text(size: 8pt, label))
  }
}))

#raw("knot_format: \"rhino\"\nknots: (0,0, 1,1, 2,2, 3,3, 4,4)\nweights: (1,w,1,w,1,w,1,w,1), w = sqrt(2)/2", block: true)

蓝色显示线按容差转为三次 Bézier；输入的有理二次曲线是整圆。

== 三次周期曲线：接缝处 C² 连续

取 7 个独立控制点，在序列末尾重复前 3 项，得到含 10 项的控制点序列。配合以下均匀节点向量，可得参数域为 $[0,7]$ 的三次周期 B 样条；周期接缝具有 $C^2$ 连续性。

#let periodic-loop = (
  knots: (-3, -2, -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10),
  control_points: ((2, 0), (2, 1.5), (0.4, 2.5), (-1.8, 1.5),
    (-2, -0.8), (-0.6, -2), (1.8, -1.5), (2, 0), (2, 1.5), (0.4, 2.5)),
  tolerance: 0.001,
)
#align(center, cetz.canvas(length: 12mm, {
  nurbs(periodic-loop, control-points: true, control-polygon: true,
    stroke: purple + 1pt)
  // Uniform cubic at u=0: (P0 + 4 P1 + P2) / 6.
  let seam = range(2).map(i => (periodic-loop.control_points.at(0).at(i)
    + 4 * periodic-loop.control_points.at(1).at(i)
    + periodic-loop.control_points.at(2).at(i)) / 6)
  cetz.draw.circle(seam, radius: 0.055, fill: black, stroke: none)
  cetz.draw.content(seam, text(size: 8pt, "接缝 u=0/7"), anchor: "north-east", padding: 4pt)
}))

#raw("// 10 explicit controls; P7=P0, P8=P1, P9=P2\n// Knots: (-3,-2,-1,0,1,2,3,4,5,6,7,8,9,10)\n// Domain: [0,7]", block: true)

完整节点形式默认 knot_format: "full"；次数 = 节点数 − 控制点数 − 1。
Rhino 形式次数 = 节点数 − 控制点数 + 1。

#pagebreak()
= 基于控制点与内插点的曲线构造

构造接口分别将输入点视为控制点或插值条件，返回节点向量、单位权重及控制点序列。默认次数为 3；`close` 指定位置闭合条件，`periodic` 指定周期条件。两者仅作用于构造接口。

#let samples = ((0, 0), (1, 1.8), (2.8, 2), (4, 0.8), (3, -1), (1, -1.2))
#let comparison(interpolate: false, close: false, periodic: false) = {
  let constructor = if interpolate { from-interpolation-points } else { from-control-points }
  let spec = constructor(samples, close: close, periodic: periodic)
  cetz.canvas(length: 10mm, {
    let polygon = samples
    if close or periodic { polygon.push(samples.first()) }
    cetz.draw.line(..polygon, stroke: (paint: gray.lighten(30%), dash: "dashed", thickness: 0.5pt))
    nurbs(spec, stroke: (paint: if interpolate { purple } else { blue }, thickness: 1pt))
    for p in samples { cetz.draw.circle(p, radius: 0.06, fill: orange, stroke: none) }
  })
}

#grid(columns: (1fr, 1fr), gutter: 8mm,
  [*控制点曲线*：橙点作为控制点。],
  [*内插点曲线*：曲线经过所有橙点。],
  [开放 / 均匀节点、两端夹紧\ #comparison()],
  [开放 / 相同节点规则\ #comparison(interpolate: true)],
  [`close: true` / 首尾位置闭合\ #comparison(close: true)],
  [`close: true` / 内插并闭合\ #comparison(interpolate: true, close: true)],
  [`periodic: true` / 三次周期 C² 接缝\ #comparison(periodic: true)],
  [`periodic: true` / 周期内插\ #comparison(interpolate: true, periodic: true)],
)

#raw("let a = from-control-points(points, degree: 3, close: true)\nlet b = from-interpolation-points(points, degree: 3, periodic: true)\nnurbs(a)\nnurbs(b)", block: true)

普通形式使用两端夹紧的均匀节点，内插参数取 Greville 位置；
周期形式使用均匀周期节点。`periodic: true` 已包含闭合含义。
仅 `close: true` 保证位置闭合，不保证接缝切向连续。



#pagebreak()
= 数据约定与插值系统
基础接口 `nurbs` 接收显式节点、权重和控制点，不修改其闭合或周期条件。节点格式默认为 `knot_format: "full"`。Rhino 紧凑形式省略完整向量两端各一个冗余节点。次数由数据长度确定；可选 `degree` 用于一致性校验。

#table(columns:(30mm,1fr),inset:6pt,stroke:0.4pt+gray,
 [*形式*],[*节点数量与次数*],
 [完整向量],[节点数 $M=N+p+1$，所以 $p=M-N-1$。],
 [Rhino 向量],[节点数 $M=N+p-1$，所以 $p=M-N+1$。])

== 配点方程与参数选择
给定待插值点 $Q_j$、节点向量及参数 $t_j$，控制点由下列配点方程确定：
$ sum_i N_(i,p)(t_j)P_i=Q_j. $
各坐标分量共享系数矩阵。非周期模式采用均匀夹紧节点，配点参数取 Greville 横坐标 $t_j=(u_(j+1)+dots+u_(j+p))/p$。该参数选择由节点向量确定，与依赖点间距离的弦长参数化不同。

周期模式使用循环基函数矩阵，求出独立控制点后在末尾重复前 $p$ 个控制点。实现中奇数次使用整数周期采样网格，偶数次使用半整数偏移，避免某些偶数点配置的奇异配点矩阵。输入点的顺序定义沿曲线的访问顺序；内插并不保证无自交。

`close: true` 在普通模式中使首尾点相同，仅保证位置闭合。`periodic: true` 使用周期构造并包含闭合含义；均匀简单节点的 $p$ 次周期曲线在接缝具有 $C^(p-1)$ 连续性。核心接口不会根据端点重合自行推断周期性。

== 实现约束
实现支持二维及三维有限坐标、1 至 8 次曲线和正权重，缺省权重为 1。插值系统最多含 512 行，并进行可解性与残差检查。曲线绘制使用第 7 节所述采样误差判据及递归限制。

== 参考资料与源码索引
#set text(size:8.5pt)
- #link("https://pages.mtu.edu/~shene/COURSES/cs3621/NOTES/spline/Bezier/bezier-elev.html")[Michigan Tech：Bézier 升阶]；#link("https://pages.mtu.edu/~shene/COURSES/cs3621/NOTES/spline/NURBS/NURBS-def.html")[NURBS 与齐次表示]。
- #link("https://web.mit.edu/hyperbook/Patrikalakis-Maekawa-Cho/node18.html")[MIT Hyperbook：B 样条的 Bézier 分段]。
- #link("https://developer.rhino3d.com/guides/opennurbs/nurbs-geometry-overview/")[Rhino：NURBS 数据约定]；#link("https://www.gnu.org/software/gsl/doc/html/bspline.html")[GSL：基函数、Greville 点与配点矩阵]。
- #link("https://typst.app/docs/reference/visualize/curve/")[Typst 原生曲线]；#link("https://cetz-package.github.io/docs/api/draw-functions/shapes/bezier/")[CeTZ Bézier 图元]。
- 源码：`rust/src/nurbs.rs`（表示与提取）、`display.rs`（显示近似）、`construct.rs`（点集构造）、`package/lib.typ`（CeTZ 接口）。
