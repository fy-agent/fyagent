# 设计

## 已有能力与缺口

主线已有 OpenAI/xAI 精确账号准入、SecretRef 托管、刷新、协议转换，以及 Claude Code/Grok Build 原生配置应用；Codex 经已有 Change Plan 确认。OpenCode 原生认证拥有独立 credential purpose，不能用复制其他 purpose 的刷新令牌代替跨工具转发。

## 选择

扩展现有 FyAgent 本地代理到 OpenCode。保持一个监听器、一个凭据解析/刷新 owner、独立 OpenCode provider namespace 和请求路由。复用现有 Responses 转换和流处理，不增加依赖或第二个执行服务。

OpenCode 绑定使用独立的窄命令/请求：`bind_opencode_managed_proxy({request:{accountId,modelId,expectedRevision}})`。返回 `{providerId,providerName,app:"opencode",alreadyBound,activated:true}`，沿用现有订阅关闭错误集合；过期 revision 返回 `provider_conflict`。不改变现有三目标命令形状。

后端通过托管认证 owner 解析用户显式选中的账号，复用稳定 Provider 构建与现有事务补偿能力。OpenCode 用户配置读写仍使用其固定路径、配置锁、revision、原子写入和恢复 owner；仅变更 FyAgent 管理的 provider/model 及显式选择的默认模型，保留未知字段、MCP、其他 provider 和原生认证文件。

前端复用现有订阅账号/模型选择区，在 OpenCode 面板挂载。应用前展示原生配置目标，捕获 revision；应用后回读 OpenCode snapshot 的 Provider/模型/默认选择及托管账号概览，失败或未知状态保留目标级写入阻止。订阅配置存在时，普通 API Key 写入暂停；用户通过明确的恢复按钮调用现有目标恢复 owner，回读确认后继续原 API Key 流程。

订阅 IPC 解析收拢到按需加载的 managedSubscriptions 模块，保留 literal IPC、严格参数和关闭错误集合。明确登记第二个 deferred port，不提高现有首屏与路由体积预算。

## 协议与边界

OpenCode 使用官方支持的 SDK provider 配置连接独立 loopback 路径。请求携带本地占位符；真实 OAuth token 只在转发前于后端解析。目标路由必须选择 OpenCode 自己的有效 Provider，不能借用 Codex 或 Grok 的 current provider。订阅失败不自动切 API Key 计费。

应用、停止恢复、崩溃恢复、并发和回读纳入现有代理生命周期。对不认识或外部已变更的内容失败关闭，不能通过重写空模板获得成功。
四目标重启均重新进入托管事务，避免通用接管回填原 API Key。恢复归属依据本次实际原子写入回执，覆盖再次绑定、备份 sidecar 和异步提交前的外部修改；不能重新采样当前字节并冒认本次写入。

## 验证与交付

隔离临时 home + 假上游验证真实本地监听器和配置读写、双来源、目标隔离、非流/流/工具请求、失败恢复与秘密不外泄；前端检查入口、确认、回读和目标切换。它们不替代真实订阅额度、厂商模型可用性或目标客户端实测。

保持已有界面样式、组件、token 和交互，无视觉重设计。代码与验证提交独立工作分支；不合并或发布。
