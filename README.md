# breplot

**主项目：纯 Rust 3dm 读取与 B-Rep 显示。** 从 3dm 原始几何与拓扑生成可复现的技术插图，输出 SVG 或 PDF Type 4，再用 Typst 排版。STEP 导入、旧内核、Maquette 和相应包／样例已经退役。

## 快速开始

需要 Rust 1.96。在仓库根目录直接运行：

```powershell
# 读取官方 Logo，输出对象／拓扑统计
cargo run --release -- nurbs-surface/3dm/fixtures/logo.3dm
# 输出着色图；后续参数依次为模式、U/V 结构线数、控制点及控制网开关、网格容差、曲线容差
cargo run --release -- nurbs-surface/3dm/fixtures/logo.3dm logo.pdf shaded 2 2 0 0.15 0.01
# 原生程序
./target/release/breplot.exe model.3dm model.svg hidden 4 4
```

默认构建是纯 Rust，不链接几何内核、不启动 DLL/Python/.NET，也不读取模型保存的显示网格。根 `Cargo.toml` 只依赖本仓库的 `nurbs-surface`，传递依赖中没有第三方 crate。

## 主文档与构建

根目录 `main.typ` 是 3dm 主文档入口，复用 [纯 Rust 3dm 原理与实现文章](nurbs-surface/3dm/pure/main.typ)，同时解释归档、NURBS、B-Rep 拓扑、采样、裁剪和显示模式。生成文章还需 Typst CLI 与 Poppler `pdftoppm`。

```powershell
# 默认：纯 Rust 回归、Logo 着色／隐藏线、主文档与预览
./scripts/build-demo.ps1
# 可选：独立曲线／曲面教材及 WASM
./scripts/build-demo.ps1 -IncludeModules
```

主输出：`target/release/breplot.exe`、`target/main.pdf`、`target/main.png`；完整 Logo Type 4 插图为 `nurbs-surface/target/pure-3dm/logo.pdf`。首次预览根文档先运行构建以生成插图。

## 支持范围

官方 Logo 的 6 个对象、22 张面、35 条共享边、79 条 trim 全部读入。支持现代 64 位 chunk 归档中的 NURBS B-Rep 子集，以及直线、复合曲线和相关 Rhino 类别名。保留原始节点向量、权重、控制点及拓扑关系，只在显示时采样。

支持着色、隐藏线、线框、控制点／控制网、结构线及其 U/V 密度、边界与近似视轮廓。不转换挤出、旋转面、解析平面、SubD、块实例；未知对象明确报告，调用方应检查 `model.skipped`。名称、图层、单位和材质暂不解析，坐标不自动换算单位。详细边界见 [曲面与 B-Rep 核心](nurbs-surface/README.md)。

UV 裁剪和视轮廓仍是离散近似；平均深度排序不保证复杂穿插的遮挡正确性，不执行实体缝合或工业级拓扑有效性认证。

## 模块与依赖隔离

- `nurbs-surface/rust/`：主项目使用的纯 Rust 3dm、曲面、裁剪及显示核心，无第三方 Cargo 依赖。
- `cetz-nurbs/`：独立曲线教材与 CeTZ 插件，不是主项目依赖。它仍使用实际需要的 `brepkit-math`、Serde 和 WASM 协议；CeTZ 仅用于该模块排版绘图。没有把这些依赖带入 3dm 主程序。

`brepkit-io`、`brepkit-topology`、`brepkit-operations` 和 Maquette 已随旧链路删除。保留的曲线数学库许可为 AGPL-3.0-only，CeTZ 为 LGPL-3.0-or-later。官方 DLL、C ABI 适配器及其测试／构建入口已移除。

```powershell
cargo tree
cargo test --release --manifest-path nurbs-surface/rust/Cargo.toml
cargo test --manifest-path cetz-nurbs/rust/Cargo.toml --lib
```
