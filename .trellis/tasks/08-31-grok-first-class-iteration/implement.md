# Implement — Grok first-class iteration

先读 [summary.md](./summary.md) 和 [use-cases.md](./use-cases.md)。父任务不改产品代码。

下游开工读序（每个子任务都要齐）：`summary.md` → 子任务 `prd.md` → `design.md` → `implement.md` → `use-cases.md` → 该任务 `research/` → `implement.jsonl` 里的 spec。

不要从空白开始，也不要只读父任务摘要就改代码。

## Feature inventory

| 编号 | 人能做成的事 | 谁做 | 回写 |
|---|---|---|---|
| F1 | Grok 官方登录/退出找得到，并且不说已经登录 | 登录窗口 | #43 |
| F2 | SuperGrok 扫码的下一步指向认证中心 | 登录窗口 | #43 |
| F3 | 模型页继续只填 API 钥匙 | 登录窗口 | #43 |
| F4 | SuperGrok 能进 Claude Code、Claude Desktop、Codex：每家先看、再改、再检查（Desktop 可在旧界面完成） | 投放窗口 | #42 / #41 / #63 |
| F5 | SuperGrok 能进 WorkBuddy：先看、再改、再检查 | WorkBuddy 窗口 | #42 |
| F6 | 名单上有名字，不等于已经完全支持 | 各窗口改字时都遵守 | #22 / #106 |

## Change inventory

| 编号 | 要改 | 不要改 |
|---|---|---|
| C1 | Grok 认证区的字，点名去终端跑 `grok login` | Claude 那种「查一下真的登了」 |
| C2 | 扫码的下一步指到认证中心 | 新做一套登录；把旧设置页搬进新界面 |
| C3 | Claude Code / Desktop：复用已有 `xai_oauth` 绑定；每家独立保存 | Claude Change Plan 新执行器；和 Codex 写一张单 |
| C4 | 现有 Codex 预览：认证中心已有 SuperGrok 账号时放行；新界面能看见这条路 | 第四套保存；预览单里放钥匙；不打招呼就盖掉原来的 API 钥匙槽 |
| C5 | WorkBuddy 自己的保存预览能用已扫码账号拉模型 | 走 Codex upsert；把刷新令牌抄进 `models.json` |
| C6 | 用自动检查把 F1–F5 钉住 | 没必要就别动模型草稿 |

## Ordered work

1. 先立登录路标，免得投放还在叫人去跑 `grok login`。
2. 再开 Claude / Desktop / Codex：每家独立预览和保存。Codex 先改「准不准预览」，再补新界面能看见的路。
3. 再开 WorkBuddy：走它自己的 Change Plan。
4. 总控把三条线接成一次能走完。
5. William 在两台电脑上按 `research/hil-matrix.md` 亲测。
6. 回写 GitHub，不关整张 #42 / #43。

## Validation

- 各窗口自己的程序检查。
- 总控：`research/hil-matrix.md` 两台电脑都打勾。
- 密码不进仓库。

## Risky files

- `src/v2/pages/agents/AgentAuthStatusPanel.tsx`
- Grok 登录交接（先看，不要轻易改短路径）
- Codex 预览是否放行（`prove_codex_target_credential_capability` 一带）
- Claude / Desktop 的 `xai_oauth` 预设和 `ProviderForm` 绑定
- WorkBuddy `create_workbuddy_save_plan` / `models.json` 写入
- 认证中心现有扫码界面（尽量只指路，不重做）

## Rollback points

- 登录路标撤了，不影响投放写入。
- Claude / Desktop / Codex / WorkBuddy 可以单独关一扇门，不要互相连坐。
