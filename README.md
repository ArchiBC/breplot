# breplot

`breplot` 是给 NURBS/B-Rep 教材制作 Typst 技术插图的实验项目。输入 STEP 模型和正交视角，在 Typst 编译时生成带平滑曲面、高光与技术线条的插图；目标是让书中的视图可以随模型和视角参数一起重建。默认输出是**PNG 曲面 + Typst 原生 `curve` 边线**的混合图像。

## 现有演示

目前已有一条实际运行的 `STEP → Rust/WASM → Typst 原生曲线与 PNG` 链路。样例是 [`testdemo/demo.stp`](testdemo/demo.stp)，由 Rhino 8.35 导出，包含 6 个实体与 37 个面。

```text
testdemo/demo.stp
  → brepkit STEP 导入
  → B-Rep 原始曲线的自适应 Bézier 近似 + 高密度遮挡网格 + 显示网格
  → 正交投影 + 遮挡区间裁剪
  → 逐像素着色 PNG + visible / hidden / silhouette 原生 curve 图层
  → Typst 页面
```

Rust 插件沿用 [Maquette](https://github.com/bernsteining/maquette) 的光栅器：Typst 用 `read(path, encoding: none)` 传入 STEP 字节；WASM 单次返回二进制封包，其中包含曲线 JSON 和曲面 PNG。Typst 直接用 `curve.cubic` 绘制可见边和隐藏边，`curve.line` 绘制视轮廓，`image(..., format: "png")` 嵌入曲面。遮挡算法借鉴 [Scenery 的线段区间裁剪](https://github.com/GiggleLiu/scenery/blob/main/scenery-engine/src/clip.rs)：每个挡住线段的三角形贡献一个参数区间，合并这些区间后切分可见线与隐藏线。高密度网格用于遮挡；较疏的显示网格交给 Maquette 的逐像素光栅器，插值法向量、使用深度缓冲并计算高光。普通边来自 B-Rep 拓扑边的原始曲线；NURBS 边按节点区间转 Bézier，其他解析边暂用自适应 Hermite 近似。遮挡裁剪使用短弦代理，显示曲线保留三次 Bézier。

## 运行

需要 Rust 1.96.0、`wasm32-unknown-unknown` target 和 Typst CLI。Windows PowerShell：

```powershell
.\scripts\build-demo.ps1
```

脚本在 `rust/` 子项目运行单元测试并编译 WASM，生成编辑预览缓存，再编译根目录的 [`main.typ`](main.typ) 和 [`examples/`](examples/) 中的视图、隐藏线、平涂和 NURBS 示例。它还把包临时复制到 `target/typst-packages/local/breplot/0.1.0`，通过 `@local/breplot:0.1.0` 编译 [`examples/package-import.typ`](examples/package-import.typ)，验证清单入口。输出为 `target/main.png`、`target/demo.png`、`target/hidden.png`、`target/flat.png`、`target/nurbs.png` 和 `target/package-import.png`。WASM 文件、预览缓存及生成结果不纳入源码；直接运行 Typst 示例前需要先构建插件。

目录按 Typst 包与 Rust 子项目划分：[`package/typst.toml`](package/typst.toml) 声明包名、版本和入口 [`package/lib.typ`](package/lib.typ)，WASM 构建后与入口同目录。根目录 `main.typ` 是可直接编译的演示文档，包含 STEP 视图与两种 NURBS 曲线；3D 示例还显示投影坐标轴、控制点编号、完整节点向量和世界坐标。它不是包入口，也没有声明为 Typst 模板。包本身仍处于本地演示阶段；公开分发前需处理下文所述的依赖许可证。

VS Code 的 Tinymist 使用 `quality=cached`：`main.typ`、`examples/demo.typ` 和 `examples/hidden.typ` 读取同一份 `target/preview-step.bin`，保存文档时只重排原生曲线和文字，不重新导入 STEP。缓存包含隐藏边，按文档参数决定是否显示。缓存使用 [`examples/step-preview.json`](examples/step-preview.json) 的**完整高密度参数**。首次打开前运行一次 `scripts/build-demo.ps1`；修改 STEP 文件、视角或预览参数后，运行 `scripts/refresh-preview.ps1` 重新生成缓存。预览缓存是静态快照；在刷新前它不会自动反映这些几何输入的变化。工作区还把预览刷新设为保存时触发。

直接执行 `typst compile main.typ target/main.png` 或构建脚本时，`main.typ` 会现场计算完整精度：默认至少约 100 万有效遮挡三角形、遮挡短弦不超过 0.5 mm。需要在命令行复现编辑器的快速预览，可执行：

```powershell
typst compile --input quality=cached main.typ target/main-preview.png
```

若要验证标准包导入，脚本已把本地包放在 `target/typst-packages`，执行：

```powershell
typst compile --package-path target/typst-packages examples/package-import.typ target/package-import.png
```

对应的 Typst 导入形式是 `#import "@local/breplot:0.1.0": step-view, nurbs-curve`；在自己的文档中使用时，需要把 `package/` 的五个文件（`typst.toml`、`lib.typ`、`step.typ`、`curve.typ`、构建后的 `breplot.wasm`）放到本地包目录 `local/breplot/0.1.0`。

`step-view-data` 可绘制 CLI `cache` 命令生成的二进制图层封包，用于反复排版同一视角；普通 `step-view` 仍在每次编译时从 STEP 重新计算。两种方式都使用原生 Typst 曲线，缓存不含 SVG。

也可直接运行原生 SVG 命令行程序：

```powershell
cargo run --manifest-path rust/Cargo.toml --bin breplot-cli -- testdemo/demo.stp target/demo.svg
```

可传入第三个参数作为 JSON 配置文件，例如：

```json
{
  "view": [35.264, 45, 0],
  "deflection": 0.005,
  "visibility_deflection": 0.001,
  "surface_deflection": 0.02,
  "max_segment_length": 0.5,
  "surface": true,
  "surface_mode": "raster",
  "surface_pixels": 1600,
  "specular": 0.18,
  "hidden": false,
  "silhouette": false
}
```

Typst 调用方式见 [`package/lib.typ`](package/lib.typ)。在使用方文档中读取 STEP 字节，再传入 `step-view`；包内读取字符串路径时，Typst 会以包文件为基准解析，无法可靠访问使用方的模型文件。

```typst
#import "package/lib.typ": step-view

#step-view(
  read("testdemo/demo.stp", encoding: none),
  width: 90%,
  view: (x: 35.264deg, y: 45deg, z: 0deg),
  hidden: false,
)
```

`view: (x: ...deg, y: ...deg, z: ...deg)` 与 CeTZ `ortho` 的三个旋转角兼容，按 Z、Y、X 顺序作用于模型；若省略 `view`，仍可使用旧的 `direction`/`up` 相机参数。`deflection` 控制 Bézier 对原始边曲线的采样检查容差；`visibility_deflection` 控制遮挡网格密度；`surface_deflection` 控制着色网格与近似视轮廓的密度。`max_segment_length` 限制遮挡测试所用短弦的三维单段长度，**不限制显示 Bézier 段长**。这些长度都使用 STEP 模型单位，本样例单位是毫米。`surface_mode: "raster"` 使用直接嵌入的 PNG 平滑曲面；`"flat"` 用 Typst 原生三角面片作对照，编译更慢。`surface_pixels` 是曲面位图宽度，范围 256–4096。`specular` 控制高光强度，`hidden: true` 显示虚线隐藏边，`silhouette: true` 可显示网格近似的视轮廓。

## 独立 NURBS 曲线模块

输入采用标准的**完整、重复节点向量**，不是“唯一节点 + 重数”压缩形式。`degree = p`、控制点数 `n` 时，`knots` 必须有 `n + p + 1` 个有限且非递减的数；有效参数域是 `[knots[p], knots[n]]`。支持非夹持节点向量，零长度节点区间跳过。控制点可写 `[x, y]` 或 `[x, y, z]`，权重省略时全为 1；显式权重必须与控制点等长且为正。当前支持 1–8 次曲线。

[`examples/nurbs-polynomial.json`](examples/nurbs-polynomial.json) 展示 3D 三次曲线和控制结构；[`examples/nurbs-rational.json`](examples/nurbs-rational.json) 展示有理二次四分之一圆。原生命令：

```powershell
cargo run --manifest-path rust/Cargo.toml --bin breplot-cli -- curve examples/nurbs-polynomial.json target/nurbs-polynomial.svg
```

Typst 可以直接传字典：

```typst
#import "../package/lib.typ": nurbs-curve
#let spec = (
  degree: 2,
  knots: (0, 0, 0, 1, 1, 1),
  control_points: ((70, 0), (70, 70), (0, 70)),
  weights: (1, calc.sqrt(2) / 2, 1),
  tolerance: 0.01,
)
#nurbs-curve(spec, width: 65%)
```

`view` 使用 CeTZ `ortho` 的角度字典，3D 默认 `(x: 35.264deg, y: 45deg, z: 0deg)`；2D 默认零旋转。3D 控制点先经正交仿射投影成 2D 控制点，**次数、节点向量、权重不变**，因此这一步得到的是精确的 2D NURBS 投影。`style` 支持 `stroke`、`stroke_width`、`opacity`、`dash`、`show_control_points`、`show_control_polygon`、`show_control_labels`、`show_axes`、`axis_length`、`control_polygon_stroke`、`control_point_fill`、`control_point_radius`。颜色使用 `#RRGGBB`；宽度、半径、虚线长度、`axis_length` 与 `tolerance` 都使用模型单位。`show_axes` 从世界原点绘制正向 X/Y/Z 轴，用同一视角投影到图面；`axis_length` 省略时取三维控制点包围盒最大尺寸的四分之一。控制点、控制多边形和坐标轴由 Typst 原生图元绘制，不改变曲线几何。

曲线模块先将每个非零节点区间提取为齐次 Bézier 段。非有理 1–3 次段精确升阶为 Typst 原生三次 `curve.cubic`；Typst 原生 `curve` 没有通用有理 Bézier 段，所以有理段及高于三次的段按 `tolerance` 自适应近似为三次 Bézier，不会输出密集折线。该容差用段内多个采样点检查，**不是严格的全局误差证明**；达到细分预算时会报错。CLI 保留旧 SVG 输出，用于单独导出与对照。

默认参数在本样例的原生运行中产生约 **119.6 万个有效投影遮挡三角形**、约 **6.6 万个着色面片**；73 条 B-Rep 边转成约 **322 段三次 Bézier**，遮挡代理短弦最长 **0.463 mm**。三角形数量在不同运行中有少量波动。百万网格只用于内部遮挡，不写入文档。0.5 mm 是代理短弦长度上限，不能等同于 Bézier 对原始 NURBS 的全局几何误差保证。

## 实现边界

- STEP 的 NURBS 边使用同一节点区间提取模块，但 HLR 仍用三维短弦代理，再按遮挡区间裁剪三次曲线。圆/椭圆等其他解析边仍走自适应 Hermite。一般有理曲线不能用普通三次 Bézier 精确表示；`deflection` 和独立曲线的 `tolerance` 都不是严格证明的全局误差界。
- 曲面用高密度三角网格判断遮挡，显示面片是另一个较疏的着色网格，视轮廓来自显示网格中相邻三角形的朝向变化。它们都不是解析式 B-Rep HLR；近距遮挡、网格缝隙、轮廓端点可能有误差。普通边的遮挡深度容差是 `1.5 × deflection`，显示网格轮廓的容差至少是 `2 × surface_deflection`。
- 默认平滑曲面是直接嵌入 Typst 的 PNG；B-Rep 边线、隐藏线和可选视轮廓使用原生矢量曲线。`surface_mode: "flat"` 可生成原生三角面片平涂版本，放大后仍能看出面片。
- 当前输出处理 `MANIFOLD_SOLID_BREP`，并未证明支持任意 STEP 来源。为读取本样例，`rust/src/step.rs` 会在内存中给省略了参考方向的 `AXIS2_PLACEMENT_3D` 补充垂直方向，不修改原始 STEP 文件。
- 演示使用 [`brepkit`](https://github.com/andymai/brepkit) 3.4.18，其仓库标注为 `AGPL-3.0-only`。此处只做本地验证；公开分发插件前需要确定依赖与许可证方案。
- `rust/vendor/maquette-core` 固定自 [Maquette `fdb31b1`](https://github.com/bernsteining/maquette/tree/fdb31b1a3f5e667fcd51b87a323051e647bfe55a/crates/maquette-core)，上游标注 MIT；源码内置以避免插件构建时拉取不稳定的 Git 依赖。
- 样例模型中的部分曲面三角化结果有边界缝隙；启用可选的网格视轮廓时，也可能出现局部尖点。还没有对其他 STEP 文件或视角作系统验证。百万网格增加编译耗时和内存占用。

## 目录

```text
main.typ                  根目录演示入口，可直接编译
package/typst.toml        Typst 包清单
package/lib.typ           包入口；构建后加载同目录的 breplot.wasm
package/step.typ          STEP 原生曲线、PNG 曲面与 CeTZ 视角
package/curve.typ         NURBS 原生曲线与控制结构
rust/Cargo.toml           Rust CLI 与 WASM 子项目
rust/src/nurbs.rs         输入校验与节点区间齐次 Bézier 提取
rust/src/curve_display.rs 非有理精确升阶、有理自适应显示；CLI 保留 SVG
rust/src/camera.rs        STEP 与独立曲线共用的正交相机
rust/src/                 其余 STEP 兼容处理、遮挡区间裁剪、平滑着色与原生曲线数据输出
rust/vendor/maquette-core 固定版本的 WASM 光栅器，含 MIT 许可证
examples/*.typ           可编译的普通视图、隐藏线及 NURBS 示例
testdemo/demo.stp        Rhino STEP 样例
scripts/build-demo.ps1   构建并验证入口与示例
scripts/refresh-preview.ps1 重新生成 Tinymist STEP 预览缓存
```

下一步重点是处理光滑曲面的连续视轮廓、建立更多遮挡和曲线近似回归样例，并决定适合公开分发的依赖组合。项目缘起见[原始讨论](https://chatgpt.com/share/6ab3411e-7e30-83ea-8cee-c465fccd0bc9)。
