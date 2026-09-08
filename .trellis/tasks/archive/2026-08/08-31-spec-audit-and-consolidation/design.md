# 设计：Trellis spec 分层与精简规则

## 文档角色

### 1. Index

负责阅读顺序、领域路由、文档角色和维护规则。只概括“去哪里读”，不复制具体
DTO、路径矩阵、错误码或测试清单。

### 2. Code spec

负责稳定且可执行的当前合同：scope、authority、signature、validation/error、
Good/Base/Bad、tests、Wrong/Correct。跨层、高风险或曾发生回归的行为保留足够深度。

### 3. Thinking guide

负责实现前的简短检查顺序和决策问题。具体功能事实必须链接到 owning spec，
不得形成第二份功能合同。

### 4. Task / provenance / Git

负责一次性调研、迁移日期、审核 SHA、历史版本与执行证据。长期 spec 只定义如何
验证和更新，不复制某一次执行结果。

## 精简规则

1. **一个语义一个 owner**：其他文档用相对链接路由，不复制整段合同。
2. **稳定规则留在 spec**：易变版本、文件清单和当前测试数量从权威配置或测试读取。
3. **历史证据外置**：提交 SHA、上游 tag object、迁移日期进入 provenance/task/Git。
4. **override 收敛**：把当前行为写进正文，并删除已经被覆盖的旧条款和顶部补丁。
5. **风险优先**：权限、secret、TOCTOU、签名、回滚、原生平台和 fail-closed 细节不因
   文档过长而折叠。
6. **规则可验证**：保留明确的失败条件和测试 owner；不以叙事性快照代替门禁。

## 计划修改范围

- 重写三个索引，明确分层、阅读顺序与维护规则。
- 精简两个 thinking guide、Frontend Reuse、Development Environment、Upstream Sync。
- 移除 Development Hooks 的固定 Trellis 版本快照。
- 将 V2 Shell、Agents/Models、Skills/MCP、Prompts/Memory 的日期化补丁合入正文，
  并删除旧冲突条款。
- 将少量“某年 clean break”“当前 handler 数量”等瞬时描述改为稳定规则。
- 其余高风险合同逐份审阅后保留，并在 inventory 中记录理由。

## 评审策略

### Round 1：结构

- 41/41 inventory 覆盖；
- index 目录覆盖和相对链接；
- 标题、重复段落、TODO/TBD、硬编码开发路径扫描；
- 日期化 override 与被覆盖条款扫描。

### Round 2：语义与代码权威

- 对照实际配置、测试和实现 owner，确认被移除内容仍有唯一权威来源；
- 重点复核 V2 navigation/configuration、upstream provenance、toolchain authority；
- 对安全/发布/Windows/Codex/Agent 合同做负向抽查，确认未削弱 fail-closed 边界。

### Round 3：最终差异与门禁

- 审阅 `git diff --stat`、`git diff --check` 和完整 diff；
- 运行 Trellis、文档、合同及格式检查；
- 归档任务后再运行不带 active-task 排除的 canonical 检查。
