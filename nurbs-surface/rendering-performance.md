# 曲面显示性能探索

当前默认选择高清曲面 PNG + 矢量线条，优先保证文档阅读流畅。完整 SVG 继续保留。该方案只改变文档承载方式，不改变曲面数据、求值、法向或结构线遮挡。

## 已测量

| 路径 | 整份 13 页 PDF 字节数 | 说明 |
|---|---:|---|
| 原全矢量 | 61,985,308 | 每个着色小片各有渐变、矩形、裁切、分组 |
| 直接路径填充 | 58,529,892 | 去掉小片矩形、裁切和分组，收益有限 |
| 默认混合 | 3,588,572 | 曲面 PNG，线条继续矢量 |

PDFium，scale=2，第 13 页三次渲染中位数：原版 0.953 秒，混合版 0.042 秒。未测用户阅读器 UI 的加载、拖动和缩放延迟。记录在 target/performance/timings.json。

## 更合适的矢量后端：PDF Type 4

SVG 当前输出每个小片一个一维 RGB 渐变；三个顶点的最终颜色一般不共线，必须再细分来压低颜色拟合误差。PDF Type 4 直接存储三角形位置和各顶点 RGB，既能减少对象，也无需单轴颜色拟合。仍需要细分逼近非线性法向光照，不能把 Gouraud 着色称为逐像素 Phong。

