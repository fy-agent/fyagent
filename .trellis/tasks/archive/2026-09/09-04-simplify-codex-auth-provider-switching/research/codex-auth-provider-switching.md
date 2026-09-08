# Codex 认证/Provider 切换调研索引

> 检索日期：2026-09-04

本文件是调研入口；规范性结论拆分到以下文档，避免单一长文重复且漂移：

1. `openai-codex-first-party-contract.md`
   - OpenAI官方auth store、默认值、AuthDotJson、权限、refresh、policy与App Server边界；
2. `current-repository-inventory.md`
   - FyAgent现有ManagedAuth、Provider、codex_config、Change Plan与V2 owner；
3. `transaction-boundary-review.md`
   - 为什么按最小差异复用 Managed Auth/Provider owner，而不新增 Change Plan operation；
4. `reuse-and-alternatives.md`
   - 开源项目、依赖与拒绝方案；
5. `hil-policy-review.md`
   - HIL从运行时硬门控调整为非阻断兼容性证据。

## 最终结论摘要

- HIL blanket gate应移除，但真实store/identity/transaction gate保留；
- unset和explicit file是当前第一方合约下的有效file store；explicit auto/keyring/ephemeral保持fail closed且不自动改配置；
- missing model_provider按openai观察；
- 官方 A→B 的核心就是完整 `auth.json` 的原子交换；
- live账号已是目标时，第三方→官方只切 `model_provider`/现有 Provider current；
- 账号和Provider都变化时才执行 auth→Provider 的最小组合；
- legacy API-key-only auth在覆盖前复用ProviderService回填，否则可能丢API key；
- 不新增 Change Plan public operation；第三方Provider管理继续复用现有Provider页；
- 新grant/完整bundle直接物化，只有材料不完整或失效时才复用refresh；
- 社区switcher只提供行为证据，不作为运行时依赖；
- 无新增依赖。
