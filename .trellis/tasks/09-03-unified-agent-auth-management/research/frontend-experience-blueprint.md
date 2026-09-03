# 前端体验蓝图：账号与认证

> 目标：把 PRD 的体验要求转换为可实现、可测试的 V2 页面结构。本文描述信息层次、状态和交互，不指定未经现有 V2 设计系统验证的视觉细节。

## 1. 用户心智模型

页面始终围绕三个问题组织：

1. **我保存了哪些官方账号？**
2. **每个软件连接了哪个账号？**
3. **这个软件当前实际把模型请求发到哪里？**

三者不能合并：

```text
OpenAI 账号：person@example.com            账号身份
Codex 连接：使用 person@example.com        软件连接
当前请求：DeepSeek API                     模型请求来源
OpenAI 官方登录：仍保留                   连接保护状态
```

默认页面语言不出现 refresh token、Credential Session、SecretRef、projection、generation 等内部概念。需要解释续期所有权时使用“由 FyAgent / Codex / OpenCode / Grok Build 自动续期”。

## 2. 导航与入口

### 2.1 侧边栏

```text
AI 软件配置
  AI 软件配置
  账号与认证
```

导航项可显示非计数型健康提示：

- 无提示：全部正常或没有账号；
- 警告点：至少一个已连接账号需要重新登录；
- 不显示具体数量，避免额度/网络短暂失败制造持续焦虑。

### 2.2 Agent 详情入口

Agent Auth 摘要提供单一主动作：

| Agent | 摘要 | 主动作 |
| --- | --- | --- |
| Codex | 账号、当前模型来源、官方登录保护 | 管理账号 |
| Grok Build | xAI账号和登录健康 | 管理登录 |
| OpenCode | 已连接Provider数量与异常项 | 管理连接 |
| Claude Code | 保持现有agent-owned登录动作 | 当前动作 |
| 国产Desktop | 保持打开应用handoff | 当前动作 |

进入 `/auth` 时只携带闭集参数：`consumer=codex|grokbuild|opencode|fyagent_proxy`、可选 `accountId`、安全 return descriptor。无绝对路径、URL或自由文本。

## 3. 页面骨架

### 3.1 宽窗口

```text
┌──────────────────────────────────────────────────────────────┐
│ 账号与认证                                 [添加账号]        │
│ 管理官方账号以及它们与 AI 软件的连接                         │
│ [账号 3] [软件连接 4]                       1 个需要处理      │
├───────────────────────┬──────────────────────────────────────┤
│ 搜索账号              │ OpenAI · person@example.com         │
│ [全部][OpenAI][xAI]   │ 已登录 · ChatGPT Plus               │
│                       │                                      │
│ ● person@example.com  │ 账号状态                             │
│   OpenAI · 默认       │ 上次认证：今天                       │
│   已连接 2 个软件     │ [重新登录] [设为默认]                │
│                       │                                      │
│ ● team@example.com    │ 已连接软件                           │
│   OpenAI · 需重登     │ Codex       已连接                   │
│   已连接 1 个软件     │ 当前请求    DeepSeek API             │
│                       │ 官方登录    已保留                    │
│ ● xai@example.com     │ [管理 Codex 连接]                    │
│   xAI · 正常          │                                      │
│                       │ FyAgent Proxy  已连接                 │
│                       │ 当前请求      OpenAI/Codex订阅        │
│                       │ [管理本地路由连接]                    │
│                       │                                      │
│                       │ 危险操作                             │
│                       │ [移除账号]                           │
└───────────────────────┴──────────────────────────────────────┘
```

实现优先复用 `CatalogMasterDetail`、`FeatureTabs`、已有按钮/notice/dialog。列表与详情各自滚动，页面标题和主动作稳定可见。

### 3.2 窄窗口

```text
账号列表页
┌─────────────────────┐
│ 账号与认证 [添加]   │
│ [账号][软件连接]    │
│ 搜索                │
│ 账号卡片            │
│ 账号卡片            │
└─────────────────────┘

点账号后
┌─────────────────────┐
│ ← 账号              │
│ person@example.com  │
│ 状态与动作          │
│ 已连接软件          │
│ 危险操作            │
└─────────────────────┘
```

返回后恢复搜索、过滤、滚动位置和焦点。不得在窄窗口把操作压进不可发现的横向滚动区。

## 4. “账号”视图

### 4.1 列表顺序

默认顺序：

1. 需要重新登录/迁移受阻；
2. 当前选中consumer使用的账号；
3. 默认账号；
4. 最近认证时间；
5. login字典序。

额度百分比不参与账号顺序，避免短周期用量使列表抖动。

### 4.2 账号卡片

```text
[Provider icon] person@example.com       [默认]
OpenAI · ChatGPT Plus
已连接：Codex、FyAgent Proxy
状态：正常
```

