# Implement — SuperGrok to WorkBuddy

先读 [design.md](./design.md) 和 [use-cases.md](./use-cases.md)。对齐源：父任务 [summary.md](../08-31-grok-first-class-iteration/summary.md)。

依赖：人要先能在认证中心扫码（登录窗口的路标）。写入本身不依赖 Codex 窄口。

## 开工顺序

1. 读 `research/current-workbuddy-save-path.md` 和 `.trellis/spec/backend/workbuddy-configuration.md`。
2. 没账号：WorkBuddy 模型页指向认证中心。
3. 有账号：用 `get_xai_oauth_models` 填模型名单，再走 `create_workbuddy_save_plan`。
4. 钉住：预览是 `workbuddy_models_save`；单子和日志没有刷新令牌；失败不连坐 Claude / Codex。
5. 亲测 UC-W1–W3。回写 #42 / #106。

## 会碰到的文件（先读再改）

| 文件 | 为什么 |
|---|---|
| `src/v2/pages/models/Page.tsx` | WorkBuddy 面板、拉模型、生成预览 |
| `src/v2/pages/models/apply/WorkBuddySavePlanWorkspace.tsx` | 预览/确认 |
| `src/v2/shared/features/change-plans.ts` | `workbuddy_models_save` |
| `src-tauri/src/services/workbuddy/types.rs` | `SaveWorkBuddyModelsRequest` |
| `src-tauri/src/commands/xai_oauth.rs` | `get_xai_oauth_models`，不是登录 |
| `src-tauri/src/proxy/providers/xai_oauth_auth.rs` | token 只留在这里 |

不要改：Codex `prove_codex_target_credential_capability`、Claude Provider 绑定、把 WorkBuddy 加成 `AppType`。

## 自动检查

- WorkBuddy 现有保存 / revision / 脱敏测试必须继续绿
- 新增或改断言：预览 operation = `workbuddy_models_save`；payload 不见 refresh token
- 不要出现 Codex upsert 的 reserved id

## 亲测

`hil-matrix.md` 的 AT9、H8、H9。不要替投放窗口勾 H5–H7。

## 回滚

只撤 SuperGrok 缝合。现有「自己填地址和钥匙」的 WorkBuddy 保存必须还能用。
