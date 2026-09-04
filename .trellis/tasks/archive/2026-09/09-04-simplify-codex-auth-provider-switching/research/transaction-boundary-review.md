# 事务边界复核：不新增 Change Plan operation

> 评审日期：2026-09-04

## 1. 复核问题

最初规划倾向于为 Codex Managed Auth 新增一套 credential-aware Change Plan
operation，把账号、Provider、auth/config、refresh owner 和 connection 全部纳入
durable ledger。复审当前代码后，这一做法超出了本需求的最小边界：

- `ManagedAuthConnectionActionRequest` 已有 connection revision 和闭集 action；
- 官方账号 A→B 不改变 Provider、DB current、device current 或 MCP；
- 当前第三方 Provider 路径已经是 config-only，正常情况下会保留官方
  `auth.json`；
- `ProviderService` 已拥有 Codex mutation guard、current-provider backfill、
  policy、DB/device current、live config 和 MCP；
- Managed Auth 新增 Change Plan operation 会引入新的 public operation/resource
  enums、ledger fixture、strict parser、job UI 与 crash recovery 分类，但这些
  对纯 `auth.json` 交换没有新增事实价值。

## 2. 修正后的决策

本任务**不新增 Change Plan public operation/adapter/resource**。

按最小差异选择现有 owner：

| 变化 | 执行 owner |
| --- | --- |
| 仅官方账号变化 | Managed Auth Codex consumer 的窄 `auth.json` 交换器 |
| 仅 Provider 从第三方切回官方 | 现有 `ProviderService` official switch seam |
| 账号与 Provider 都变化 | 同一 Codex mutation guard 下，先安全准备/提交 auth，再复用 `ProviderService` 切 official；失败时按 auth revision 补偿 |
| 官方切第三方 | 完全沿用现有 Provider Change Plan/Provider 页面；Managed Auth 不复制此能力 |

Change Plan 仍是 Provider 页面显式 Provider 变更的 owner；本任务不修改其公共
contract。Managed Auth 只调用 `ProviderService` 的 crate-private lock-held seam，
不得复制 backfill、proxy policy、DB/device current、live config 或 MCP 逻辑。

## 3. 为什么这个边界更简单且足够安全

### 3.1 纯账号切换没有必要进入 Provider ledger

官方 A→B 的真实副作用只有：

1. 对账当前 live 官方账号的最新 token；
2. 原子替换完整 `auth.json`；
3. 写后读回身份；
4. 更新 Managed Auth connection/refresh owner；
5. Codex 正在运行时标记 pending restart。

把 Provider DB/device/MCP 和 Change Plan job 一并引入只会放大状态空间。

### 3.2 组合切换的中间态是可判定、可重试的

第三方 → 官方 B 时，安全顺序是：

1. 在现有 Codex Provider mutation guard 内读取基线；
2. 若 live auth 是 legacy API-key-only，先复用 ProviderService current backfill；
3. 原子写 B 的完整 auth，并读回；
4. 复用 ProviderService 把 route 切到 `codex-official`；
5. 最后提交 connection/owner。

如果进程在第 3 步后中断，Provider 仍是第三方，当前请求路径不受新官方 auth
影响；下一次 overview 能识别“目标 auth 已写、route 尚未切换”并安全重试。
如果第 4 步成功，则目标状态已经达到。若 Provider switch 返回失败，只有在
auth revision 仍是本次写入值时才恢复 preimage；发现外部变化就停止覆盖。

因此不需要为了这一可恢复中间态新增一套 durable operation ledger。

### 3.3 仍保留必要的并发和恢复边界

- 所有 Codex auth/provider 组合动作共用现有 per-app mutation guard；
- auth 文件另有进程内 writer lock、expected revision、bounded read、原子写、
  `0600`、readback 与 revision-aware rollback；
- credential generation/refresh owner 使用现有 CAS；
- renderer 不传 token、路径、TOML、Provider ID 或 store override；
- overview 始终从 live 文件与 DB authority 重建，不从一次函数返回猜测成功。

## 4. 不采用的方案

### 新增 credential-aware Change Plan

拒绝作为首期方案。它会扩大 public contract 和实现范围，但纯 auth swap 不需要
Provider job ledger；组合切换已有可判定的安全中间态和现有 Provider owner。

### 在 Managed Auth 内复制 Provider switch

拒绝。必须调用现有 `ProviderService` seam；不得复制 backfill、current、proxy、
MCP 或 config writer。

### 先改 Provider，再补 auth

拒绝。可能出现 route 已变为 official、但目标 auth 尚未就绪的窗口。应先使
目标官方 auth 就绪，再切 route。

### 不检查 legacy API-key-only auth 直接覆盖

拒绝。当前新路径虽为 config-only，历史状态仍可能把第三方 key 放在
`auth.json`。必须先由现有 Provider owner证明已回填，否则 fail closed。

## 5. 触发重新评审的条件

只有出现以下事实之一，才重新考虑引入 Change Plan operation：

- Managed Auth 动作需要跨进程持久排队或用户可取消 job；
- 组合切换出现无法通过 live readback 分类的中间态；
- ProviderService 无法提供 lock-held、可验证的窄 seam；
- crash recovery 必须依赖历史步骤 ledger，而当前 authority 无法重建状态。

在这些事实出现前，不为“看起来统一”而扩张 Change Plan。
