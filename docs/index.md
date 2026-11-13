# 索引 #

不全，以代码库为准。如果代码库不一样可能是正在进行重构。

### `ln_paint/` 主软件

- `build/`
    - `android/` 包含给安卓用的 gradle 和 Kotlin 代码
    - `appimage/` 构建 AppImage 的资源
- `res/`
    - `interface/` 交互图标，来自 [Lucide Icons](https://lucide.dev/).
    - `logos/` LnPaint 的 Logo
- `examples/` 渲染示例，主要用于基线回归测试
- `fonts/` 预嵌入安卓的字体
- `lang/` 本地化
- `src/lib.rs` 库入口
- `src/bin/ln_paint.rs` 桌面端入口

### `ln_paint/src/` 主软件代码

- `ui/`
    - `renderer/` 基础渲染
        > - 包含至少一个 RenderControl
        > - 不包括任何 collider
        - `quad` 矩形基础
        - `rrect`
        - `text`
        - `wireframe`
        - `canvas`
        - `vtor`
        - `brush_preview`
    -  `widget/` 可复用组件
        > - 一定包括至少一个 Renderer
        > - 至少包括一个 collider
        > - 可复用于不同逻辑
        - `palette/` 调色板
            - `hsl`
            - `oklab`
        - `container/` 容器
            - `tabs`
        - `area`
        - `button`
        - `panel`
        - `slider`
    - `section/` 业务片段
        > - 包含具体业务逻辑
        > - 一定使用 container
        > - 大概率使用 layout
    - `object/` 自由物件
        > - 类似 section 但是主要用于放在画布内
        - `color_snip`
    - `layout/` 基础排版
    - `common` 通用属性
    - `i18n` 本地化
- `paint/` 绘制系统
    - `brush/` 笔刷引擎
        - `param` 笔刷参数
        - `blur`
        - `pixel`
        - `round`
        - `smudge`
        - `tint`
    - `edit/` 编辑引擎
        - `merge`
        - `copy`
        - `clear`
    - `mipmap.wgsl`
    - `render.wgsl`
    - `readback.wgsl`
    - `traveler` 历史记录
    - `chunk` 绘制单元
    - `layer` 图层系统
    - `stream` 加载系统
    - `wrapper` 总汇
- `input/` 输入系统
    - `collider` 核心交互体
    - `focus`
    - `mouse`
    - `pointer`
    - `timer`
    - `touch`
- `color/` 颜色系统
    - `hsl`
    - `oklab`
    - `srgb`
- `save/` 持久化系统
    - `chunk`
    - `object`
- `render/`
    - `camera` 相机
    - `nest` 嵌套
    - `export` 无头导出
- `shaders/` 着色器编译
- `unit/` 单位

### `ln_world/` 管理系统

- `src/lib.rs` 主代码

### `docs/` 文档

- 具体请见 `AGENTS.md`。

### `scripts/` 脚本

- `gen/` 测试数据复现
- `hooks/` 推荐的 Git 钩子