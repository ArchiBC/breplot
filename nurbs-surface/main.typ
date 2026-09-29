#set document(title: "NURBS 曲面：从节点向量到可靠的数据结构", author: "breplot")
#set page(paper: "a4", margin: 20mm, numbering: "1")
#set text(font: ("Microsoft YaHei", "Arial"), size: 10pt, lang: "zh")
#set par(justify: true, leading: 0.65em)
#set heading(numbering: "1.")
#show raw: set text(size: 8pt, font: "Consolas")
#let snippet(path, name) = {
  let source = read(path)
  let start = "// doc:" + name + ":start"
  let end = "// doc:" + name + ":end"
  assert(source.split(start).len() == 2, message: "missing or duplicate source marker: " + start)
  assert(source.split(end).len() == 2, message: "missing or duplicate source marker: " + end)
  raw(source.split(start).at(1).split(end).at(0).trim(), lang: "rust", block: true)
}

#let output-mode = sys.inputs.at("surface-output", default: "hybrid")
#let surface-plugin = if output-mode == "pdf" { plugin("package/surface.wasm") }
// PDF presentation is independent of native geometry and SVG export.
#let surface-image(path, width: 100%) = {
  if output-mode == "pdf" {
    let id = path.replace("target/", "").replace(".svg", "")
    image(surface-plugin.surface_demo(bytes(id)), format: "pdf", width: width)
  } else if output-mode == "vector" {
    image(path, width: width)
  } else {
    let base = path.replace("target/", "target/display/").replace(".svg", "")
    box(width: width)[#image(base + ".png", width: 100%)#place(top + left, image(base + "-lines.svg", width: 100%))]
  }
}

#align(center)[
  #text(size: 21pt, weight: "bold")[NURBS 曲面数据结构]
  #v(4mm)
  从数据到参数求值与三角网格
]

= 本模块完成什么

本模块实现数据校验、参数求值、一阶偏导、法向与保留 UV 的三角网格，为后续遮挡提供基础。Rust crate 没有第三方依赖，也不依赖现有曲线内核。

文末展示实际求值、细分和原生 SVG 渐变着色结果，可选叠加网格；另支持多边形 UV 裁剪与结构线显示，尚未实现通用实体遮挡。数据校验不保证几何正则、无自交或周期连续；求值遇到奇异位置时返回空法向。

= 从曲线走向张量积曲面

沿 U 和 V 两个方向各定义一组 B 样条基函数。令控制点数量为 $n_u$、$n_v$，次数为 $p$、$q$，则曲面写成：

$ S(u,v) = (sum_(i=0)^(n_u - 1) sum_(j=0)^(n_v - 1)
  N_(i,p)(u) N_(j,q)(v) w_(i,j) P_(i,j)) /
  (sum_(i=0)^(n_u - 1) sum_(j=0)^(n_v - 1)
  N_(i,p)(u) N_(j,q)(v) w_(i,j)) $

控制点是三维欧氏坐标；权重单独保存。将控制点乘以权重得到齐次坐标属于后续算法，不在数据构造时进行，以免提前引入乘法溢出。

U/V 没有主次关系，也不必同次。下面展示一个 U 二次、V 一次的控制网格：

#table(columns: (1fr, 2fr, 2fr), inset: 7pt,
  [索引], [V = 0 列], [V = 1 列],
  [U = 0 行], [$P_(0,0)$], [$P_(0,1)$],
  [U = 1 行], [$P_(1,0)$], [$P_(1,1)$],
  [U = 2 行], [$P_(2,0)$], [$P_(2,1)$],
)

API 的二维数组顺序为 `control_points[u][v]`；内部下标为 `u * v_count + v`。这个约定对非方形网格尤其重要，测试使用 4×3 网格检查方向没有被交换。

#snippet("rust/src/surface.rs", "surface")

#pagebreak()
= 节点向量就是完整数组

以一个方向为例，令控制点数量为 $n$，完整节点数组长度为 $m$。次数由 $p = m - n - 1$ 推导。注意这里 $m$ 是数组长度，不是最后一个下标。

例如 `knots = [0,0,0,1,2,2,2]`，控制点数量为 4，次数为 2。重复值直接留在数组中，不额外存储节点重数。

#snippet("rust/src/axis.rs", "axis")
#snippet("rust/src/axis.rs", "degree")

