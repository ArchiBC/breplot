# 3dm 回归样本

## 官方 Logo：纯 Rust 解析目标

`logo.3dm` 是 [openNURBS 8.x 官方 NURBS Logo](https://github.com/mcneel/opennurbs/blob/8.x/example_files/V7/v7_rhino_logo_nurbs.3dm) 的原始副本，2026-09-30 获取，未经过另存、类型转换或修复。该目录最新 NURBS Logo 是 V7；另有 SubD Logo，不属于本次范围。

- 字节数：882441；Git blob：`eaf017585a1fbdabec2bc69b862c180f74f39e0c`。
- SHA-256：`854730269B0FCAEEC41CA29122936C05ADCC2B94BAD678C4C72E7480CF865FCC`。
- 固定内容链接：[Git blob](https://api.github.com/repos/mcneel/opennurbs/git/blobs/eaf017585a1fbdabec2bc69b862c180f74f39e0c)。来源许可见随附 [openNURBS license](LICENSE.openNURBS)。
- 6 个 B-Rep，22 面、35 边、30 顶点、32 环、79 trims。支持曲面和复合曲线中的 NURBS 别名，不依赖显示网格。
- 格式研究参照官方 [opennurbs_brep_io.cpp](https://github.com/mcneel/opennurbs/blob/eb92af3ba1806b0a34a99aba0d3bda83e3d46083/opennurbs_brep_io.cpp)、`opennurbs_archive.cpp`、`opennurbs_nurbscurve.cpp`、`opennurbs_nurbssurface.cpp`、`opennurbs_polycurve.cpp` 与 `opennurbs_object.cpp`，Rust 仅实现所需解码子集。

## 保留的历史输入样本

早期自行构造并保存的 Rhino 8 / archive 80 文件，单位毫米。没有用户模型。使用 Rhino3dm 8.35.0 的几何构造/写入功能制作，现已固化为输入样本；测试及显示不需要样本生成器、.NET、Python 或 Rhino。

| 文件 | 几何与断言 |
|---|---|
| box.3dm | (0,0,0) 至 (2,3,4)，6 面、12 共享边，法向向外 |
| sphere.3dm | 原点球心、半径 2，有理权重保留 |
| cylinder.3dm | Z 轴圆柱，半径 2、高 4，带两个裁剪端盖 |
| hole.3dm | XY 平面外框 [-3,3]²，中心圆孔半径 1，面积约 36-π |
| assembly.3dm | 箱体 [-3,-1]×[-1,1]×[-1,1] 与球心 (2,0,0)、半径 1 的球；验证边 ID 按对象偏移 |
| unsupported-point.3dm | 原点对象，要求显式拒绝 |
| invalid-hole.3dm | 失败构造保留：将外框与内圆交给单环 CreateTrimmedPlane 后，edge.m_vi 与闭合性不匹配；要求拒绝 |

正确圆孔用外框先建平面，再显式添加 Inner 环。失败样本保留原始归档，不能以自动修补后样本替代负例。测试不会重写任何样本或刷新基准。官方 DLL 适配器及其测试已删除；纯 Rust 回归目前使用 logo.3dm、sphere.3dm（旋转面应整对象跳过）和 unsupported-point.3dm（点对象应报告跳过）。其余历史输入保留，表中描述不是当前支持声明。
