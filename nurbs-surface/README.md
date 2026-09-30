# 独立 NURBS 曲面数据

本模块独立实现数据表示、参数求值、一阶偏导、法向与 UV 三角网格，不依赖 OCCT、brepkit 或现有曲线内核；Rust crate 无第三方依赖。另提供基于顶点法向与 SVG 线性渐变的原生 Gouraud 预览。不依赖第三方绘图库；已支持多边形 UV 裁剪与等参结构线；尚未实现通用实体遮挡。已提供 Type 4 内存输出和仅供本文案例使用的 Typst/WASM 调用实验，尚无通用曲面参数的 WASM 输入协议。

`main.typ` 是自解释入口：说明张量积、节点与有效参数域、数据不变量，以及对应 Rust 实现。关键片段直接从源码读取；圆柱控制网格示例也是可运行 Rust 程序。

```powershell
.\nurbs-surface\scripts\build.ps1
```

输出 `main.pdf` 和 `target/main-*.png`，运行单元测试与数据示例，并检查 `wasm32-unknown-unknown` 编译。根目录 `scripts/build-demo.ps1` 同样调用此脚本。

## 数据约定

- `KnotAxis` 直接存完整重复节点数组。`new(knots, pole_count, Full/Rhino)` 从长度推导次数，支持 1–25 次。Rhino 输入仅补两端各一个节点，不恢复已省略的原始外围值。
- `NurbsSurface::new(u, v, control_points, weights)` 接收 `[u][v]` 三维点网格与同形权重网格；省略权重表示全 1。内部按 `u * v_count + v` 展平，只提供只读访问。
- `NurbsSurface::bezier(points, weights)` 从网格大小构造 `[0,1]²` 上的夹紧单跨面，不补控制点。
- 节点必须有限且非递减，每个值最多出现 `degree + 1` 次，允许内部完全断开。有效域为 `knots[degree]..knots[pole_count]`，必须有正有限长度；`spans()` 跳过零长区间并保留完整数组下标。
- 权重必须有限且严格为正，不归一化；坐标必须有限。允许几何退化，数据合法不代表法向存在或面无自交。
- 周期数据由调用方显式提供展开后的节点与控制点；不设周期开关，不补点，不验证周期接缝。闭合与周期性不能仅由首尾位置判断。
- 有理性查询表示权重沿相应方向是否变化，使用精确比较；统一非单位权重仍属于非有理表示。不做特殊几何抵消分析。

## 参数求值与网格

- `evaluate(u,v)` 返回 `SurfaceSample { point, du, dv, normal }`。解析偏导相对于原节点参数；内部节点取右侧，域末端取左侧。参数越界报错，奇异位置法向为 `None`。
- `mesh_uniform(n, max_vertices)` 对每个非零节点矩形生成 n×n 小格并三角化。顶点携带 UV、原曲面法向和节点区间下标；节点矩形之间不焊接，保留折痕和断开的单侧数据。
- `tessellate(MeshOptions)` 根据三角形重心和边中点的三维误差整体翻倍加密，达到预算仍不满足容差时报错。这不是局部自适应算法，也不是严格全局误差保证；尚无法向夹角控制。
- 零面积三角形跳过并计数；`Mesh` 保留 `sampled_error`、`subdivisions`、`degenerate_triangles` 诊断。
- `rust/examples/mesh.rs` 生成平面、完整柱面、完整有理球面与鞍面，输出 `target/meshes.json`、四组光滑 SVG 与网格叠加 SVG。球面极点保留空解析法向，展示时以邻面法向近似补齐。

## 裁剪与结构线

- `TrimRegion::new(outer, holes)` 接收简单多边形 UV 外环和多个不相交孔洞，支持凹环；不自动解释周期接缝，暂不接受 NURBS 裁剪曲线。
- `mesh_trimmed(region, subdivisions, max_vertices)` 在 UV 中切分三角形，新增点回原曲面求值。`tessellate_trimmed(region, options)` 检查裁剪后误差并整体加密。细小碎片使用归一化 UV 容差过滤，不是精确几何布尔运算。
- `structure_lines(region, u_values, v_values, samples_per_span)` 生成按裁剪域截断的等参折线与裁剪边界，保留 `IsoU`、`IsoV`、`TrimBoundary` 语义；不将三角网格边当作结构线。
- `render_svg_with_lines(mesh, lines, options)` 在不透明模式下用线段—三角形深度区间遮挡，在透明模式下显示全部结构线。基于近似网格与短弦，带自遮挡深度偏置，尚无 BVH 或解析 HLR。
- 文档包含带孔鞍面、裁剪球面和完整球面结构线实例，均由独立 Rust 实现生成。

## OCCT 参考与差异