异常：

```text
[Provider icon] team@example.com
OpenAI
需要重新登录                         [重新登录]
Codex 暂时无法使用此账号
```

规则：

- 账号级异常只显示最高优先级原因；详情展示所有受影响连接。
- quota/profile独立区域可以显示“额度暂时不可用”，不改变“已登录”。
- 卡片上不放“删除”；删除只在详情危险区域，降低误触。
- email视觉截断，但accessible name保留完整文本。

### 4.3 空状态

```text
还没有官方账号
添加 OpenAI、xAI 或 GitHub Copilot 账号后，可以在支持的软件之间管理连接。
[添加账号]
```

从 Codex deep link进入时：

```text
Codex 还没有可用的 OpenAI 账号
添加账号后可直接连接到 Codex。
[添加 OpenAI 账号]
```

不展示与当前任务无关的所有Provider选择。

## 5. “软件连接”视图

### 5.1 Consumer 卡片

```text
Codex                                      [已连接]
账号连接      OpenAI · person@example.com
当前模型来源  DeepSeek API
官方登录      已保留
自动续期      由 Codex 管理

[切换账号] [切回 OpenAI Official] [刷新状态]
```

```text
OpenCode                                   [需要处理]
Provider 连接  OpenAI · 正常
               xAI · 需要重新登录
应用状态       正在运行

[管理 Provider] [打开 OpenCode]
```

```text
Grok Build                                 [等待重启]
xAI 账号       person@example.com
凭据已更新，Grok Build 尚未重新加载

[立即重启] [稍后]
```

### 5.2 多安装目标

若 inventory 有多个目标，卡片先显示“选择安装实例”，不默认选第一个。选择器复用现有稳定 target capability；用户选择后，connection ID 与 target ID 固定，后续操作前再次 revalidate。

### 5.3 未安装

账号页不承担安装：

```text
OpenCode
尚未检测到可管理的桌面安装
[返回 AI 软件配置]
```

不得在 Auth mutation 中偷偷触发安装。安装完成后刷新 inventory 即可继续。

## 6. 添加账号向导

### Step 1 — 选择账号类型

```text
添加账号
选择官方账号

OpenAI
用于 Codex、FyAgent Local Proxy 或 OpenCode

xAI
用于 Grok Build、FyAgent Local Proxy 或 OpenCode

GitHub Copilot
用于支持 Copilot 的 Provider
```

若从 consumer deep link进入，优先过滤可用Provider，并在下方显示“将连接到 Codex”；用户仍可取消自动连接，仅保存账号。

### Step 2 — 选择用途

```text
这次登录用于
(●) 连接 Codex
( ) 仅保存账号
```

如果相同identity已经存在其他session：

```text
此账号已用于 FyAgent Local Proxy。
为 Codex 单独授权可以避免多个程序同时续期同一登录凭据。
```

不要求用户理解refresh-token lineage。

### Step 3 — 官方登录准备

OpenAI：

```text
即将打开 OpenAI 官方登录页
域名：auth.openai.com / chatgpt.com
完成后浏览器会返回此设备。

[继续] [改用设备码]
```

xAI：

```text
即将打开 xAI 官方验证页
域名：auth.x.ai
浏览器会要求确认下方设备码。

[继续]
```

域名文本来自前端闭集，不从authorization URL任意显示。

### Step 4A — Browser callback waiting

```text
正在等待 OpenAI 授权
请在浏览器中完成登录。此窗口可以暂时关闭，登录会继续。

[重新打开官方页面] [改用设备码] [取消登录]
```

阶段变化：

```text
已收到授权，正在验证…
正在安全保存账号…
正在连接 Codex…
正在确认 Codex 登录状态…
```

不能显示“授权成功”后再因secret/native write失败回退为错误；只有最后readback完成后显示成功。

### Step 4B — Device Code

```text
在 OpenAI 官方页面输入设备码
https://auth.openai.com/codex/device

ABCD-EFGH                         [复制]
只使用你刚刚在 FyAgent 中请求的设备码。

[打开官方页面] [取消登录]
```

user code使用等宽样式、可选择/复制；复制反馈不抢走焦点。过期时原位置切换为：

```text
设备码已过期
[生成新设备码] [返回]
```

### Step 5 — 完成

```text
账号已添加
person@example.com · OpenAI

Codex 已连接并验证
当前模型来源：OpenAI Official

[完成] [查看 Codex 连接]
```

部分完成：

```text
账号已安全保存
Codex 尚未加载新的登录信息，需要重启。

[立即重启 Codex] [稍后]
```

这不是全局失败，也不能显示“Codex 已连接”。

## 7. 重新登录

重新登录从账号详情发起，并明确影响：

