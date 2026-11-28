# 世界模块 #

世界模块提供**多元素响应**能力，这包括同时访问多个元素，用事件串联多个元素，以及多元素依赖处理。

设计思路源于 ECS 但**不是 ECS**，没有 Entity 以及其他能够联系 Component 成一组的能力，也没有 System 提供的高并行能力。

主要的功能是一个 observer 消息系统，顺便一脚干碎 Rust 的借用检查器。

## 独立 Element 写法 ##

独立 Element 是指那些可以脱离 World 存在的组件，一般给渲染组件管线等功能丰富的结构体使用。

特点是包含了一些 GPU 等外部的复杂资源，使用独立方法创建或者使用 `Descriptor` 功能。

实例：`camera.rs` 

## 简单 Element 写法 ##

这个写法更常见一些，用于大部分业务代码、渲染组件等。

特点是 Element 所有字段都是公开的，全部都是公共 API，外部调用者可以**直接构造**这些结构并直接插入世界。

- 内部实现发生在 `when_insert` 阶段，一般路由回结构体的 `init` 方法。
- 内部实现的内容，如 GPU 资源、或者嵌套元素返回的句柄，不保存在 `Element` 上，而是直接利用 observer 闭包捕获。
- 闭包捕获虽然已经足够满足绝大多数场景了，但是有些时候某些资源需要在不同过程之间共享；这个时候可以创建一个私有结构体，命名一般为 `*Instance`，并在 observer 捕获句柄，在需要的时候 `fetch` 得到需要的数据。

高度依赖 world 的 observer 系统来保存运行变量，可以说是针对业务逻辑代码特化的。

- 优点是针对 observer 进行了特化，事件调用简单，响应代码写起来很规整。
- 但缺点也是针对 observer 进行了特化，内部字段被封闭在 observer 系统内，业务逻辑基本只能依附于 observer 系统存在，单单一个结构体什么都做不了。

实例：`quad.rs`, `oklab.rs`, `desktop_ui.rs`, `text.rs`

## queue 借用展平 ##

`fetch` / `fetch_mut` 会得到一个 RAII 风格的守卫 `Ref` / `RefMut` 用于运行时借用检查。

为了防止冲突，优先使用 `queue` 或 `insert` 来展平执行，而不是使用 `drop` 或者代码块。

`queue` 的闭包或者 `when_insert` 调用有以下保证：

- 所有 Element 都已经插入，你刚刚调用的 `insert` 一定可以成功 `fetch`
- 借用检查完全为空（`when_insert` 的话，只有 `self` 被可变借用）
- 深度优先展开，也就是这个调用一结束，立马调用你刚刚入队的命令（无交错）

## insert & remove 生命周期 ##

我们遵循生命周期最小的原则：

- 在 insert 后无法 fetch
- 在 insert 命令 flush 之后正常 fetch
- 在 remove 后立刻无法 fetch
- 在 remove 命令 flush 之后当然无法 fetch

```text
insert            output `Handle<T>`
  insert-record
...
flush
  insert-with     (if you use `insert_with`)
  insert-index    #### fetch-available now
  insert-hook     ==> `when_insert`
...
insert-deps       *children* `when_insert`
...
remove            input `HandleGeneric`
  remove-deps     *children* `when_remove`
  remove-hook     ==> `when_remove` (immediately!)
  remove-record   #### fetch-unavailable now
...
flush
  remove-index
  remove-with     (if you use `remove_with`)
    drop          performed by Rust
```

## trigger 生命周期 ##

默认 `trigger` 调用是及时的

- 允许 zero-copy 和内部可变性等功能，但这种功能很少
- 因为触发的 observer 是未知的无法保证没有借用冲突，调用时应当没有任何借用
- 生命周期管理不当可能会触发可变性检查导致 panic 或出现*循环访问*导致栈溢出

队列 `queue_trigger` 延迟调用

- 不必操心声明周期，一般用这个，除非有特殊需求
- 额外对投递后失效做了宽容处理，也就是如果目标失效了静默跳过
- 不过需要事件 ownership 而不是 reference

结论：总是用 `queue_trigger`，别用 `trigger`

## 约定顺序 ##

`observer` 或者 `when_insert` 推荐以下顺序进行书写：

1. **scope** 顶头声明位置，设置依赖
2. **fetch** 抓取必要组件
3. **insert** 插入数据，节点
4. **observer** 添加必要的事件控制

不是强制的，有复杂控制流的时候怎么方便怎么来。

## trace ##

使用一个简单的标签系统来标记执行位置。

- `init` 来自 World 默认状态，一般只有 `Lnwin` 能接触
- `foo |` 表示 foo 调用 queue 进行展平后的 flush 过程内，且没有给予新标签
- `foo Handle` 表示在 foo 阶段插入节点时
- `foo {Handle}` 表示在 foo 阶段使用 `insert_with` 实例化节点时
- `bar ~Handle` 表示在 bar 阶段移除节点时
- `bar ~Handle ~Handle` 表示在 bar 阶段移除节点时，通过 dependency 移除依赖子节点时
- `bar ~{Handle}` 表示在 bar 阶段使用 `remove_with` 回收节点时
- `foo .[baz]<PointerEvent>Handle` 表示在 foo 阶段传递 `PointerEvent` 触发了一个来自 `baz` 阶段添加的 observer
- `foo | .[baz]<PointerEvent>Handle` 表示 `PointerEvent` 是通过 `queue_trigger` 传递的

Handle 的显示格式是 `Handle[trace]<type>({index}v{gen})`

## Storage - Validation - Cache 模型 ##

`ln_world` 设计上就不是零运行时开销的；相反，其内部可变性模型导致其会在运行时进行非常多（无效的）借用检查。

运行时检查目前关不掉，未来可能会引入一个关闭线程内检查的 feature flag，用潜在内存错误换更好的性能。

但是运行时检查是可以最大程度规避，甚至利于多线程的的：

- 我们可以缓存线程内世界借用状态来进行主动检查缓存，在同样的业务代码位置提高缓存击中（主动分支预测）。
- 也可以通过显式的 GhostCell 模式获取类型令牌来零开销线程并发（主动遍历缓存）。

就可以得到一个安全、高效、可追踪的世界模型啦 :D