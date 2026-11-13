# 渲染 #

## 1. 概述

渲染分为多个部分：

1. **总控 Render** - 包含大部分 GPU 资源，负责重绘
2. **相机 Camera** - 作为坐标系，同时包括了所有视图内的渲染控制，交互控制节点
3. **控制 RenderPrepareControl RenderDrawControl** - 闭包
4. **管线 ComponentPipeline** - 包含绑定组布局，管线布局，Shader 等，用于创建实例
5. **组件 Component ComponentInstance** - 包含绑定组和缓冲区

## 2. 嵌套

`Nest` 用于嵌套功能，也就是能够传播渲染指令，控制渲染裁切矩形

实际应使用包装过的 `Container`，会转发交互，并添加矩形背景

## 3. 重绘

重绘由 OS 发出的 `WindowEvent::RedrawRequest` 事件控制。

