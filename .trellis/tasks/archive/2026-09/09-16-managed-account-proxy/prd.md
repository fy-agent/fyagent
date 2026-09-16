# 托管账号本地代理链路

## Goal

参考用户提供的 Cherry Studio 2.0.14 源码，将 FyAgent 已登录账号的本地代理链路做成可验证的完整业务：代理用途登录 → SecretRef 凭证 → 本地监听 → 目标 Agent 配置 → 请求时鉴权/协议适配 → 流式响应。原生账号直接写配置的方式保持不变。

## Background and evidence

- 输入为 `cherry-studio-2.0.14.zip`；版本及 SHA-256、源码位置与官方调研见 `research/cherry-comparison.md`。
- 当前 `services/managed_auth/service.rs::upsert_proxy_connections` 根据默认凭证就写入 Connected，不能证明监听或消费端配置有效，并可能留下失效默认账号的旧连接。
- 当前 `services/provider/managed_xai.rs` 仅提供 xAI → Claude/Codex/Desktop 的绑定；缺少 OpenAI 代理账号与 Grok Build 的同等接入。
- 当前 `proxy/providers/codex.rs` 强制 xAI 转 Chat，导致已有 native Responses namespace/sanitize 分支不可达；与参考源码 Grok CLI Responses 路径不一致。
- 现有 SecretRef、ProxyService、Provider transaction、Change Plan、SSE converter 和配置备份足以承载迁移，不需引入 Electron 或第二个服务。

## Requirements

### R1 — 代理用途与原生用途隔离

只能绑定 `proxy_upstream` 且 `refresh_owner=fyagent` 的 OpenAI/xAI 凭证。原生 Codex/Grok/OpenCode 登录、令牌归属、账号直接投影及其确认流程不重写。不得向 Agent 文件、Provider 配置或 renderer 暴露上游 OAuth access/refresh token。

### R2 — 统一的目标接入

为 Claude Code、Codex、Grok Build 复用已有 Provider 保存/激活流程提供托管账号代理入口。公开身份由后端解析，不依赖用户手填 legacy ID。Codex 继续保存草稿并经既有 Change Plan 预览/确认；其他目标复用已有配置披露及回读。保留旧 xAI 命令及 Desktop 行为兼容性。

### R3 — 请求契约

OpenAI 与 xAI 代理账号使用各自固定的官方订阅源。Responses 消费端直接走 Responses；Claude Messages 复用既有双向转换。请求时解析当前凭证，统一执行提供方要求的请求体/头整形，保留工具调用、流式终止、用量与错误语义。

### R4 — 刷新与失效

每个凭证独立串行刷新。失效 grant 标记需要重新认证；暂时网络/限流失败不能注销账号。刷新结果必须属于仍然有效的原凭证世代，不能复活已删除/重新登录/交给原生客户端的会话。上游 401 允许在同一账号内强制刷新并重试一次，不切到其他账号或付费 API。

### R5 — 运行与失败状态真实

账号就绪、代理监听、目标配置采用是不同事实。不得仅因登录成功显示整条代理链路已连通。激活先绑定监听再发布配置，使用 loopback；保留既有备份、并发锁与补偿，停止/端口冲突/失效账号不得返回假成功。

## Acceptance criteria

- AC1/R1: Native consumer/login 文件路径与非代理行为未改变；测试证明原生会话不能作为代理源，代理激活保留原生 auth/MCP/无关配置。
- AC2/R2: 两类代理账号可生成三种目标的正确源定义；renderer → typed Port → command/ACL → Provider/ProxyService 链路有回归测试。Codex 的保存不等于激活。
- AC3/R3: mock vault + 真实 loopback listener 的集成测试证明官方 host/path 在网络替换前正确，Bearer/账号路由头正确，Responses 与 Claude SSE 工具调用及终止事件可消费。
- AC4/R4: 过期、并发刷新、401 一次重试、terminal/retriable、凭证世代变化及原生 owner 拒绝有测试；无隐式账号或 API-key fallback。
- AC5/R5: 默认账号缺失/变更、监听未运行、激活端口冲突、配置回读和恢复失败均有准确结果；不停止其他已采用同一监听器的目标。
- AC6: 更新职责对应的 SPEC，完成当前宿主机质量门禁及独立 diff 评审后提交、归档并记录 journal。Mock 联通不宣称真实订阅权益/额度或 Windows 真机验收。

## Out of scope

重写原生登录/配置投影；复制 Electron/Bun 网关；新增 daemon、Docker、云代理或系统代理；开放公网监听；新增账号提供方；更改 API-key 业务；自动调用真实账号消耗额度；猜测账号模型权益；无关页面改版。
