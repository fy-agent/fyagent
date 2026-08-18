# Issue #35 可消费主线

## Goal

#35 模块交出一扇 #55 能按已锁 surfaces 核 `secretRef` 投影 digest 的门。GitHub #35 仍开。旧卡 `08-14-issue-35-secret-backend` 仍是 D2 权威，本卡接手实现。

## User value

上一班堆了 70+ fail-closed 测试，#55 仍接不上。本卡只关「投影身份可核」这一扇门，不再切 DTO。

## Confirmed facts

- 主机 Mini。工作树 `codex/issue-35-secret-backend` HEAD `4c393721`。未推。
- D2 `a338ee18` GRANTED。surfaces 锁定正文在归档，SHA-256 `8e33a79367218656209beeb3637290410c4baa4d565a9155882544979bed57b6`。#55 工作树同名文件是 0 字节，不当权威。
- 三份投影已有形：`SecretCandidateActivationProjection`、`SecretApplyPlanProjection`、`StagedSecretImportActivationProjection`。
- 现 mint 把 `binding_set_cas.digest` 填进 `projectionDigest`，apply 路径 `let _ = plan`。合同要求 RFC 8785 省略 `projectionDigest`，再加三条前缀后 SHA-256 小写 hex，无 `sha256:`。
- `crate::change_plan::secret_admission` 不存在。native 体大量 `todo!`。生产不写 Keychain。
- #55 仍 `ProviderCredentialIntentV1` + `expectedVersion`。#41 lease ≠ activation lease。

## Requirements

- R1 三份投影的 `projectionDigest` 按 D2 / surfaces §2 计算并回填。digest 不匹配整张拒绝。
- R2 三条前缀原样：`fyagent.secret.candidate-activation.v1\n`、`fyagent.secret.staged-import-activation.v1\n`、`fyagent.secret.apply-projection.v1\n`。
- R3 只用已有符号。不发明类型。不 mint `AdmittedSecretChangePlan` / `AdmittedStagedSecretImportPlan`。
- R4 `list_secret_candidates` 与 `check_secret_apply_readiness` 吐出的投影带正确 digest；apply 不得再 `let _ = plan`。
- R5 未知字段、digest 错、三份互解码失败。
- R6 本卡关闭 ≠ GitHub #35 关闭。不代关 issue。不推 `fy-agent/fyagent` 主仓。

## Acceptance Criteria

- [ ] AC1 同一投影两次独立计算 digest 相等；改任一合同字段 digest 变。
- [ ] AC2 digest 为 64 位小写 hex，无 `sha256:` 前缀。
- [ ] AC3 人工改 `projectionDigest` 后 `validate_repr` / deserialize 拒绝。
- [ ] AC4 三份投影互不混解码。
- [ ] AC5 `check_secret_apply_readiness` 成功路径持有未丢弃的 `SecretApplyPlanProjection`，其 digest 与 §2 一致。
- [ ] AC6 focused `secret_` 测试覆盖 AC1–AC5。证据等级 `local_runtime`。不宣称 #55 已接线、不宣称 native/UAT。

## Out of scope

- 关 GitHub #35 / 代填验收。
- #55 产品改动、丢掉 `ProviderCredentialIntentV1`、自己 `resolve_for_apply`。
- #41 两段 lease。
- 真写 Keychain / CredMan / Windows UAT。
- 再切 fail-closed 命令 DTO。
- `git add` `phase1-visual-*`。
- 覆写已锁 surfaces。
- #105、PR #108/#109、《才来》。

## Key decisions

- 本卡关闭信号 = digest 门，不是原 PRD §10。
- 阵列只调度，不在同一 `types.rs` 上并行写。
- 旧卡不改 D2 正文。