Rhino 格式省略两端各一个外围节点，因此次数计算为 $p = m - n + 1$。构造器只补回两端各一个值，随即统一使用完整数组；不会增加、移除或重排控制点。对于非夹紧数据，无法恢复被省略的原始外围数值，保留的是有效域内所需的表示。

== 有效域与非零节点区间

零基数组的有效域为 `[knots[p], knots[n]]`。它不一定等于数组首尾值。例如完整数组 `[0,1,2,3,4,5,6,7,8]` 配合 5 个控制点表示三次方向，有效域是 `[3,5]`。

`spans()` 遍历下标 `p..n`，只返回相邻节点不相等的区间，同时保留原数组中的左端下标。示例 `[0,0,0,1,2,2,2]` 返回 `(2,[0,1])` 和 `(3,[1,2])`。这些原始参数和索引将用于后续子面提取。

== 构造时必须满足的不变量

- 次数在 1–25 之间，控制点数量至少为次数加一。25 是当前实现边界，不是数学上限。
- 节点有限、非递减，同一个值最多出现 `p+1` 次；允许内部出现 `p+1` 次，表示可能断开的分段，不承诺连续。
- 有效域长度为正且有限。不按容差合并相近节点，保持原始参数数据。
- 控制网格为矩形且与两个方向的控制点数量一致；权重网格同形。
- 坐标有限，权重有限且严格为正。省略权重时全为 1，不做自动归一化。

所有构造都返回 `Result<_, DataError>`。校验后的字段私有，访问器只返回只读借用或数值，避免外部修改破坏这些不变量。错误带字段路径，例如 `weights[0][1]`。

#pagebreak()
= 一段能运行的数据示例

下面给出四分之一圆柱的有理控制网格。U 方向采用有理二次圆弧的三个控制点，V 方向将其沿 Z 轴延伸。这里验证数据结构和属性；几何测试另以多个参数验证圆柱半径、切向和法向。

#snippet("rust/examples/data.rs", "example")

运行 `cargo run --manifest-path nurbs-surface/rust/Cargo.toml --example data`，可以执行上面的构造与断言。文章片段直接读取这份源码，源码标记缺失或重复会使文档编译失败。

== Bézier 是单跨特例

`NurbsSurface::bezier(points, weights)` 由网格大小推导两个次数，并分别生成两端重复“次数加一”次的 0、1 节点。4×4 网格得到双三次单跨面；3×2 网格得到二次×一次单跨面。支持省略权重，也支持有理权重。

`is_bezier()` 检查两个方向均夹紧且各有“次数加一”个控制点。一般单个有效区间的非夹紧表示不直接标记为 Bézier，因为尚未进行表示转换。

== 有理性与周期性

`is_u_rational()` 检查权重是否沿 U 方向变化，`is_v_rational()` 检查沿 V 的变化。若所有权重均为 7，公因子可以约去，仍判为非有理表示，但保存的数值不变。查询使用精确比较，不尝试特殊几何抵消分析。

周期数据需要调用方显式提供展开后的节点、控制点及权重。本模块不增加 `periodic` 或 `closed` 标志，不自动补点，不由首尾位置相同推断周期连续性。调用方可以用求值结果检查接缝，本模块不自动认证周期性。

#pagebreak()
= 代码导航、验证与后续边界

#table(columns: (1.3fr, 2fr), inset: 7pt,
  [文件], [职责],
  [`rust/src/axis.rs`], [完整节点向量、次数推导、有效域、非零区间],
  [`rust/src/surface.rs`], [控制网格与权重校验、只读查询、Bézier 构造],
  [`rust/src/lib.rs`], [公共 API 与字段路径错误],
  [`rust/src/tests.rs`], [有效样例、错误输入及方向性回归],
  [`rust/examples/data.rs`], [文章使用的可执行数据示例],
)

单元测试覆盖：非方形网格的 U/V 方向；full/Rhino 有效域；非夹紧数据；内部断开节点；零长域、非有限数、非法次数和节点重复；网格维度不匹配；非正权重；有理方向；显式展开数据；合法但退化的几何数据。

`scripts/build.ps1` 运行这些测试和示例，检查 WASM 目标编译，并编译本文 PDF 与 PNG。WASM 编译检查表示数据 crate 能用于该目标，不表示已经提供可被 Typst 调用的插件。

== 借鉴 OCCT 的范围

