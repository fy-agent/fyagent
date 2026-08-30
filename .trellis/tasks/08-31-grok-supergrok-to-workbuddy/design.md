# Design — SuperGrok to WorkBuddy

先读父任务 [summary.md](../08-31-grok-first-class-iteration/summary.md)。事实见 `research/current-workbuddy-save-path.md`。合同见 `.trellis/spec/backend/workbuddy-configuration.md`。用例见 [use-cases.md](./use-cases.md)。

本子任务不管登录路标，不管 Claude / Desktop / Codex 的 Provider 绑定。

## 边界

WorkBuddy 不是 `AppType`，不是 Provider。保存只有这一条：

`create_workbuddy_save_plan` → `workbuddy_models_save` → `apply_change_plan(planId, planDigest)`

请求形状已经定死：`base_url` + `api_key` + 模型 ID + revision / overwrite token。公开计划和日志必须没有钥匙。

不要走 Codex upsert。不要第四个执行器。不要把 WorkBuddy 改成 Provider。

## 和 SuperGrok 怎么接

1. 没登录：指向认证中心，不假装已经写进 WorkBuddy。
2. 已登录：用现有 `get_xai_oauth_models` 拉模型名单，填进现有 WorkBuddy 预览。不要再扫一次码。
3. **禁止**把 OAuth 刷新令牌抄进 `{trusted-home}/.workbuddy/models.json`。令牌会过期，也是把托管秘密复制到另一家软件的文件里。
4. WorkBuddy 运行时读自己的文件。能少填一把钥匙就少填；文件格式做不到，就在预览/亲测里写明卡在文件格式，不要谎报「已经 OAuth 绑定」。

## 数据流

```text
认证中心 xai_oauth
  →（可选）get_xai_oauth_models
  → WorkBuddySavePlanWorkspace.createWorkBuddySavePlan
  → apply_change_plan
  → models.json 回读（get_workbuddy_status / model ids）
```

UI 主人：`src/v2/pages/models/Page.tsx` 的 WorkBuddy 面板 + `WorkBuddySavePlanWorkspace.tsx`。

## 兼容

修订、覆盖确认、并发修改、备份路径，继续遵守 `workbuddy-configuration.md`。不要改 MCP / Skills 那条线。

## 回滚

只撤「用已登录账号拉名单 / 生成预览」的缝合。不要拆现有 WorkBuddy 保存。
