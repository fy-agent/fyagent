# 复用与替代方案评审

> 评审日期：2026-09-04

## 1. 复用顺序

按仓库 reuse contract：

1. 当前 FyAgent owner；
2. 已采用 crate/std/Tauri primitive；
3. 第一方 permissive source/protocol；
4. 维护良好的 OSS adapter；
5. 最后才自研。

本任务在前两级已覆盖运行时能力，第三方项目只作为行为证据，不新增依赖。

## 2. 选定复用

| Need | Owner | 结论 |
| --- | --- | --- |
| OpenAI OAuth/refresh/identity | 现有 managed-auth OpenAI实现 | 直接复用，不复制HTTP/OAuth |
| Secret storage/CAS/owner | SecretRef + repository | 直接复用 |
| durable事务 | Change Plan typed executor | 扩展闭集operation/resources，不新建工作流 |
| Provider切换/回填/policy/MCP | ProviderService | 提炼共享credential-aware official seam |
| Codex config/auth文件 | codex_config + config | 扩展现有atomic/readback owner |
| frontend状态 | ManagedAuth/ChangePlan FeaturePorts + Query | strict parser + authoritative readback |
| upstream schema | OpenAI Codex固定Apache-2.0源码 | 作为fixture/contract，不绑定内部crate |

## 3. 开源项目调研

审查了以下社区实现作为行为参考：

- `Loongphy/codex-auth`：围绕Codex auth文件的账号备份/切换；
- `bjesuiter/codex-switcher`：Codex CLI provider/profile切换；
- `fuyu0425/codex-as`：多账号切换；
- `Lampese/codex-switcher`：切换前保存live最新token，体现refresh rotation风险；
- `PepsiCommunity/codex-api-switcher`：API/provider切换。

这些项目证明“保存当前live token再切换”是实际需求，但它们通常自有profile文件、shell/CLI流程或独立writer。FyAgent已经有SecretRef、ProviderService、Change Plan和V2 contract，直接引入会产生双owner，因此不作为运行时依赖。

## 4. 被拒绝方案

### 直接把HIL常量改为true

拒绝。gate后没有完整writer/readback/backfill/owner handoff；只翻常量会放大状态假象和凭据丢失风险。

### Managed Auth新建第二个事务协调器

拒绝。Change Plan已拥有digest、guard、idempotency、cancel、durable event和crash recovery。应新增adapter而非重造。

### 先写官方auth，再调用现有Provider switch

拒绝。现有Provider switch在写目标前回填current live；先覆盖auth会让第三方API key丢失。必须在Provider owner内部复用回填顺序。

### 新建一套Codex auth/config writer

拒绝。`codex_config`/`config`已有路径、atomic write、Windows replace、backup和TOML owner。

### 通过App Server/CLI执行 `codex login`

不适用于本需求。第一方协议可启动新登录，但没有导入指定已保存OAuth bundle的稳定接口；shell还引入PATH、版本、窗口、超时和输出脱敏问题。

### 直接依赖OpenAI Codex内部Rust crate

拒绝。它是上游workspace内部实现，会带来大依赖图、API churn并重复FyAgent已有OAuth/secret owner。采用固定schema fixture + 最小adapter。

### 新增通用keyring crate，立即支持keyring/auto

首期拒绝。通用crate不能自动匹配Codex service/account naming、serialization、fallback和runtime backend；`auto`更无法静态判定。

### 把显式auto/keyring/ephemeral改成file

拒绝。会静默改变用户安全选择，并可能留下两份凭据。unset不同：第一方当前合约本身默认file，因此无需写配置也可作为有效file观察。

### Auth页复制第三方Provider/API key管理

拒绝。已有ProviderService/页面是owner，复制会形成两套配置与状态。

### 继续把matching-host HIL作为运行时开关

拒绝。HIL是兼容性证据，不能替代schema、权限、回填、CAS、readback和fault injection，也不应永久禁用已满足机器证据的能力。

## 5. 依赖结论

预计Cargo/npm manifest与lockfile零变化，复用当前：

- `serde_json`
- `toml_edit`
- `zeroize`
- `sha2`
- `windows-sys`
- std/Unix permissions
- `tempfile` / `serial_test`（测试）

任何新依赖需求都必须先回到planning完成exact version、license、维护、安全、平台、MSRV/features、transitive size和重复栈评审。