参考固定版本 OCCT V7_9_0 的 `Geom_BSplineSurface`：借鉴 U/V 分离、二维控制网格、权重形状检查，以及曲面几何与面拓扑分离的组织方式。

本实现直接存完整节点向量，不采用 OCCT 的独立节点与重数数组，也不采用紧凑周期存储。内部节点允许出现 `p+1` 次；不采用节点容差合并或权重最小几何阈值。因此本模块不是 OCCT 行为兼容层。

源码为独立实现，未复制或链接 OCCT。参考入口：

#link("https://github.com/Open-Cascade-SAS/OCCT/blob/V7_9_0/src/Geom/Geom_BSplineSurface.cxx")[OCCT V7_9_0 · Geom_BSplineSurface.cxx]

== 下一阶段接在哪里

当前求值器直接消费完整节点向量，网格直接采样原始曲面。后续可加入 Bézier 子面提取、局部自适应细分、法向角误差控制及遮挡区间计算。

裁剪环属于参数域上的 Face，共享边属于拓扑；相机、材质和显示容差属于渲染配置。它们均不混入当前曲面数据结构，后续可通过组合对象接入。

#pagebreak()
= 参数求值：原曲面上的点与切向

`evaluate(u,v)` 先查找两个参数所在的非零节点区间，再计算各自最多“次数加一”个非零基函数。`evaluate.rs` 的 `basis` 用局部 Cox–de Boor 递推，并用乘积法则同步传播一阶导数。它直接消费完整节点数组，没有重数表，也没有调用几何内核。

内部节点默认取右侧，参数域终点取左侧；超出有效域或非有限参数报错，不自动夹紧、外推或周期回绕。网格在节点区间边界使用指定区间的单侧求值，保留折痕与完全断开处的各自几何。

== 齐次结果如何还原

将加权三维分子记为 $A$，标量分母记为 $W$，得到 $S=A/W$。一阶导数由商法则给出：

$ S_u = (A_u - S W_u) / W, quad S_v = (A_v - S W_v) / W. $

`h[0]`、`h[1]`、`h[2]` 分别保存齐次值、U 偏导和 V 偏导；每项最后一个分量是权重项：

#snippet("rust/src/evaluate.rs", "quotient")

计算前对当前支撑区域的权重除以最大权重，减轻统一大权重带来的溢出；数据本身不变。极端动态范围仍可能导致分母下溢或导数溢出，此时明确报错，不返回 NaN。

#snippet("rust/src/evaluate.rs", "sample")

偏导针对原始参数。例如将 U 域从 `[0,1]` 拉伸为 `[2,6]`，相同位置的 U 偏导缩小为四分之一。法向方向为 $S_u times S_v$；先分别归一化切向，再叉乘，避免直接叉乘超大向量。零切向或归一化叉积长度不大于 `64 * f64::EPSILON` 时返回 `None`。这是数值奇异判据，不是模型长度容差。

验证使用平面的解析位置和偏导、有理圆柱的恒定半径与径向法向、非夹紧数据的线性再现，以及有限差分对照。原始节点参数缩放与全体权重缩放均有独立测试。

#pagebreak()
= 网格细分：从参数矩形到三角形

`mesh_uniform(n, max_vertices)` 对每个非零 U/V 节点区间组成的矩形，各划分为 $n times n$ 个小格。小格角点按参数正方向排列为 a、b、c、d，固定拆成 `(a,b,c)` 与 `(a,c,d)`，绕序与 $S_u times S_v$ 一致。

每个顶点保留 UV、原曲面位置、解析法向及所属节点区间。单个节点矩形内部共享顶点；不同矩形保留独立顶点，以保存单侧法向和断开边界。连续节点两侧位置应在浮点误差内一致，但索引不焊接。因此这是渲染网格，尚不是实体拓扑网格。

== 检查的是实际三角形误差

每个三角形检查重心及三条边的中点。先按重心坐标插值得到 UV，在原曲面求点，再与同样坐标插值的三角形位置比较，取三维距离最大值。双线性鞍面虽然可以由四角精确进行双线性插值，却不能被两个平面三角形精确表示，因此仍需细分；测试专门覆盖这种情况。

`tessellate(options)` 根据该误差整体加密，每轮将每个节点区间的细分数翻倍，直到达到容差：

#snippet("rust/src/mesh.rs", "refine")

