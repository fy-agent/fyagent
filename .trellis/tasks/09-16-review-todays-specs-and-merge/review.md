# SPEC review findings

## Reviewed change set

2026-09-16 的稳定行为提交分为四条证据链：

| Commit | Behavior | Reviewed owners |
| --- | --- | --- |
| `f84d50fb` | FDE 提示词和 MCP 目录 | prompt presets、MCP catalogue、Renderer tests/browser evidence |
| `18d63c5c` | macOS helper 产物优先级 | release driver、system-commit SPEC、mise task contract |
| `e26c712d` | 托管账号代理复用 | Managed Auth、Provider、Proxy Runtime、HTTP pipeline、Agent target、Renderer Port/UI |
| `a28b8c3a` | 空闲代理 overview 可解析 | backend projection、strict Renderer parser、count/label regressions |

归档任务用于理解设计理由与证据边界；当前代码和回归用于确认事实；稳定 SPEC 不复制
提交 SHA、CI run、测试数量或一次性调试过程。

## Findings and resolutions

### 1. FDE 提示词目录缺少可执行集合

原 SPEC 只说明“六类”和静态目录，但没有列出分类 ID/顺序、每类数量，也把搜索笼统
写成 metadata/body。实现实际是 6 个固定 ID、每类 5 个、共 30 个，搜索只覆盖
name/description/category label/tags/inputs/approach/deliverables/evaluation。

处理：冻结精确类型、顺序、数量和字段；单测新增完整分类数组断言。没有修改目录正文
或 UI 行为。

### 2. FDE MCP filter 的 owner 不能阻止成员漂移

原 SPEC 只说“相关国内条目 + 八个新条目”，测试只锁数量、八个新增和少数代表项。
这不能发现一个无关条目加入同时另一个条目被移除的等量漂移。

处理：冻结 catalogue 顺序下的完整 20-ID 集合，并把测试改为精确有序数组；八个新增
recipe 的命令、env、secret、权限和 upstream 来源契约保持不变。

### 3. macOS helper 行为正确，但失败矩阵不够直接

脚本和 `miseTaskContract` 已经保证 `$SCRATCH_PATH/release/$name` 先于遗留
`apple/Products/Release/$name`。缺口是 SPEC 没把“双树同时存在”写进矩阵和
Wrong/Correct，未来评审仍可能误把 universal 旧产物当更可靠候选。

处理：补双产物场景、Good/Bad 和顺序示例；实现与测试不变。

### 4. Managed Auth core 同时承担四个 owner

`backend/managed-auth.md` 同时描述 vault/refresh、Provider bind、Proxy route observation
和 overview parser 补偿，接近 Trellis 注入上限并与 `proxy-runtime.md`、
`local-proxy-pipeline.md` 重复。前端通用契约仍沿用 `grok-subscription.md` 历史命名。

处理：

- 新建 `backend/managed-account-proxy.md`，唯一拥有显式账号准入、闭合命令/DTO/错误、
  stable Provider 身份、Claude/Grok 即时激活、Codex draft、route observation 映射和
  overview count/label 不变量；
- core 只保留 identity/credential、SecretRef、迁移、refresh 与 access-material；
- Proxy Runtime 只拥有 listener、目标写入/读回、锁和补偿；Local Proxy 只拥有请求
  变换、同账号 401 replay、SSE 和 usage；
- 前端当前 owner 迁到 `managed-account-subscriptions.md`，旧路径成为只读历史路由。

拆分后 `managed-auth.md` 为 25099 bytes，新 owner 为 15338 bytes；所有受影响 required
SPEC 均小于默认 `context_injection.max_file_bytes=32768`。

### 5. 实现一致性结论

完整数据流核对未发现需要修产品代码的偏差：

- generic bind 只允许 Claude Code/Codex/Grok Build，legacy xAI façade 才保留 Desktop；
- explicit overview identity 经 proxy-purpose/FyAgent-owned/Ready/SecretRef readback 准入，
  不回退 default 或 plaintext legacy；
- OpenAI Codex 使用 `fyagent_chatgpt`，Codex 不改变 current marker，Claude/Grok 才返回
  `activated=true`；
- observer 读取 effective Provider + loopback listener + live takeover，false/None 分别映射
  disconnected/unknown；`none` 清空 label，count 按仍命名账号的 unique consumer 计算；
- 401 refresh 保留同 credential lineage 且最多重放一次；HTTP/SSE owner 未被复制。

因此本任务只更新 SPEC/索引/任务材料，并增加两处精确目录回归。

## Evidence boundary

- 本轮没有改交互或 native 行为，不把历史 Chromium/WebKit/browser fixture 重新描述为
  本轮执行。
- synthetic IPC、fake vault 和 loopback upstream 证明契约/路由，不证明真实订阅额度、
  所有模型可用性或账号授权。
- 当前 macOS task-contract 证明脚本顺序，不等于签名发行、SMJobBless 安装或 Windows
  真机证据。
