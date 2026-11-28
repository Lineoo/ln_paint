# Val 数据绑定 #

注意数据绑定是 `lnpaint` 范围的。这是一个建立在 observer 系统上的数据流控制系统。

## 数据更新 ValChanged

`ValChanged` 提示这个 Element 可能部分或完全发生变化。注意是*可能*，也就是说有可能不变。

接收者：
- 一般用 `ValBind` 进行接收
- 假设这个节点的内容更新了

发送者：
- 在改变内容时发送
- 一般来说“发送就行了”，不要激进地节流
- 只针对文本这种性能开销大的进行节流

**复杂语义**

考虑一个 `Button` 结构包含字段 `rect: Handle<Val<IRect>>`

如果 `IRect` 变化了，那么**只触发 `Val<IRect>`**，`Button` 不应触发更新事件。

但如果是 `Handle<Val<IRect>>` 变化了，那么确实应该发送给 `Button`。（但实际上不应变更句柄！）

## 标准数据容器 Val

Val 用于存储任意数据，这些数据可以不是 Element，即使是 Element 也不会触发它们被添加时原本的生命周期钩子。

## 数据传递 ValBind

用于从世界中读取计算数据，添加监听 ValChanged 事件，然后得到一个 Val 容器。
