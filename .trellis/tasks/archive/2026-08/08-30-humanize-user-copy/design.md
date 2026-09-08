# Design — Human-readable user copy

## Change boundary

The behavior gap is presentational: FyAgent often exposes the words used to
design or verify a feature instead of the information a user needs to complete
a task. The underlying conservative safeguards must not be weakened.

The change therefore lives in renderer presentation code, public
documentation, tests coupled to visible copy, and the frontend SPEC. Backend
reason codes, plan/job DTOs, readback checks, rollback logic, persistence, and
platform ports remain intact.

## Writing model

At each moment, copy answers only the relevant questions:

1. What is happening or what happened?
2. What changed or may have changed?
3. What can the user do next?

Implementation mechanisms are translated into outcomes:

- “已从真实配置回读：WorkBuddy 已启用此 Skill” becomes “WorkBuddy 已启用此
  Skill”.
- “当前页面不会保留乐观成功状态” becomes “设置未更新，请重试”.
- “安装准备度” becomes “安装状态”.
- “零写入预览” becomes “确认前不会修改配置”.
- “确认只发送计划身份” becomes “确认后按此预览应用；不会再次提交密钥”.
- “回读不一致” becomes “保存结果与当前配置不一致，请重新打开并检查”.

The interface may still name a backup, configuration file, Provider, API Key,
or model ID when that object is directly useful to the user. It must not name
an internal state machine, event sequence, opaque plan/job identity, adapter,
projection, baseline, compensation engine, or evidence policy.

## Interface changes

### Agent configuration

Keep the existing authoritative refetch after assignment, authentication, and
installation actions. Only rewrite the resulting copy. Successful actions
state the final assignment; failed or mismatched refreshes state that the
setting could not be confirmed and recommend retrying or reopening the page.

### Installation status

Rename “readiness” presentation to “installation status”. Reason-code mapping
remains exhaustive, but messages explain the available route in ordinary
language. A failed status read does not guess; it says the status could not be
checked and offers retry or product-page guidance where available.

### Configuration previews and application progress

Retain the current preview-before-write, single-confirmation, digest, expiry,
rollback, and readback contracts. Present them as:

- changes that will be made;
- files or settings affected;
- backup and recovery behavior;
- progress;
- final result and any required action.

Do not render the backend event sequence. It is diagnostic metadata and has no
user action attached to it. Internal `ChangePlan` and `ChangeJob` names remain
in types, ports, wire-parser tests, and developer docs.

## Documentation changes

The root READMEs share one information order:

1. one concrete product sentence;
2. download, manual, and contribution links;
3. current supported tasks and limits;
4. screenshots;
5. quick start;
6. safety and release notes;
7. development and licensing.

Future direction is short and explicitly separated from current capability.
Marketing slogans, mission/vision triplets, repeated “AI era” framing, and
explanations of what a metaphor “really means” are removed.

Other public docs are changed only when the same problems occur. Technical
terms required for exact procedures stay. Historical release records and
legal language are not normalized for style.

## Regression strategy

A focused source-contract test scans production V2 presentation files and
rejects a small, reviewed set of implementation phrases. It deliberately
excludes bounded feature-port and platform-adapter files so internal protocol
names remain available. This avoids subjective punctuation policing and keeps
the test tied to the concrete UX failure.

Existing behavior tests continue to prove authoritative refresh, stale-plan
rejection, cancellation, rollback, and secret handling. Their visible text
expectations are updated to the new presentation.

## Rollback

All product changes are text/spec/test changes except removal of rendered
backend event sequence metadata. Reverting the implementation commit restores
prior presentation without data migration. No backend or stored configuration
rollback is required.
