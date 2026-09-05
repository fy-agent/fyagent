# 入口与状态所有权复核

## 实际边界

目录保留原来的安装/更新/启动动作，配置内页不再挂第二个安装器。账号管理的软件连接详情复用共享 Change Plan 视图进行已有 Codex 来源切换；模型页仍然编辑、测试和保存并启用本次配置，通过闭合集合路由跳转到账号页。没有改变侧栏和目录布局，没有更改原生 DTO、秘密保管或写入协议。

## 复核与修复

- 共用 apply 视图、view model、错误映射和 Query job observer 提升到 shared/features/change-plans-ui；Models 保留专有保存控制器。
- 新 currentId 回读不能把刚完成的 job 隐藏。终态必须读回 Provider 和 Auth 两个 owner；失败仅可重试读取，不重发写入。
- source 操作未确认时阻止其他账号修改；未知 admission 保守锁定；后台读失败保留旧面板和操作身份。
- 双向一致性：账号切回官方及登录完成时使 Provider/OpenCode 查询失效，不能保留旧来源选择列表。
- 隐藏面板保持 React 状态但停自动查询；全部重复提交、失败读取、返回上下文均有回归。
- 使用 useId 避免 Models/Auth keep-alive 同时挂载时产生重复可访问性 ID。

官方复核依据：React lazy/Suspense（https://react.dev/reference/react/lazy），TanStack Query 的禁用、取消及定向 invalidation（https://tanstack.com/query/latest/docs/framework/react/guides/disabling-queries 、https://tanstack.com/query/latest/docs/framework/react/guides/query-cancellation 、https://tanstack.com/query/latest/docs/framework/react/guides/query-invalidation）。保留成熟库，不自建后台轮询调度器。

## 验证

最终 V2 类型与 lint 通过，519 项 V2 测试通过；浏览器全量168项通过，加入 Models/Auth 往返后专项8项（两用例 × 四视口）通过。反向失效已纳入AuthPage回归。完整 prearchive 退出0，覆盖根前端与Rust全量及contracts。真账号登录、系统凭据、原生签名/安装不由浏览器 fixture 证明。
