# OpenAI Codex 第一方认证与 Provider 合约

> 检索日期：2026-09-04
> 规范性来源仅采用 OpenAI 官方文档、官方仓库和官方许可证。
> 固定源码 commit：`8e6a44b428e31f91b21edc97904fcdf4f0931ade`

## 1. 官方来源

### 文档

- Authentication: https://developers.openai.com/codex/auth
- Configuration reference: https://developers.openai.com/codex/config-reference
- Advanced configuration/custom providers: https://developers.openai.com/codex/config-advanced
- Sample configuration: https://developers.openai.com/codex/config-sample

### 固定源码

- Auth store / `AuthDotJson`: https://github.com/openai/codex/blob/8e6a44b428e31f91b21edc97904fcdf4f0931ade/codex-rs/login/src/auth/storage.rs
- `TokenData`: https://github.com/openai/codex/blob/8e6a44b428e31f91b21edc97904fcdf4f0931ade/codex-rs/login/src/token_data.rs
- auth manager/refresh/mode resolution: https://github.com/openai/codex/blob/8e6a44b428e31f91b21edc97904fcdf4f0931ade/codex-rs/login/src/auth/manager.rs
- config types/defaults: https://github.com/openai/codex/blob/8e6a44b428e31f91b21edc97904fcdf4f0931ade/codex-rs/config/src/config_toml.rs
- App Server protocol: https://github.com/openai/codex/blob/8e6a44b428e31f91b21edc97904fcdf4f0931ade/codex-rs/app-server/README.md
- License: https://github.com/openai/codex/blob/8e6a44b428e31f91b21edc97904fcdf4f0931ade/LICENSE

## 2. 已确认事实

### Credential store

第一方注释/文档定义：

- `file`：Codex home下 `auth.json`；
- `keyring`：OS credential store；
- `auto`：运行时优先 keyring，不可用时回退 file；
- `ephemeral`：源码中的非持久模式；
- **未配置 `cli_auth_credentials_store` 时默认 `file`。**

设计含义：

- FyAgent首期可支持显式 file和unset/default-file；
- 显式 auto/keyring/ephemeral不能通过静态配置推断成 file；
- 不应为启用功能而自动改写用户显式 store选择。

### Provider default

第一方配置把 `model_provider` 作为活动 Provider选择器；缺失时使用内建 OpenAI Provider。

设计含义：

- observation中 missing `model_provider`必须归类 official/openai；
- 第三方 Provider配置与官方 login cache是独立轴。

### Auth JSON shape

固定版本 `AuthDotJson` 包含：

- `auth_mode`；
- `OPENAI_API_KEY`；
- `tokens`；
- `last_refresh`；
- 可选 `agent_identity`；
- 可选 PAT、Bedrock API key/access keys。

标准 ChatGPT登录构造器写 `auth_mode=chatgpt`、token data和last refresh，其它互斥认证字段为空。`TokenData`包含 ID token、access token、refresh token和可选account ID。

设计含义：

- 不能把任意现有 JSON“保留未知字段合并”到另一个账号；未知字段可能是另一 auth mode的secret；
- PAT/Bedrock/API-key不混入目标 ChatGPT文档；
- `agent_identity`是账号绑定的私钥/JWT材料，只能在证明同账号时保留；缺失时第一方可为managed ChatGPT binding按需注册新identity。

### File permissions

第一方 file-store writer在Unix创建 owner-only `0600`文件。

设计含义：

- FyAgent必须至少匹配该权限语义；
- generic JSON write成功不够，必须读回权限和身份。

### Automatic refresh

官方文档与源码表明 Codex会自动刷新 ChatGPT token，并把更新写回 auth storage；源码在刷新前后检查当前 auth snapshot/account，避免覆盖已变化会话。

设计含义：

- 投影后 Codex成为 live lineage唯一 refresh owner；
- FyAgent切离账号前必须对账最新 native token；
- 同账号外部 refresh可吸收，不同账号变化不能覆盖受管 credential。

### Refresh响应并非可假设完整

OpenAI refresh API返回 access token，refresh/ID token是可选字段。FyAgent现有 refresh DTO也如此。

设计含义：

- “只有 refresh token就一定能重建完整 auth.json”不是可靠前提；
- refresh缺 ID token时，只能安全复用仍有效、身份匹配的旧ID token；否则 requires reauth。

### Managed login policy

官方配置支持强制 login method和ChatGPT workspace限制。

设计含义：

- FyAgent在写入前检查政策与目标账号；
- policy mismatch是能力/身份失败，不是HIL问题。

### App Server复用边界

审查的第一方App Server协议能启动ChatGPT/API-key登录、读取账号和logout，但没有稳定请求把FyAgent已有的 access/refresh/ID-token bundle导入为指定持久账号。

设计含义：

- App Server不是“无重新登录切换已保存账号”的直接替代；
- 不应通过shell执行 `codex login`规避当前owner和事务。

## 3. License与依赖结论

OpenAI Codex是Apache-2.0。本任务把固定源码作为合约/fixture依据，不依赖其内部workspace crate，不复制整套login/keyring/app-server。FyAgent实现最小adapter并在注释/测试中记录来源。

## 4. 上游变化检查

实施开始和归档前：

1. 比较固定文件与upstream HEAD；
2. 检查schema、default、permission、auth mode和refresh变化；
3. 发生实质变化时更新本文件、fixture和planning；
4. 不从新文档描述自动扩大store支持范围。
