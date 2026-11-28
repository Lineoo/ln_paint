# Android 技术栈关键问题 #

`winit-android` 使用了经过 patch 的代码。winit 上游在安卓这一块不是很活跃所以只能我自己做了。

技术栈：

- `jni` Rust 和 Java 互操作
- `android-activity` Rust 到 NativeActivity 胶水层
- `winit-android` winit 绑定
- `MainActivity` JVM 层代码

## 1. 运行模型

1. 两个线程：Java 主线程，`android_main` 线程。
2. 每个 Activity 实例调用一次 `android_main`；NativeActivity 被销毁重建（relaunch）时进程不变。
3. `NativeActivity` 自己会进行 `loadNativeCode`/`unloadNativeCode`

## 2. 问题清单

### 2.1. Destroy 事件和二次重建

现状：

**Destroy 事件会阻塞 Java 主线程**

1. 因为配置变更等问题 Activity 开始 OS 销毁流程
2. JVM 投递 Destroy 给 Native 线程并阻塞
3. winit 不对其进行特殊处理，仍然循环等待来自 JVM 的事件

平行参考：

- 在别的平台 "OS 要销毁" 总是等价于 "进程要没了"，所以 `winit` 只保证 drop 和停止循环，没有支持二次重建。
- 问题：Android 平台的 Activity 是会二次重建的。

解决方案：

- 既然 winit 不支持，那就尽可能不要让 Activity 重建，在 manifest 阻挡全部事件（缓解方案）
- 全量：`keyboard|keyboardHidden|navigation|orientation|screenSize|screenLayout|smallestScreenSize|uiMode|density|fontScale|locale|layoutDirection|colorMode|mcc|mnc|touchscreen|dpad`。

万一 Android 出于什么原因还是想要 relaunch Activity：

- winit 在得到后 Destroy 事件后至少得给应用发一次事件，至少可以保存一次数据
- 基于 winit Activity 被摧毁 = 进程要没了的假设，直接调用 `ActiveEventLoop::exit` 强制退出事件循环，至少不能让 Java 线程阻塞。
- 退出之后 Android 会尝试重建，第二次重建的时候直接 panic，这是对的。然后这之后就需要解决 panic 的问题。

需解决：

- 现在 lnwin 的 cleanup 强依赖 CloseRequested，类似的模型会导致 redb 忘记保存以及 GPU 设备资源的一些问题

### 2.2. panic

panic 后进程*不会正常 abort*

- lnpaint 自己内部代码 panic
- 被 android_main 外层的捕获，输出消息
- 发送 finish 给 JVM
- 不会停掉 JVM 所以 static 可能残留