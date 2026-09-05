# 设计约束

在前两个子任务完成后实施。首屏就绪从外壳挂载移动至实际路由内容提交；保留原有深链接监听准备事件的语义，必要时分离呈现就绪与接收激活。native 继续 hidden 创建，只改变正常启动 show 的 owner；必须设计就绪竞争、静默启动与失败兜底，禁止用固定延迟当作正常成功判据。

首个路由加载优先，其他模块非阻塞预取并处理拒绝。不一次性 eager 打入主 bundle，不等待所有目录扫描、远端模型或账号网络请求。不改变 IPC/秘密/安装语义。

## 源码复核后的实施方案

- 复用原生 ActivationInbox，不创建第二个窗口/调度框架。正常启动、托盘显式唤起、macOS reopen 和轻量模式重建统一将 Focus 放入已有队列。队列同时等待窗口几何准备与 renderer-ready；原有解析后的深链接语义、容量及去重策略保持。
- 保留 `frontend-deeplink-ready` 事件协议，只把 V2 的发送点从外壳提前挂载移到实际路由内容提交。默认 Agents 额外等待本地目录查询结束（成功/明确错误均可呈现），Auth 等待首个 overview；不等待目录扫描、远端列表或后续后台刷新。
- 初始 hash 只用于闭合主路由模块选择。先 await 当前模块，再 render 并预取其余模块；后台预取拒绝被消化，真实进入失败仍交给可重载的错误页。不开 eager bundle，不制造延迟动画。
- 隐藏 WebView 可能节流 rAF，因此就绪依据 React commit/effect，不以 rAF 或 document.visibilityState 作为显示前置条件。
- 正常成功路径没有定时显示。原生 Tokio 15 秒看门狗只在有待唤起请求、仍未 ready 时给出一次原生错误对话框，允许显式重载或稍后处理；不冒充成功/直接展示加载页。使用代际标识淘汰被 reload/销毁取代的等待，静默启动没有唤起请求不弹框。数据库恢复仍走已有强制显示分支。

官方依据（2026-09-05复核）：https://v2.tauri.app/learn/splashscreen/ 对隐藏主窗口和前后端就绪的职责说明；https://react.dev/reference/react/Suspense 与 https://react.dev/reference/react/lazy 对 suspend/commit 和模块缓存的语义。采用已有库和应用队列，示例中的固定 sleep 不用作成功条件。

集成发现目录已提交不等于品牌图片已解码（浏览器naturalWidth曾为0）。共享Brand/BrandIconFrame标记本地首屏图片，ready前使用原生HTMLImageElement.decode等待当前已挂载、非hidden/非lazy的同源或data装饰图；排除远端资源，不遍历未来页面。解码失败不隐藏可用界面，卸载/隐藏后的迟到结果作废。并同步测试为等待解码后再断言图片的固定尺寸，不削弱48px资源断言。依据：https://developer.mozilla.org/en-US/docs/Web/API/HTMLImageElement/decode 。