这不是局部四叉树算法。相邻区间使用相同细分数，避免 T 接点；代价是平坦区域也随弯曲区域一起加密。`max_subdivisions` 和 `max_vertices` 限制工作量，预算不足时返回错误，不静默交付未达标结果。

采样误差不构成全局误差证明，可能漏掉高频或高度局部的变化。当前也没有法向夹角误差控制。精确零面积三角形不输出，并累计到 `degenerate_triangles`；奇异顶点的法向可为空。

== 可运行接口

#snippet("rust/examples/mesh.rs", "mesh-example")

`Mesh` 返回 `vertices`、`triangles`、`subdivisions`、`sampled_error` 与 `degenerate_triangles`。这些量可以继续供光栅器和线段遮挡计算使用，但网格生成本身不执行可见性判断；结构线显示另通过短弦与网格计算遮挡区间。

#pagebreak()
= 典型曲面：平面、柱面、球面与鞍面

四个案例全部先构造 NURBS 数据，再使用同一求值器与细分器。柱面覆盖整周且不含端盖；球面使用双二次有理张量积，覆盖整个单位球面；两者的接缝控制点显式重复。

#let cases = if output-mode == "pdf" { json(surface-plugin.surface_demo(bytes("stats"))) } else { json("target/meshes.json").map(c => (..c, triangle_count: c.triangles.len())) }
#let card(id, title) = block(width: 100%)[
  #align(center)[*#title*]
  #surface-image("target/" + id + ".svg", width: 100%)
  #let data = cases.find(c => c.id == id)
  #align(center, text(size: 8pt)[
    #data.triangle_count 个三角形，采样误差 #calc.round(data.sampled_error, digits: 5)
  ])
]
#grid(columns: (1fr, 1fr), gutter: 8mm,
  card("plane", "双线性平面"), card("cylinder", "完整柱面（无端盖）"),
  card("sphere", "完整有理球面"), card("saddle", "双线性鞍面"))

颜色由顶点解析法向计算，三角形内使用 SVG 线性渐变插值光照。没有依赖第三方绘图库或几何内核。默认 PDF 使用高清曲面 PNG 加矢量结构线/网格线，减轻阅读器负担；完整 SVG 仍保留。构建时可选择全矢量输出。先插值法向并归一化计算目标光照，再自适应细分着色小片，以 SVG 渐变逼近目标；不是逐像素渲染。

#pagebreak()
= 球面的表示与极点

U 方向采用四段有理二次圆；V 方向采用从南极到北极的两段有理二次子午线。控制点由子午线半径与圆的二维控制坐标相乘，权重为两个方向权重的乘积。完整节点数组直接保留重复值。

#snippet("rust/examples/support/mod.rs", "sphere")

球面测试覆盖单位半径、径向法向、整周接缝和两个极点。极点所有经线汇聚，解析参数切向退化，求值器保留 `normal: None`；SVG 展示层对这种顶点使用相邻三角形法向的面积加权平均，并合并几何上重合极点的贡献。这是展示近似，不回写或伪造原曲面的解析法向。

#grid(columns: (1fr, 1fr), gutter: 6mm,
  surface-image("target/sphere.svg", width: 100%),
  surface-image("target/sphere-mesh.svg", width: 100%))

左右图使用完全相同的顶点与三角形，右图仅打开网格叠加。零面积三角形仍由网格模块跳过并计数；接缝索引未焊接。

#pagebreak()
= 原生 SVG 如何实现光滑面片

`svg.rs` 自行实现正交投影、顶点法向光照和渐变输出。每个顶点使用环境光与 Lambert 漫反射：$I = 0.28 + 0.72 max(0, n dot l)$。纯漫反射时，三个颜色通道都与同一个标量强度成正比。加入高光后先计算最终 RGB，再拟合单层不透明颜色渐变。

`color_axis` 选取颜色距离最远的两个顶点，作为 RGB 轴；第三个颜色投影到此轴。`shade_patches` 把这种颜色拟合的误差也纳入细分判据，检查顶点与内部探针。每个着色小片只输出一个 RGB 渐变，不再输出白色高光透明遮罩。

在屏幕三角形内，Gouraud 光照是重心坐标插值，等价于仿射函数 $I(x,y)=a x+b y+c$。计算其梯度，再选择最小与最大顶点强度对应的两条等值线作为 SVG 线性渐变的起止位置：

#snippet("rust/src/svg.rs", "gradient")

