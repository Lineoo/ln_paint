# Widget 组件 #

- renderer 基础渲染类
  - 包含至少一个 RenderControl
  - 不包括任何 collider
-  widget 组件类
  - 一定包括至少一个 Renderer
  - 至少包括一个 collider
  - 可复用于不同逻辑
- section 片段类
  - 包含具体业务逻辑
  - 一定使用 container
  - 大概率使用 layout
- layout 基础排版类
  - transform 和 luni
