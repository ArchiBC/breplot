# cetz-nurbs

独立的 Typst / CeTZ NURBS 扩展。此目录可单独复制、构建和使用，不需要 breplot、STEP 样本或光栅器。

`main.typ` 是完整中文图解文章，涵盖 Bézier 与 de Casteljau、升阶、有理与齐次投影、均匀／非均匀／重节点、B 样条的等价 Bézier 分段、实际三次显示算法、Rhino 整圆和周期曲线，以及控制点／内插点两个生成器。构建后阅读 `main.pdf`；教学插图辅助代码位于 `article/figures.typ`。

## 构建

需要 Rust 1.96.0、`wasm32-unknown-unknown`、Typst 0.14 或更新版本。CeTZ 固定为 `@preview/cetz:0.5.2`，首次编译需要下载或已有包缓存。

```powershell
.\scripts\build.ps1
```

脚本运行几何测试、构建 `package/cetz_nurbs.wasm`，编译 `main.typ` 为 `main.pdf` 与 `target/main-{p}.png`，并将包复制到 `target/typst-packages/local/cetz-nurbs/0.1.0` 验证清单导入。分发本地包时复制 `package/` 内的 `typst.toml`、`lib.typ`、`evaluation.typ` 和构建后的 `cetz_nurbs.wasm`。

## CeTZ 绘图接口

```typst
#import "@preview/cetz:0.5.2" as cetz
#import "@local/cetz-nurbs:0.1.0": nurbs
#let spline = (
  knots: (0, 0, 0, 1, 1, 1),
  control_points: ((0, 0, 0), (1, 2, 1), (3, 0, 2)),
  weights: (1, 0.8, 1),
  tolerance: 0.001,
)
#cetz.canvas({
  cetz.draw.ortho(sorted: false, {
    nurbs(spline, name: "spline", stroke: blue + 1pt,
      control-points: true, control-polygon: true)
    cetz.draw.line((0, 0, 0), (3, 0, 0), stroke: gray)
  })
})
```

`nurbs` 返回 CeTZ 绘图元素，不创建画布、不自动缩放或应用 `spec.view`。2D 可直接绘制；3D 使用调用方的 `draw.ortho`，平移、缩放、分组和其他图元共用同一变换。`stroke` 等剩余命名参数交给 CeTZ 路径样式，省略时继承画布样式。`name` 为分组名，提供 `spline.p0`、`spline.p1` 等控制点锚点和 `spline.curve` 路径。

辅助参数包括 `control-points`、`control-polygon`、`control-labels`、`point-radius`（画布单位）、`point-fill`、`polygon-stroke`。绘图最终是原生矢量路径；CeTZ 的正交投影不提供实体遮挡判断。

`nurbs-curve(spec, width: 100%)` 是独立插图包装器，保留原 breplot 的字典、`view` 和 `style` 接口。它自行创建 CeTZ 画布，3D 默认等轴测视角，2D 默认零旋转。旧调用只需将导入改为本包。`style` 保留 `stroke`、`stroke_width`、`opacity`、`dash`、`show_control_points`、`show_control_polygon`、`show_control_labels`、`show_axes`、`axis_length`、`control_polygon_stroke`、`control_point_fill`、`control_point_radius`；长度使用模型单位，颜色支持 Typst 颜色或十六进制字符串。

`native-cubics(spec)` 返回世界坐标三次 Bézier 控制点数组，供自定义渲染使用。

## 几何与依赖边界

- 支持 1–8 次曲线，控制点为数值 `(x, y)` 或 `(x, y, z)`，默认使用完整重复节点向量，也可用 `knot_format: "rhino"`。支持非夹持节点与不连续节点区间；权重必须为正，省略时全为 1。
- 非有理 1–3 次段按节点区间精确转三次 Bézier（浮点计算误差除外）。一般有理段和更高次段使用自适应三次近似，`tolerance` 在世界坐标中检查多个采样点，不是严格全局误差界。CeTZ 后续缩放会同时缩放误差。
- Rust 内核只依赖 `brepkit-math`、Serde 和 WASM 协议，不依赖 breplot、STEP、拓扑、HLR 或 PNG。breplot 以禁用 `plugin` feature 的方式复用同一内核。
- 固定的 `brepkit-math 3.4.18` 清单标注 `AGPL-3.0-only`，CeTZ 0.5.2 标注 `LGPL-3.0-or-later`。本次为本地拆分验证，未发布；公开分发时仍需确定项目自身许可证和依赖许可方案。

