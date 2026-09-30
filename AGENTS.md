# 项目协作约定

默认用中文与用户交流；代码标识符、命令和原始日志保留原文。Windows 本地任务使用 PowerShell 语法。

## 项目与现状

`breplot` 主项目是纯 Rust 3dm 读取与 B-Rep 显示，根 Cargo.toml 提供 breplot CLI，根 main.typ 是 3dm 主文档。旧 STEP 导入链路已经退役，不恢复其构建入口。实际支持范围以各模块 README 和可运行案例为准。

- `cetz-nurbs/`：独立曲线 Rust 内核、Typst 包与 CeTZ 演示；`package/typst.toml` 指向模块内的 `package/lib.typ`。几何主要在 `rust/src/nurbs.rs`、`construct.rs`、`display.rs`。
- `nurbs-surface/`：独立曲面、裁剪、B-Rep 显示及 3dm 子集解析；`rust/src/pure3dm/` 是纯 Rust 读取路径，官方 DLL 及 C ABI 适配器已移除，不恢复该路径。
- 根目录 `main.typ` 转入纯 Rust 3dm 文章；各模块 `main.typ` 同时解释原理、实现和限制，不是包入口。

## 实现原则

- 节点直接存数组，不改用节点加重数表；保留 full/Rhino 输入语义，不自动补控制点。构造扩展才提供 degree、close、periodic 选项。
- 区分 B-Rep 原始曲线、共享拓扑边与显示网格边。保留原始几何，只在显示阶段离散化；不把 Bézier 或采样折线近似称为精确转换或全域精度保证。
- 正交投影、相机方向和前后深度变更必须验证部分遮挡及无遮挡情况。几何层输出图层，Typst 负责文字排版；保留可见边、隐藏边、轮廓等语义。
- 纯 Rust 3dm 路径不依赖 DLL、Python、.NET 或保存的显示网格；未知几何明确报告，不静默丢面，不扩展挤出／旋转转换范围。测试不加载官方 DLL。
- 曲线模块仍依赖 brepkit-math，许可与其他依赖见模块说明；公开分发前核查当前许可，不沿用过期概括。

## 验证与交付

- 分别运行 `cargo test --manifest-path cetz-nurbs/rust/Cargo.toml --lib` 与 `cargo test --release --manifest-path nurbs-surface/rust/Cargo.toml`。
- 执行 `scripts/build-demo.ps1` 验证主项目读取、显示与 Typst 文章，并检查图像；`-IncludeModules` 另构建独立曲线／曲面教材与 WASM。
- 原生编译不替代 Typst 编译，非空输出不替代视觉验证。不替换失败样本或刷新基准掩盖失败。
- 保留已有未提交工作与无关文件；文档完成状态必须与可运行功能一致。
