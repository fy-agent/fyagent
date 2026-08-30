# 用例 — 登录三分法

下游按这条改、按这条测。双机勾选表在父任务 `research/hil-matrix.md`。

## UC-L1 官方登录只开门

- 对应 AT1、H1
- 人：新界面打开 Grok → 点登录
- 期望：终端出现 `grok login`；界面是「已交给官方认证入口」；**没有**「认证结果已验证」「已登录」
- 锁：`AgentAuthStatusPanel` 测试 + `agents-v3.spec.ts`

## UC-L2 官方退出也不验证

- 对应 H2
- 人：同一页点退出
- 期望：终端 `grok logout`；仍不说已验证

## UC-L3 扫码去认证中心

- 对应 AT2、AT4、H3
- 人：要 SuperGrok 扫码，或看 Codex 认证区
- 期望：下一步指向认证中心；不叫人去终端扫码；Agent 页不启动 `auth_start_login`
- 过期：指回认证中心，不是 `grok login`

## UC-L4 模型页只填钥匙

- 对应 AT5、H4
- 人：打开模型页 Grok Quick Setup
- 期望：只有 API 钥匙；没有 `grok login` 说明书
- 空草稿没动手：不报错。没改草稿则 #141 B7 = `not touched`

## UC-L5 Claude 验证还在

- 对应 AT3
- 人：打开 Claude Code 认证
- 期望：原来能「认证结果已验证」的路还在；不要被 Grok 的 handoff 文案污染

## 本窗口不做

H5–H8（投放和 WorkBuddy）。ChatGPT 登录。