```text
重新登录 person@example.com
将更新该账号用于 Codex 的登录凭据。
FyAgent Proxy 使用独立登录，不受影响。
```

成功后留在原账号详情，保持选择和滚动位置。若新登录返回不同稳定身份，backend拒绝覆盖原账号，UI显示：

```text
登录的不是原账号
请使用 person@example.com 完成登录，或返回后将新账号单独添加。
```

不得通过email相同就强制合并tenant/workspace不同的身份。

## 8. 切换与断开

### 8.1 切换账号

```text
切换 Codex 账号

当前：person@example.com
目标：team@example.com

切换时 Codex 需要关闭并重新打开。当前第三方 Provider 配置不会被删除。
[切换并重启] [取消]
```

backend preview列出真实影响；前端不自行推导。

### 8.2 切回官方

```text
切回 OpenAI Official
将停止使用 DeepSeek API，并继续使用已保存的 person@example.com 登录。
[切换]
```

官方session失效：先重新登录，第三方配置在最终提交前保持运行。

### 8.3 断开

断开connection不默认删除账号：

```text
断开 Codex 与 person@example.com？
账号仍会保存在“账号”中，FyAgent Proxy连接不受影响。
[断开] [取消]
```

## 9. 移除账号

影响预览示例：

```text
移除 person@example.com？

将断开
• Codex
• FyAgent Local Proxy

不会改变
• Codex 当前 DeepSeek API 配置

OpenCode 使用独立登录，不受影响。

[移除账号] [取消]
```

若部分断开失败，不删除secret并显示可重试状态。不能出现账号卡片已消失但consumer继续依赖悬空凭据。

## 10. 状态与文案矩阵

| Backend state | 标题 | 主要动作 |
| --- | --- | --- |
| `ready` | 已登录 / 已连接 | 管理、切换 |
| `checking` | 正在确认状态 | 取消可用操作或等待 |
| `needs_reauth` | 需要重新登录 | 重新登录 |
| `migration_blocked` | 账号迁移未完成 | 重试迁移 / 查看说明 |
| `secret_store_locked` | 系统凭据库已锁定 | 解锁后重试 |
| `native_store_unsupported` | 当前凭据存储方式暂不支持 | 在官方应用登录 / 查看支持方式 |
| `external_change_detected` | 登录已在其他应用中更改 | 刷新状态 / 重新选择账号 |
| `pending_restart` | 等待应用重新加载 | 重启应用 / 稍后 |
| `projection_readback_failed` | 已保存账号，但未确认软件连接 | 重试确认 / 打开软件 |
| `unknown` | 无法确认当前状态 | 刷新状态；不显示成功 |

错误详情保持两层：首屏一句安全摘要；“查看详情”只显示safe reason、时间、consumer和建议，不显示raw backend error、URL query或路径。

## 11. 进行中任务

页面标题下方统一显示backend session：

```text
正在等待 OpenAI 登录…                              [查看]
```

同一Provider/purpose只允许一个冲突session。用户再次点击“添加”时，打开现有session而不是新建。终态展示一次后从列表移除；失败可“重试”，重试生成新session ID和state。

## 12. 视觉与交互原则

- 使用现有 V2 设计语言，不新引入图标/组件库。
- 正常状态弱化，异常和主动作清晰；不把页面变成监控仪表盘。
- 一屏只突出一个主动作；破坏性动作远离常用动作。
- 状态变化不导致列表大幅跳动；selected item稳定。
- 不显示token过期秒级倒计时；显示认证健康与需要动作即可。
- Spinner、进度文字与disabled action一致；不要让按钮点击后无反馈。
- Toast只做补充；关键成功、失败、pending restart保留在页面/dialog中。
- 所有官方外链使用现有安全外链组件或backend opener；不通过普通 `<a target>` 绕过策略。

## 13. 前端验收场景

至少以 Mock FeaturePort 和 browser test完成：

1. 新用户从Codex Agent卡片添加OpenAI账号并自动连接。
2. 1455/1457不可用，向导无损转Device Code。
3. dialog关闭后从进行中任务恢复；取消后late callback不显示成功。
4. 同一identity已有Proxy session，连接Codex时说明将建立独立授权。
5. Codex从OpenAI切DeepSeek再切回，官方登录保护状态始终可见。
6. OpenCode Desktop已安装但CLI不存在，仍显示Provider状态与管理入口。
7. xAI账号需要重新登录，只影响Grok connection，不把quota失败误判为退出。
8. 删除多连接账号，影响预览正确；部分失败时账号不消失。
9. pending restart与restart failure的页面状态可恢复。
10. keyboard、focus、窄窗口、reduced motion和screen-reader名称通过。
11. DOM、console、mock payload、snapshot没有token/code/state/verifier/SecretRef/路径。
12. 从 `/auth` 返回Agent详情时恢复原Agent和section。
