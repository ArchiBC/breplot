# 项目协作约定

默认用中文与用户交流；代码标识符、命令和原始日志保留原文。Windows 本地任务使用 PowerShell 语法；涉及 WSL、Linux 或容器时按实际环境选择 shell。

## 项目与现状

`breplot` 用于从 STEP/B-Rep 生成可复现的 Typst 教材插图。默认是位图平滑曲面与矢量 Bézier 边线的混合输出；当前已有 Rust 原生 CLI 和 Typst WASM 演示。[`README.md`](README.md) 记录运行步骤、实际支持范围与已知限制。不要把 Bézier 近似描述成精确 NURBS 转换或工业级隐藏线算法。

目录分为 Typst 包 `package/`、Rust 子项目 `rust/`、演示入口 `main.typ` 和示例 `examples/`。`package/typst.toml` 指向 `package/lib.typ`；`main.typ` 是演示文档，不是包入口。`package/lib.typ` 接收调用方读取的 STEP 字节，不应在包内读取使用方的字符串路径。构建后 `package/breplot.wasm` 与 `lib.typ` 同目录。

Rust 关键文件：`rust/src/step.rs` 只为 STEP 省略的放置参考方向做内存兼容处理；`cetz-nurbs/rust/src/nurbs.rs` 校验标准 NURBS 输入并提取齐次 Bézier 节点区间；`cetz-nurbs/rust/src/display.rs` 负责共享三次曲线转换；`rust/src/curve_display.rs` 保留 CLI SVG 并复用独立内核；`rust/src/camera.rs` 是共用正交相机；`rust/src/bezier.rs` 保留其他解析边的 Hermite 近似与 HLR 短弦；`rust/src/hlr.rs` 将短弦按投影三角形的遮挡区间切分；`rust/src/shade.rs` 计算平涂面片和法向量；`rust/src/raster.rs` 使用内置 Maquette 光栅器逐像素着色；`rust/src/lib.rs` 负责导入、两级曲面三角化、原生曲线数据和兼容 CLI 的 SVG 输出。`package/step.typ` 用 Typst 原生图元绘制。独立插件 `cetz-nurbs/` 有自己的清单、WASM 和构建脚本，`cetz-nurbs/package/lib.typ` 提供 CeTZ `nurbs` 绘图元素与 `nurbs-curve` 插图包装器，不依赖 STEP 插件。

## 实现原则

- 保持 B-Rep 拓扑边与三角网格边的来源区别。普通边来自 B-Rep 原始曲线，高密度网格辅助遮挡，显示网格负责曲面着色与第一版曲面视轮廓。默认参数在 `testdemo/demo.stp` 上应保有至少 100 万个有效遮挡三角形，遮挡代理短弦单段长度不超过 0.5 mm；不要把单段长度称为显示 Bézier 或 NURBS 的全局精度。
- 正交投影是当前接口；修改相机坐标系或遮挡深度时，验证方向、前后关系及线段被部分遮住的情况。
- `main.typ` 的 Tinymist 预览使用 `target/preview-step.bin` 快照，由 `scripts/refresh-preview.ps1` 根据 `examples/step-preview.json` 生成。修改 STEP、视角或预览参数后刷新缓存；正式 `scripts/build-demo.ps1` 仍用完整高密度参数现场计算，不能把预览快照当作精度验证。
- 不以“原生矢量图元”推断整张图或曲线几何精确。默认曲面是 PNG；低次非有理 NURBS 边可按节点区间精确转 Bézier，其余边仍含近似。记录近似与深度容差；改进曲线质量时继续使用原始边曲线。
- 独立 NURBS 基础接口仅使用节点、权重和控制点，允许 2D/3D；`knot_format` 支持 full/rhino，并从数组长度推导次数，不自动补控制点。`from-control-points` 与 `from-interpolation-points` 两个点集扩展才提供 degree、close、periodic 选项；普通节点均匀且两端夹紧，内插用 Greville 参数，周期形式使用周期节点。插件演示入口是 `cetz-nurbs/main.typ`。文档示例直接定义节点和控制点；`view: (x:, y:, z:)` 与 CeTZ `ortho` 旋转角兼容。正交投影是仿射变换：先投影控制点，次数、节点和权重不变；非有理低次段可精确转 Typst 原生三次曲线。一般有理段的原生三次显示仍是近似；旧 SVG 圆弧只留在 CLI 中。
- `brepkit` 目前用于演示且标注 `AGPL-3.0-only`。变更依赖或准备公开分发时，核查当前许可证、API、WASM 兼容性及实际 STEP 样本，而不是沿用旧对话的概括。
- 几何层输出二维图层，Typst 层负责排版和文字。新增接口应尽量保留可见边、隐藏边、轮廓等语义。

## 验证与交付

- 在 `cetz-nurbs/rust/` 和 `rust/` 分别运行 `cargo test --lib`，再执行 `scripts/build-demo.ps1` 验证真实 `testdemo/demo.stp` 的 WASM/Typst 链路。原生构建通过不能替代 Typst 编译验证。
- HLR 变更应有能验证前后深度、部分遮挡、无遮挡的测试；检查实际 PNG 视觉结果，不以非空输出作为充分证据。
- 若 STEP 样本失败，保留失败案例并定位解析、拓扑、三角化或投影阶段。不要通过替换样本或刷新基准掩盖失败。
- 保留用户已有文件与未提交工作，只修改任务范围内的内容；文档中的完成状态要与实际可运行功能一致。