演示入口为 `main.typ`（`examples/demo.typ` 保留转发入口）；包含有理圆弧、三维旋转、画布样式继承、控制点锚点、重复节点导致的断开曲线、8 个独立控制点整圆和三次周期曲线。

## 基础数据与节点格式

基础 `nurbs` / `nurbs-curve` / `native-cubics` 接口的几何数据只有 `knots`、`control_points`、`weights`（省略时全为 1）。不提供 `close`、`closed`、`periodic` 开关，不添加或删除控制点。闭合及周期性由调用者的数据本身表达。

`knot_format` 选择节点存储约定，默认 `"full"`。设节点数为 m、控制点数为 n：

| 格式 | 推导次数 p | 转换 |
| --- | --- | --- |
| `"full"` | m - n - 1 | 直接使用完整重复节点向量 |
| `"rhino"` | m - n + 1 | 内部补回两端各一个冗余节点 |

两种形式都不需要 `degree`。旧调用仍可提供 `degree`，但必须与推导结果一致，否则报错。Rhino 格式不是唯一节点加重数的压缩形式。对于非夹持和周期数据，两端冗余节点的补齐也不会修改有效参数域内的曲线。

`main.typ` 的整圆采用 Rhino 格式，节点 `(0,0,1,1,2,2,3,3,4,4)`；显式给出 9 个控制点和 9 个权重，P8=P0，因此图中只有 8 个不同位置。三次周期示例显式给出 10 个控制点（末 3 项重复前 3 项）和 14 项完整节点，不使用自动展开开关。

## 指定节点与参数的插值

`interpolate-at-parameters` 适合教材中的节点对比图：输入曲线应经过的点及其参数值，
指定完整或 Rhino 格式的节点，可选固定权重和两端的一阶导数，求解控制点。
它使用原曲线的有理基函数，不从显示用的三次路径反推几何。

```typst
#import "@local/cetz-nurbs:0.1.0": interpolate-at-parameters, nurbs
#let curve = interpolate-at-parameters(
  ((0, 0), (1, 2), (3, 1), (4, 0)), (0, 0.2, 0.8, 1),
  knots: (0, 0, 0, 0, 1, 1, 1, 1),
)
// 在 cetz.canvas 中绘制：nurbs(curve)
```

- 支持 2D/3D；每个点必须给一个严格递增且位于有效区间内的参数。
- `start-derivative` / `end-derivative` 默认为 `none`；向量表示有效区间端点处的 **dC/du**，包含大小，不能只给单位切向。
- 控制点数等于点约束数加上导数约束数。节点数量据此推导次数（支持 1–8）；完整节点长度为控制点数 + 次数 + 1。
- `weights` 默认为全 1，可提供每个待求控制点的正权重；固定权重，求解控制点位置。
- `knot-format: "rhino"` 接受省略两端各一个节点的格式，默认 `"full"`；返回完整节点数据，可直接用于绘图和求导。
- 当前最多 64 个约束，使用行归一化和主元消元；拒绝奇异系统并检查插值残差。
- 不自动推测参数、不保证无振荡。重建位图中的曲线仍是近似；之后的曲率计算针对重建出的曲线。

## 两个点集扩展

```typst
#import "@local/cetz-nurbs:0.1.0": nurbs, from-control-points, from-interpolation-points
#let a = from-control-points(points, degree: 3, close: true)
#let b = from-interpolation-points(points, degree: 3, periodic: true)
// 在 cetz.canvas 中绘制：nurbs(a); nurbs(b)
```

两个函数返回只有 `knots`、`control_points`、`weights` 的标准完整节点数据，权重全为 1；可以交给 CeTZ 绘图函数，也可以交给 `nurbs-curve` 独立排版。