标准依据：[PDF Association 色彩速查表](https://pdfa.org/wp-content/uploads/2023/08/PDF-Color-CheatSheet.pdf)。Typst 现支持 PDF 图片：[官方说明](https://typst.app/docs/reference/pdf/)。本机已实际编译并渲染嵌入的 PDF，未仅根据文档推断支持。

可复现原型（仅 Python 标准库，不新增生产依赖）：

```powershell
python nurbs-surface/scripts/probe-pdf-mesh.py
typst compile --root nurbs-surface nurbs-surface/target/performance/mesh-probe.typ nurbs-surface/target/performance/mesh-probe-typst.pdf
```

原型使用现有 sphere 网格、两轮统一着色四分、16 位坐标和颜色，一个压缩 mesh stream。15,360 三角形，直接 PDF 为 120,138 字节，Typst 嵌入后 122,804 字节。PDFium scale=1.5 单次渲染约 0.065 秒；这与整页基准条件不同，不能直接算加速比。

限制：只验证不透明球面，球极点法向采用球面专用径向回退；平均深度排序沿用旧限制；没有正式误差验收、透明度合成、裁剪/共享边回归和完整阅读器矩阵。原型不进入默认构建，不可当生产后端。

## 后续方案比较

- PDF Type 4：最值得继续做的原生矢量方案；下一步应迁入 Rust，复用法向和颜色误差细分，保留三角形共享边，再验证半透明分组及遮挡。
- 原生逐像素着色：长期最可控的混合输出方案；自己实现深度缓冲、归一化法向插值和抗锯齿，直接产出图片。能避免当前先生成大 SVG 再烘焙的构建成本，但需要新增实际渲染器，当前没有实现。
- 放宽 SVG 颜色容差：会减少片数，但容易重新出现用户已指出的分界，不作为默认性能优化。
- 仅压缩坐标文本：可减 SVG 字节，不能显著降低 PDF 阅读器的渐变绘制次数。


## 整份文档的内存接口验证

后续已实现 Rust Type 4 输出及 WASM 案例桥接：`build-type4.ps1`。全部 13 页图例都走同一内存 PDF 字节调用，不依赖插图中间文件。隔离目录验证：没有 target 目录，编译只新增 article.pdf，耗时约 4.8 秒。

最终 main-type4.pdf 为 8,309,858 字节，无位图对象，含 1929 个 Type 4 着色对象（网格线的绘制顺序及透明父三角形导致分组）。PDFium 第 13 页 scale=2 三次中位数 0.015 秒。当前没有压缩 mesh stream，坐标升级为 32 位，RGB 为 16 位。

该实现复用 SVG 几何、法向、着色探针和结构线遮挡；WASM 仅暴露案例 ID 接口，通用曲面输入编码没有实现。半透明父三角形使用隔离透明组，在组内不透明绘制小片，但实际 PDFium 渲染仍有三角形接缝；不透明面部分位置有微小裂点。它验证了文件体积与 API 可行性，未达到默认替换混合输出的视觉质量。


## 2026-09-30：依据规范完成的优化（当前版本）

核对 Adobe 发布的 [ISO 32000-1:2008 原文](https://raw.githubusercontent.com/adobe/dc-acrobat-sdk-docs/master/docs/standards/pdfstandards/pdf/PDF32000_2008.pdf)：8.7.4.5.5、表 82（印刷页 189–192）规定 Type 4 顶点记录与共享边；8.7.4.3、表 78 规定 AntiAlias；11.6.4 的形状/不透明度模型约束透明合成。未把 AntiAlias 当作保证消缝的开关。

- 顶点记录是高位在先的 flag、坐标、颜色，逐顶点补齐字节；保留 32 位坐标和 16 位 RGB，避免以降低精度换体积。
- flag 0 开始三角形，后两条记录的 flag 被忽略；flag 1 使用上一三角形的后两个顶点，flag 2 使用第一个和第三个顶点。只在编码后的坐标与颜色完全相同时复用。原三角形的深度顺序保持不变，只调整其内部互不重叠子片的访问顺序。
- Type 4 直接按顶点 RGB 插值，不需要 SVG 单轴颜色拟合。相同 0.001 光照采样阈值下减少了过度细分，并添加独立内部采样回归。
- 投影内部不重叠的连续父三角形可以批量绘制；遇到投影重叠或网格描边立即结束当前批次。前后面保持独立透明合成，不把整个实体一次淡化。
- 网格流使用 FlateDecode。零依赖编码器采用 [RFC 1950](https://www.rfc-editor.org/rfc/rfc1950) zlib 外壳及 [RFC 1951](https://www.rfc-editor.org/rfc/rfc1951) 固定 Huffman/LZ77；压缩无收益时存原流。使用 Python 标准 zlib 独立解码 1224 组随机/重复/窗口边界数据，逐字节一致。

整份 13 页 PDF 从 8,309,858 字节降到约 1.82 MB（文档文字更新会轻微改变字节数）。审计为 0 个位图、1059 个 Type 4 流、70779 个着色三角形；其中约 28000 个三角形使用共享边续接。`python nurbs-surface/scripts/verify-type4.py` 可以重新检查最终文件的 Flate 数据和网格记录。

剩余问题：PDFium 下透明面仍可见细线接缝，不透明面仍有少量微小裂点，不能称为完全修复。后续应优先实现相邻自适应小片的共形划分、排查 T 形连接以及阅读器扫描转换差异，而非用重叠透明片掩盖缝隙。Type 5 适用于规则格网，裁剪后不通用；Type 6/7 表达二维曲边色块，不会自动替我们求 NURBS 法向、光照或三维遮挡。


## Edge / Adobe 色彩兼容修复

Edge 实测确认：将 Typst 嵌入 Type 4 插图时添加的 ICC 透明组混合空间改为 `/DeviceRGB` 后，完整文档恢复正常蓝灰色。正式 `scripts/build-type4.ps1` 已接入 `normalize-pdf-groups.py`，需要 Python + PyMuPDF（`python -m pip install PyMuPDF`）。此依赖仅用于最终 PDF 的兼容处理，Rust 几何/着色核心仍无第三方依赖。

处理保留组隔离和透明度，仅修改带着色资源的 Form 的 `/Group/CS`；写回前逐一检查所有解码数据流未变、页数未变。全程在内存处理最终 PDF，不生成逐图中间文件。直接运行 `typst compile` 不包含此修复，应通过构建脚本导出。已确认版本约 1.84 MB、13 页；透明排序和接缝的既有限制仍适用。

诊断注意：旧 `edge-group-diagnostic.pdf` 的 A/B 在 PyMuPDF 拼版时丢失了页面透明组，实际上与 C 一样没有组，不能据其标签推断 ICC 正常。`diagnose-edge-groups.py` 已改为显式添加 Form 组并断言，输出 `edge-group-diagnostic-v2.pdf`。
