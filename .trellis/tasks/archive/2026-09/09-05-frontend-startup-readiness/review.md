# 首屏呈现评审

## 根因与方案

正常 setup 直接 show，而 V2 外壳的 effect 提前发送就绪；两处都会放大懒加载时的空壳。改为首个静态路由模块优先加载、内容提交后发送原事件、已有原生激活队列同时等待几何准备。主窗口仍由原生创建和显示，不引入 splash 页面或第二个窗口状态库。

Agents 等本地 catalog、Auth 等本地 overview 成功或明确失败；不等待扫描、远端模型或全部页面预取。渲染失败页能重载。可选模块预取拒绝不会变成未处理错误；不承诺浏览器失败模块缓存能无重载恢复。隐藏 WebView 不依赖 rAF/visibility 才能发 ready，避免循环等待。

静默启动不加 Focus，托盘/Dock/轻量重建复用同一队列。15秒仅用于原生失败对话框，从不授权 show。回调按load generation检查，旧回调不重载已就绪或新建窗口。数据库恢复分支保持原有强制显示，不改变凭证/后台任务。

## 复用与依据

- https://v2.tauri.app/learn/splashscreen/：前后端启动准备与隐藏窗口分离；不照搬示例sleep作为成功标准。
- https://react.dev/reference/react/Suspense ：suspend树未提交，fallback不代表真实内容就绪。
- https://developer.mozilla.org/en-US/docs/Web/API/Window/requestAnimationFrame ：隐藏页面帧回调可能暂停，不能作为native初次show的前置条件。
- Tauri插件dialog非阻塞回调、Tokio以及现有ActivationInbox承担机制；不引入新依赖。

## 验证与边界

已执行：原生Rust全量（包括三项新队列/看门狗状态测试）通过；V2类型/lint通过，529项测试通过；首屏与激活根契约8项通过。慢模块/中断模块的浏览器专项8项通过，最终新增可选预取失败用例后执行完整浏览器和build/prearchive并复核。

全量发现一个图片尺寸用例在资源完成解码前读取naturalWidth，实际值0。未降低固定资源尺寸断言：测试等待decode，共享启动ready也等待明确标记的本地首屏装饰图；隐藏/远端/lazy图不阻塞，解码失败允许显示可用界面。新增两项解码/迟到结果回归后V2为531项通过。Clippy要求折叠原生错误恢复的条件，按等价逻辑修正，没有增加allow或禁用警告。

此前全量V2偶现取消脏表单后再次弹窗，局部复跑通过但不作为已解决证明；父任务集成复核将验证共享Dialog焦点返回到自动激活tab的行为并修复。这不改变启动方案，但属于本轮必须闭环的交互质量问题。

当前未执行Windows/macOS真实主窗口首帧、Dock/UAC、真实系统凭据与正式安装签名验证。不把portable状态机和浏览器模拟数据当作这些证据；不编造毫秒性能提升。

最终验收：531项V2单测、184项浏览器、renderer/route-chunk/standalone构建全部通过；完整prearchive退出0，含根单测1620通过/1既有跳过，Rust3472通过/6既有忽略，Clippy/格式/契约全部通过。三个已审查native文件更新对应结构摘要，无新豁免或阈值降低。单文件预览的打包体积提示保留，没有改变生产路由分包或调高告警阈值。