每片使用 `gradientUnits="userSpaceOnUse"`，以浮点 RGB 百分比写入两个渐变色，并指定 sRGB 插值。单元测试将三个顶点投影回渐变轴，验证恢复其原始光照值；不会把每个三角形涂成一个平均颜色。

== 可控范围与后续工作

当前支持单色漫反射、白色高光与面片透明度，尚无阴影。`shade_patches` 在边中点、重心和三个内部点比较目标光照与渐变合成后的 RGB，超差则四分。`shading_tolerance` 默认 0.001（归一化颜色通道），独立于几何误差；最多递归 8 层、全图 250000 小片，超限报错。这是采样判据，不是全域误差证明。曲面轮廓仍取决于三角网格密度；光照插值的平滑不能替代几何细分。

预览按三角形平均深度从远到近排序。该方法适合当前展示样例，不是通用遮挡算法；交叉三角形、循环遮挡和复杂实体需要后续深度缓冲或几何裁剪。最终 RGB 直接填充着色小片路径，省去每片独立的矩形、裁切和分组。不透明且无网格描边时，裁切区域向外扩展 0.6 个 SVG 单位；使用单个凸六边形和斜切角限制细长三角形的扩展范围，避免多个路径的并集在 PDF 阅读器中出现拼接黑点。着色小片可扩展覆盖，但统一裁切在原三角形内；半透明时原三角形不扩展，透明度在父组只施加一次，避免内部细分重复叠色。原三角形之间仍可能存在抗锯齿接缝。

== Type 4 的内存调用实验

`pdf.rs` 按 ISO 32000-1 §8.7.4.5.5 编码顶点和 RGB，共享边用 flag 1/2 续接，网格流用原生 Flate 无损压缩；`render_scene` 复用投影、法向和结构线遮挡；Type 4 直接插值 RGB，按实际颜色误差细分，无需 SVG 的单轴颜色拟合。投影不重叠片批量绘制，前后面仍分别合成透明度。插件返回内存 PDF 字节，Typst 的 `image` 接收字节，不读写插图文件。以下为本文案例接口；通用 Rust 接口是 `render_pdf_with_lines(mesh, lines, options)`。

```typst
#let engine = plugin("package/surface.wasm")
#image(engine.surface_demo(bytes("sphere-structure")), format: "pdf")
```

用 `scripts/build-type4.ps1` 生成整份 `main-type4.pdf`。WASM 是编译产物；曲面 PDF 只存在内存中。最终文档再由 Python/PyMuPDF 将嵌入着色图形的透明组混合空间改为 DeviceRGB，保留透明度与网格数据，修复 Edge/Adobe 发黑；直接编译 Typst 不包含此兼容处理。该路径仍是实验：透明三角形接缝与部分阅读器的微小裂点尚待改进，默认文档继续使用混合输出。

构建脚本运行原有几何测试与独立案例测试，生成世界坐标 `meshes.json`、四种曲面的光滑 SVG 与网格叠加 SVG，再编译本文。新模块保持零第三方 Rust 依赖；已有曲线模块的 CeTZ 使用不受影响。

#pagebreak()
= 高光与透明度参数

`SvgOptions` 新增 `specular`（0–1，默认 0.25）、`shininess`（大于 0 且不超过 10000，默认 32）与 `opacity`（0–1，默认 1）。透明度的 0 表示完全透明，1 表示不透明；高光强度为 0 时关闭高光，指数越大高光越集中。参数非有限或越界时报错。

高光使用 Blinn–Phong 半程向量 $h=(l+v)/norm(l+v)$，顶点强度为 $s=k_s max(0,n dot h)^a$。背向光源或观察者时高光为零。在 Rust 中直接合成为最终颜色 $(1-s) C_d+s C_("white")$。这是一种有界白色混合模型，并非物理能量模型或加法高光。

#snippet("rust/examples/mesh.rs", "material")