参考固定版本 [OCCT V7_9_0 Geom_BSplineSurface](https://github.com/Open-Cascade-SAS/OCCT/blob/V7_9_0/src/Geom/Geom_BSplineSurface.cxx) 的 U/V 分离、二维控制网格、权重形状校验和几何/拓扑分离。代码独立编写，未复制 OCCT 源码，也不链接 OCCT。

按项目要求，不采用 OCCT 的独立节点加重数存储、不采用其紧凑周期表示。这里使用完整节点向量；内部节点允许出现 `p+1` 次。节点不做容差合并，权重不套用 OCCT 的最小几何分辨率阈值。本模块不是 OCCT API 或数值行为兼容层。


## 原生光滑显示

`SvgOptions` 提供 `specular`（0–1，默认 0.25）、`shininess`（0–10000，不含 0，默认 32）和 `opacity`（0–1，默认 1）。高光采用 Blinn–Phong 强度，在 Rust 中与漫反射作有界混合，直接输出最终 RGB 渐变，避免独立高光透明遮罩；材质透明度每个三角形合成后只施加一次。半透明前后面分别叠加，沿用平均深度排序，不模拟折射，也不保证复杂相交面的透明顺序正确。光照在同一包围盒上合成后统一裁切；半透明模式仅允许父三角形内的着色小片扩展覆盖，父组施加一次透明度，避免细分重复叠色；原三角形之间可能保留抗锯齿接缝。文档提供相同球面网格的无高光、高光和透明对照。

`render_svg(mesh, SvgOptions)` 使用正交投影、顶点 Lambert 光照和每三角形一个线性渐变实现 Gouraud 着色，可选网格描边。先对插值法向归一化计算目标光照，再以小三角形渐变自适应逼近。`shading_tolerance` 默认 0.001，检查最终归一化 RGB 的采样误差（含单轴颜色拟合误差）（不是全域上界）；最多递归 12 层、每原三角形 65536 小片、全图 1000000 小片，超限报错。细分只增加 SVG 着色片，不改变几何和结构线。不是逐像素渲染，极窄高光可能被有限探针漏采。三角形采用平均深度排序，不保证相交面或复杂实体的遮挡正确性。不透明无网格模式使用有界斜切角扩展的统一裁切区域减轻白缝；漫反射与高光共享覆盖，轮廓精度仍由 mesh 决定。


## 文档显示与性能

默认 `scripts/build.ps1` 生成混合 PDF：由 Typst 将原生 SVG 曲面烘焙为透明 PNG（600×480 SVG 默认生成 1600×1280 像素），结构线、裁剪边界和网格描边仍以 SVG 矢量叠加。完整 SVG 不删除。阅读文档不再反复求值数万个渐变；极大倍率下曲面图片会显露像素。Rust 几何/着色仍无第三方依赖；图片转换复用已有 Typst CLI。

全矢量：`./nurbs-surface/scripts/build.ps1 -VectorSurfaces`。直接编译 `main.typ` 可传 `--input surface-output=vector`；默认混合模式要求先构建 `target/display/`。`build-display-assets.ps1 -Ppi 192` 可单独刷新图片。

实测 13 页文档从 61,985,308 字节降至 3,588,572 字节。PDFium 第 13 页、scale=2、各三次含首次渲染的中位数从 0.953 秒降至 0.042 秒；这是本机渲染测量，不代表所有阅读器的打开时间。

进一步探索和原型限制见 [rendering-performance.md](rendering-performance.md)。


## Type 4 无插图中间文件实验

运行 `./nurbs-surface/scripts/build-type4.ps1`，生成 `main-type4.pdf`。除 Rust/WASM 编译产物外，仅输出最终文档：不调用 mesh 示例，不读取 meshes.json，不生成或读取逐图 SVG、PNG、PDF。已在只含 main.typ、源代码片段、WASM 的隔离目录编译成功，新增文件仅 article.pdf。

```typst
#let engine = plugin("package/surface.wasm")
#image(engine.surface_demo(bytes("sphere-structure")), format: "pdf")
```

这是实际 PDF 字节 API，不是 Typst 原生 Type 4 元素接口，也不需要修改 Typst。Rust 通用接口为 `render_pdf` / `render_pdf_with_lines`；WASM 的 `surface_demo` 目前仅接受演示案例 ID。

全份 13 页测得 8,309,858 字节，0 个位图对象；PDFium 第 13 页 scale=2 三次中位数约 0.015 秒。使用未压缩网格流、32 位坐标、16 位 RGB；不能与旧 Python 单球面压缩原型的 120 KB 直接比较。透明三角形接缝和阅读器下的微小裂点仍存在，因此保持实验状态，默认仍为混合输出。


Type 4 最新优化：完整文档约 **1.82 MB**（此前 8.31 MB 为未压缩实验结果）。已加入共享边 flag 编码、RGB 专用着色细分、投影不重叠片批处理和原生 Flate 压缩，仍无第三方 Rust 依赖、仍由 WASM 内存传输。透明接缝尚未完全解决。规范依据和验证方法见 rendering-performance.md 最新章节。


## Edge / Adobe 色彩兼容修复

Edge 实测确认：将 Typst 嵌入 Type 4 插图时添加的 ICC 透明组混合空间改为 `/DeviceRGB` 后，完整文档恢复正常蓝灰色。正式 `scripts/build-type4.ps1` 已接入 `normalize-pdf-groups.py`，需要 Python + PyMuPDF（`python -m pip install PyMuPDF`）。此依赖仅用于最终 PDF 的兼容处理，Rust 几何/着色核心仍无第三方依赖。

处理保留组隔离和透明度，仅修改带着色资源的 Form 的 `/Group/CS`；写回前逐一检查所有解码数据流未变、页数未变。全程在内存处理最终 PDF，不生成逐图中间文件。直接运行 `typst compile` 不包含此修复，应通过构建脚本导出。已确认版本约 1.84 MB、13 页；透明排序和接缝的既有限制仍适用。

诊断注意：旧 `edge-group-diagnostic.pdf` 的 A/B 在 PyMuPDF 拼版时丢失了页面透明组，实际上与 C 一样没有组，不能据其标签推断 ICC 正常。`diagnose-edge-groups.py` 已改为显式添加 Form 组并断言，输出 `edge-group-diagnostic-v2.pdf`。


## B-Rep 显示装配与模式

`Brep::display(DisplayOptions, MeshOptions, SvgOptions)` 返回可输出 SVG/PDF 的 `DisplayScene`。支持 `Shaded`、`HiddenLine`、`Wireframe`，以及独立的控制点、控制网、结构线、边界、近似轮廓开关。视轮廓为实验性功能，默认关闭；通过 `DisplayOptions { silhouettes: true, ..Default::default() }` 显式启用，拓扑边界仍默认显示。U/V 结构线数量 `iso_count` 与几何细分独立；控制辅助始终透视显示。所有面的网格共同参与线条遮挡，显式共享边 ID 仅绘制一次。

`demo::cube_brep()` 提供六个 NURBS 面和十二条共享显示边的案例；`examples/display.rs` 生成五种显示示例，构建脚本及 `main.typ` 已纳入演示。边目前是调用者提供的折线，不执行自动缝合；裁剪仍是 UV 多边形，轮廓以显示网格定位法向与视向点积的零等值线，再在 UV 中二分求值原始曲面；切向点积近零判定避免节点边断线，共用零边去重。没有完整环拓扑、实体合法性验证或精确可见性保证。奇异法向处轮廓可能断开；着色画家排序不能保证相交面的结果。

## 纯 Rust 3dm B-Rep 读取：官方 Logo 目标

新入口 `pure3dm::read(&bytes)` 完全由 Rust 解码，无 Cargo 依赖，不加载 DLL、不调用 Python/.NET，也不读取保存的显示网格。返回原始 NURBS、复合曲线与 vertex/edge/trim/loop/face 拓扑；`Model::to_brep(tolerance)` 才采样为显示数据，复用现有显示模式、裁剪、结构线和 Type 4 输出。

目标为官方 openNURBS 8.x 中最新的 NURBS Logo 样本（archive 70），固定为 [logo.3dm](3dm/fixtures/logo.3dm)。**6 个对象、22 张面、35 条共享边、79 条 trim 全部解析，跳过 0 个**。移除官方 DLL 前曾完成一次对照验证：曲面节点、控制点、权重完全一致；曲面和代理边/trim 采样最大点差约 `1.02e-14`。

```powershell
# 从仓库根目录执行；只做解析统计时省略输出路径
cargo run --release --manifest-path nurbs-surface/rust/Cargo.toml --example read3dm_pure -- nurbs-surface/3dm/fixtures/logo.3dm logo.pdf shaded 2 2 0 0.15 0.01
./nurbs-surface/scripts/build-pure-3dm.ps1
```

CLI 输出路径后的参数依次为 `shaded|hidden|wire`、U/V 结构线数量、控制点及控制网 `0|1`、网格容差、曲线采样容差。API 的 `DisplayOptions` 可分别控制点、控制网、边界和轮廓。默认视向 `(0,-1,0)` 适合本 Logo，容差以原文件坐标数值计。自解释文档：[3dm/pure/main.typ](3dm/pure/main.typ)；输出 `target/pure-3dm/main.pdf`（轻量位图预览与矢量隐藏线图），独立 `logo.pdf` 保留 Type 4 着色。

范围明确限制为现代 64 位 chunk 归档（头版本 50/60/70/80，实际目标验证 70；80 的不支持对象样本也有回归）、B-Rep 3.0–3.3、NURBS 曲线/面及其 Rhino 别名、直线和复合曲线。不支持旋转面、挤出、解析平面、圆弧、块实例、SubD 等转换。未知几何整对象进入 `model.skipped`，CLI 打印原因；调用方应检查该报告。损坏已解码记录、非法索引和超限输入报错。名称、图层、单位、材质及用户数据暂不解码，不做单位换算；跳过块不作完整 CRC 校验声明。

文件上限 128 MiB、对象 4096、复合曲线嵌套 16 层，显示最多 1024 面、每环 256 点。裁剪仍是采样多边形，轮廓由显示网格近似，平均深度排序不保证复杂穿插正确，Logo 小孔附近仍可能看到碎线。纯 Rust 接口可编译到 WASM，但尚未给 Typst WASM 插件增加 3dm 字节入口。`cargo test --release --test pure3dm` 覆盖目标、损坏、跳过、拓扑及 PDF 链路。官方 DLL、适配器和对照测试已移除；上述对照数字是移除前的历史测量。
