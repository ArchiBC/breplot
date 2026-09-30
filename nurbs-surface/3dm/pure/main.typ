#set document(title: "纯 Rust 3dm：从官方 Logo 到 B-Rep 显示", author: "breplot")
#set page(paper: "a4", margin: 17mm, numbering: "1")
#set text(font: ("Microsoft YaHei", "Arial"), size: 10pt, lang: "zh")
#set par(justify: true, leading: 0.6em)
#show raw: set text(font: "Consolas", size: 8pt)
= 纯 Rust 3dm：从官方 Logo 到 B-Rep 显示

目标文件来自 McNeel openNURBS 8.x 的 `example_files/V7/v7_rhino_logo_nurbs.3dm`。这是该仓库当前最新的 NURBS Logo 样本；并非把旧模型另存后替换目标。归档版本为 70，原文件 882441 字节，包含 6 个 B-Rep 对象，全部读入，跳过 0 个。

#figure(image("../../target/pure-3dm/logo.png", width: 88%), caption: [Rust 解码、裁剪、细分及着色。本文嵌入位图预览以保持轻量；独立 `logo.pdf` 保留 PDF Type 4 曲面与矢量线。])

读取过程是 `bytes → chunk → class → NURBS / topology → display`。`pure3dm::read` 接受内存字节，不加载 Rhino、DLL，不启动 Python 或 .NET，也不使用文件中保存的显示网格。当前 crate 无第三方 Cargo 依赖。解析与显示分离，调用方可保留精确归档几何，只在显示时进行离散化。

```rust
let model = pure3dm::read(&bytes)?;
// 调用方必须检查 model.skipped，避免把部分支持当成完整读取。
let brep = model.to_brep(0.01)?;
let scene = brep.display(display, mesh_options, render_options)?;
let pdf_bytes = scene.pdf()?; // 在内存中生成，无中间几何转换文件
```

#pagebreak()
= 归档协议与精确几何

`rust/src/pure3dm/archive.rs` 负责有界读取。现代归档的 chunk 头是 32 位类型码与 64 位长度／短块值。短块没有载荷；带 CRC 的块末尾保存 CRC32。CRC 只覆盖当前层直接读取的字节，完整子块不计入父 CRC。`Reader::take` 更新 CRC，`Reader::chunk` 则读取独立的子作用域。读取长度、计数、嵌套层数与文件总大小均有上限，EOF 记录的文件大小必须匹配。

`geometry.rs` 识别 NURBS 曲线、NURBS 曲面、直线与复合曲线，也识别目标中的 `TL_NurbsCurve`、`TL_NurbsSurface` 类别名。复合曲线保存分段参数与子曲线，不强行拟合成另一条 NURBS。当前不转换旋转面、挤出、解析平面、圆弧、SubD、网格、块实例。遇到未知几何类时整对象进入 `skipped`，不会只留其中几张可读面。

Rhino 的省略两端节点格式恢复为完整节点向量；不使用重数表。归档中的有理控制点是齐次坐标 $(w x,w y,w z,w)$，读取后保存欧氏控制点与权重，求值时只乘一次权重：
$ S(u,v) = frac(sum_(i,j) N_i(u) M_j(v) w_(i,j) P_(i,j), sum_(i,j) N_i(u) M_j(v) w_(i,j)). $

`topology.rs` 保留顶点、共享边、裁剪边、环和面，以及代理曲线的子区间与反向标志。面指向曲面，环指向面，trim 指向二维曲线与共享三维边；反向是拓扑属性，不改写控制点。代理参数 $t$ 在其显示定义域 $[a,b]$ 上归一化为 $s=(t-a)/(b-a)$，反向时取 $1-s$，再映射到原曲线子区间。`rev3d` 与二维代理反向是两个独立字段。

#table(columns: 4, inset: 5pt, [对象序号], [面], [共享边], [裁剪边],
  [0], [4], [6], [12], [1], [3], [6], [16], [2], [2], [2], [5],
  [3], [6], [12], [28], [4], [4], [6], [12], [5], [3], [3], [6],
  [总计], [22], [35], [79])

当前仅读取几何和拓扑。名称、图层、材质、单位设置、用户数据及缓存网格按块边界跳过；坐标保持文件数值，不做单位转换。CRC 校验覆盖已解码的记录，不能据此声称整个文件所有不透明数据都已校验，也不声称实现完整 3dm 规范。

#pagebreak()
= 从裁剪曲线到显示模式

`display.rs` 在每个非零节点区间内检查四分点与中点，按世界空间曲线偏差及归一化 UV 偏差细分。二维 trim 按代理方向拼接为闭合环；三维边只存一份显示折线，面引用其索引。原曲线仍保留在 `RawBrep` 中。四分点检查是采样判据，不是严格全域误差界。

`TrimRegion` 对 UV 多边形裁剪；完全位于内部的网格单元直接保留，避免被环的耳切对角线反复切碎。退化边界采用局部坐标计算有理导数；对于无法确定唯一法线的奇点，从当前三角形内部接近求取方向相关的极限近似，保持顶点位置不变。接近数值零面积的三角形不参加显示。

可设置 `Shaded / HiddenLine / Wireframe`、控制点、控制网、结构线及 U/V 密度、拓扑边界与近似视轮廓。本例曲线容差为 0.01，网格采样容差为 0.15，均为文件坐标单位；正交视向为 $(0,-1,0)$。结构线密度是每张面每方向的内部等参数线数量。

#figure(image("../../target/pure-3dm/logo-hidden.svg", width: 70%), caption: [同一 Rust B-Rep 数据的隐藏线模式，U/V 密度均为 2。])

移除官方 DLL 前的历史对照结果：22 张面的控制点、节点与权重完全一致；曲面网格采样、35 条边与 79 条 trim 的代理参数采样最大点差为约 $1.02 times 10^(-14)$。测试另外覆盖损坏 CRC、截断、超长块、无效拓扑、未知对象跳过，以及纯 Rust 到 Type 4 PDF 的完整链路。

当前仍不是完整实体内核：不做缝合或闭合有效性认证，UV 裁剪是折线近似，轮廓来自显示网格；曲面按三角形平均深度排序，交叠或穿插几何可能遮挡错误，细小部位也可能出现线段碎片。每环最多 256 点、每对象／显示最多 1024 面；超限显式报错。着色 RGB 采样容差保持 0.001，递归上限 12，每原三角形最多 65536 着色片、全图最多 1000000 片。

复现：仓库根目录运行 `./scripts/build-demo.ps1`；直接读文件使用 `cargo run --release -- model.3dm model.pdf`。现有测试与构建完全不依赖官方 DLL；C ABI 适配器和对照测试已删除。脚本仅需 Rust、Typst、Poppler；详细命令与样本来源见 `nurbs-surface/README.md` 和 `3dm/fixtures/README.md`。