- `from-control-points` 将输入点作为控制点。普通形式采用均匀内节点，两端各重复 p+1 次。
- `from-interpolation-points` 求解控制点，曲线经过所有输入点。为保持均匀节点，开放/闭合形式采用 Greville 参数（每个基函数对应的 p 个节点的平均值）；周期形式采用循环 Greville 网格。它不是弦长参数化，也不自动消除振荡或自交。当前稠密求解最多支持 512 个内插点（包括仅闭合时补回的首点）。
- 两者均支持 2D/3D，同一次调用中的点维度必须一致；次数默认 3，支持 1–8，至少提供 p+1 个点。
- `close: true` 在需要时补回首点，使用夹紧节点，仅保证位置闭合。输入已经重复首点时不再追加。
- `periodic: true` 构造均匀周期节点并循环展开前 p 个控制点/权重；内插版本先求解周期控制点。它已包含闭合含义，优先于 `close`。允许输入末尾重复一次首点，会先去掉这个重复项。默认三次曲线的接缝为 C² 连续。

这些构造选项仅属于点集扩展，不能传给基础 NURBS 数据接口。`main.typ` 第三页并排展示控制点曲线和内插曲线的开放、闭合、周期形式。

周期验证使用齐次 Bézier 端点的解析位置、一阶和二阶导数。`brepkit-math 3.4.18` 的 `derivatives(u,2)` 在非夹持样本返回非有限值，绘图和此项验证不调用该方法。一般有理/高次曲线的三次显示仍是近似，不保证保留输入曲线所有高阶连续性。

在插件目录生成 PDF（构建脚本也会自动生成）：

```powershell
typst compile --root . main.typ main.pdf
```

约定参考：[Rhino 整圆示例](https://developer.rhino3d.com/en/samples/rhinocommon/add-nurbs-circle/)、[Rhino 节点存储说明](https://developer.rhino3d.com/guides/opennurbs/nurbs-geometry-overview/)、[GSL Greville 参数说明](https://www.gnu.org/software/gsl/doc/html/bspline.html)。

## 求点、导数与曲率

`evaluate-point(spec, u)` 返回原参数域中的二维或三维点。
`evaluate-derivatives(spec, u)` 返回 `(point:, first:, second:)`，导数相对于原始参数 `u`。
`curve-domain(spec)` 和 `curve-degree(spec)` 返回有效区间与次数。
`curve-curvature(spec, u)` 返回 `(point:, vector:, magnitude:)`；vector 是单位切向量对弧长的导数，支持 2D/3D，magnitude 是曲率大小。

这些接口用齐次 de Boor 和解析有理导数计算原曲线，不通过显示三次曲线求值；支持 full/rhino 节点与省略权重。内重节点处取右侧值，区间终点取左侧值。仅在对应侧导数存在时才具有几何意义；零速点的曲率会报错，不在区间外外推。可用于参数标记、切线箭头、曲率梳及密切圆。`examples/evaluation-tests.typ` 验证有理圆弧、3D、参数缩放、重节点和周期接缝；构建脚本会执行它。

## 下载与安装本地 Typst 包

源码地址：[ArchiBC/breplot](https://github.com/ArchiBC/breplot)，独立库在
[`cetz-nurbs/`](https://github.com/ArchiBC/breplot/tree/main/cetz-nurbs)。
也可以[下载 ZIP](https://github.com/ArchiBC/breplot/archive/refs/heads/main.zip) 后解压。
仓库不包含构建产物，首次使用需按前述要求安装 Rust、WASM target 和 Typst。

Windows PowerShell：

```powershell
git clone https://github.com/ArchiBC/breplot.git
Set-Location breplot
.\cetz-nurbs\scripts\build.ps1
.\cetz-nurbs\scripts\install-local.ps1
```

安装器将四个运行文件复制到 `%APPDATA%\typst\packages\local\cetz-nurbs\0.1.0`
并核对 SHA256；设置了 `TYPST_PACKAGE_PATH` 时优先使用该目录。
可用 `-PackagePath` 指定另一个包根目录；非默认目录需要相应的 Typst 配置。

```typst
#import "@local/cetz-nurbs:0.1.0": nurbs, evaluate-point, evaluate-derivatives
```

从此其他项目无需复制 `package/`；更新源码后重新构建、安装即可。