#grid(columns: (1fr, 1fr, 1fr), gutter: 3mm,
  [#surface-image("target/sphere-matte.svg", width: 100%) #align(center)[无高光]],
  [#surface-image("target/sphere-glossy.svg", width: 100%) #align(center)[高光 0.8 / 指数 48]],
  [#surface-image("target/sphere-transparent.svg", width: 100%) #align(center)[相同高光 / 不透明度 0.35]])

每个着色小片先合成漫反射与高光的 RGB，父三角形统一裁切并施加一次 `opacity`，避免不同光照层的边缘覆盖不一致。可选网格线作为独立覆盖层绘制，使用同一透明度参数。不同深度的前后面分别参与叠加，因此穿过两层不透明度为 0.35 的表面，其合成不透明度约为 $1-(1-0.35)^2=0.5775$，不等于把整张不透明图像淡化为 0.35。

透明模式沿用三角形平均深度排序，没有折射、吸收厚度或通用的顺序无关透明。着色细分检查内部光照，减少粗网格产生的块状亮度变化；极窄高光仍可能躲过有限探针。几何网格决定法向插值场与轮廓，着色细分不能修复几何误差。

#pagebreak()
= UV 裁剪：外环、孔洞与真实边界

`TrimRegion::new(outer, holes)` 接收原始 UV 坐标的多边形外环和孔洞。环隐式闭合，也接受末点重复首点；方向自动统一。支持凹环与多个互不相交的孔洞，拒绝自交、环接触、孔洞嵌套、越出参数域及零面积输入。周期接缝需要调用方先展开，不自动跨缝解释。

当前输入是折线环，不是 NURBS 裁剪曲线。圆孔示例使用 32 边多边形近似，曲线到折线的误差不包含在曲面网格容差中。内部按外环包围盒归一化，二维判定容差为 `1e-12`，极小碎片可能被舍弃；尚非精确谓词的工业级布尔算法。

== 为什么不能只删除中心落在孔内的面片

小孔可能完全落在一个粗三角形内部，而中心点不在孔内。当前算法先用耳切法将外环、孔洞分别分解成三角形；将网格三角形与外环三角形求交，再依次减去孔洞三角形。减法以半平面切分并保留外侧碎片，最后三角化。

新增 UV 顶点重新在原始曲面求位置与解析法向，沿用来源节点区间的单侧求值。`tessellate_trimmed` 对裁剪后面片重新检查采样误差，并按预算整体加密。测试验证单个粗格内孔洞仍保留、凹外环与凹孔洞的 UV 面积守恒，而不是只检查非空结果。

#snippet("rust/examples/mesh.rs", "trim-example")

#pagebreak()
= 裁剪面与结构线的显示

#grid(columns: (1fr, 1fr), gutter: 5mm,
  [#surface-image("target/trimmed-saddle.svg", width: 100%) #align(center)[带孔鞍面与 U/V 结构线]],
  [#surface-image("target/trimmed-sphere.svg", width: 100%) #align(center)[带孔球面与裁剪边界]])

`structure_lines(region, u_values, v_values, samples_per_span)` 接收明确的等参值，保持 U 不变生成 IsoU，保持 V 不变生成 IsoV。与多边形边界求交后，在保留区间内采样原曲面；按节点分跨，避免跨过完全断开的节点连接两侧。

#snippet("rust/src/lines.rs", "lines")

结构线不是三角网格边。裁剪边界单独标记为 TrimBoundary 并加粗。当前线条是按节点区间均匀采样的三维短弦，采样数控制密度，不提供全局曲线精度保证；参数域的直线裁剪边映射到曲面后通常也是曲线。

#pagebreak()
= 结构线遮挡：只绘制可见参数区间

#align(center, surface-image("target/sphere-structure.svg", width: 65%))

不透明展示先绘制曲面，再对每条结构线短弦执行线—投影三角形检测：二维半平面裁剪得到重叠区间，三角形深度与线段深度沿区间均为一次函数，求出前方三角形覆盖的区间，合并后绘制补集。包围盒先过滤明显不重叠的三角形。

该算法针对离散网格与短弦，不是原始曲面的解析隐藏线算法。深度偏置采用两倍网格采样误差加投影范围的 `1e-9`，减轻附着曲线的自遮挡；小于偏置的近距遮挡仍可能误判。当前逐段扫描三角形，尚未引入 BVH 加速。

半透明模式有意显示全部结构线，便于观察前后结构，不将半透明面当作不透明遮挡物。裁剪仍在 UV 域发生，因此任何模式都不会把等参线画过孔洞。

本轮验证包括：孔洞与凹域面积、环方向一致性、非法环与预算错误、等参线在孔边截断、节点断开处不连线，以及无遮挡、前后深度、局部遮挡和深度交叉案例。裁剪输出仍是独立面片网格，尚未焊接为共享边的实体拓扑。
