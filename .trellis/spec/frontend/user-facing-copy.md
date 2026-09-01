# User-Facing Copy

This contract applies to text that a product user or repository visitor reads:

- production V2 headings, descriptions, notices, dialogs, progress text,
  controls, tooltips, and accessible names;
- leftover renderer translations in the four registered locales;
- root READMEs and public documentation under `docs/**`.

It does not prohibit precise engineering terminology in source identifiers,
wire contracts, tests of serialized behavior, backend logs, or internal specs.

## Core rule

Write from the reader's task, not from the implementation that produced the
state.

For each message, include only the information needed to answer the relevant
questions:

1. What is happening or what happened?
2. What changed, or what remains uncertain?
3. What can the reader do next?

Do not narrate how the code proved the result when the proof mechanism does not
change the user's decision.

## Interface copy

### Use outcomes, not implementation commentary

Bad:

```text
已从真实配置回读：WorkBuddy 已启用此 Skill。
当前页面不会保留乐观成功状态。
```

Good:

```text
已在 WorkBuddy 中启用此 Skill。
无法确认 Skill 设置是否已更新。请刷新后重试。
```

The implementation may still perform an authoritative refresh. The interface
states the result or the uncertainty; it does not explain optimistic updates,
readback authority, state convergence, or proof policy.

### Name the object the user can recognize

Keep product terms when users must identify or act on them:

- Provider, Skill, MCP, Prompt, model ID;
- API Key or login state;
- a named application;
- a configuration file or backup when its path or existence matters.

Desktop lifecycle launch is **「打开软件」**, not 「打开应用」 and not a
helper/XPC/transaction label.

Do not expose internal-only concepts as interface labels:

- Change Plan / Change Job identifiers;
- adapters, projections, baselines, compensation engines;
- event sequence numbers, revisions, digests, opaque tokens;
- “readiness”, “authoritative state”, “success evidence”, “optimistic state”,
  or renderer/backend wiring.

An internal reason code may map to ordinary language, but the code itself is
not user copy.

### Success, failure, and uncertainty

- A success message states the completed outcome. Do not add self-congratulation
  or explain the verification mechanism.
- A failure message states what did not complete and gives a retry, correction,
  or support path.
- An uncertain result must not claim success or failure. State what cannot be
  confirmed, stop unsafe follow-up writes where required, and tell the user how
  to inspect or retry.
- Do not expose raw backend errors unless the contract explicitly marks them as
  safe, actionable display text.

### Settings and user-manual installer copy

- Settings/Tooling may show install/update controls only for Grok Build.
  Claude, Gemini, OpenCode, OpenClaw, Hermes, and Codex must not present
  npm, Shell, PowerShell, WinGet, copy-all, or remote-script install bundles.
- Conflict and upgrade dialogs may show source/version/default, never an
  absolute path, SID, package family, or the command that will run.
- Public `docs/user-manual/**` must match that policy. Agent Desktop products
  are installed from the Agent directory, not from a Settings command table.
  Optional Node/Homebrew notes in getting-started remain environment setup,
  not FyAgent Agent installers.

Examples:

```text
配置已保存。请在 Codex 中新建会话后检查模型。
无法确认当前配置。请重新打开页面并检查后再保存。
安装仍在进行。可稍后刷新安装状态。
```

### Empty and loading states

- State what is missing, not what internal query or assignment structure is
  unavailable.
- Provide the next useful action when one exists.
- Avoid showing the same loading sentence twice visually. A spinner's
  accessible label may match the visible sentence when needed for assistive
  technology.

### Confirmation and safety copy

Explain:

- what setting or file will change;
- whether confirmation is still required;
- whether an existing value will be overwritten;
- whether a backup or rollback is available;
- what the user should verify afterward.

Do not explain opaque plan identity, digest validation, write adapters,
idempotency, or event replay in the product surface. Those remain engineering
contracts and tests.

### Tone and structure

- Prefer a direct verb and a concrete noun.
- Use the shortest sentence that preserves the decision-relevant facts.
- Avoid generic praise, inflated importance, motivational filler, and
  anthropomorphic claims.
- Avoid repeated rhetorical structures such as mission/vision/value triplets,
  “not X but Y” explanations, or mechanically symmetric lists unless the
  distinction itself helps the reader decide.
- Do not ban punctuation in isolation. An em dash, colon, or bold label is not
  a defect by itself; judge the sentence by clarity, specificity, and purpose.
- Do not describe a message as “real”, “authoritative”, “safe”, or “verified”
  unless the adjective changes what the user should do and the claim is backed
  by the actual contract.

## Public documentation

### Lead with current, concrete value

A README introduction should establish:

1. what the product is;
2. which current task it helps with;
3. the supported platform or maturity limit that affects adoption;
4. where to download it or learn the first workflow.

Do not make readers pass through slogans, a mission statement, a metaphor
definition, or a future product vision before seeing current behavior.

### Separate current capability from direction

- Put shipped behavior in a clearly named current-capabilities section.
- Keep future direction brief and explicitly non-shipped.
- Never convert an aspiration into a present-tense product claim.
- Preserve evidence boundaries for signing, notarization, platform support,
  tests, and release trust.

### Preserve exact technical and legal meaning

Plain language is not permission to remove facts needed for a command,
security decision, data migration, compatibility boundary, or license.

Do not stylistically rewrite:

- license text or attribution;
- historical release facts merely to make them sound newer;
- exact paths, commands, reason codes, or wire fields when a technical reader
  must use them;
- warnings whose precision prevents data loss or unsafe execution.

Rewrite the surrounding explanation so readers can find, understand, and act
on those facts.

## Translation rules

- Production V2 remains hardcoded Chinese until an explicit i18n migration is
  approved.
- Leftover renderer copy must update `en`, `ja`, `zh`, and `zh-TW` keys
  together.
- Translations should preserve the same user decision, not mirror sentence
  structure word for word.
- Product names, commands, configuration keys, and serialized values remain
  unchanged unless their owning contract changes.

## Review checklist

Before merging user-visible text, verify:

- [ ] The first sentence says what happened or what the screen is for.
- [ ] Every technical term is one the target reader must recognize or use.
- [ ] Errors and uncertain states include a safe next step.
- [ ] Copy does not reveal an opaque token, event sequence, adapter, projection,
      baseline, internal state machine, or implementation proof mechanism.
- [ ] Settings/user-manual copy does not restore non-Grok CLI install/update
      command tables or display absolute install paths.
- [ ] The text does not claim a capability, platform, signing state, or
      successful result beyond available evidence.
- [ ] A README/document section serves a reader task rather than explaining the
      author's framing or writing process.
- [ ] Leftover locale changes are complete in all four languages.
- [ ] Relevant unit/browser assertions and the V2 user-facing copy contract
      pass.

## Validation

Run the checks that cover the changed surface:

```bash
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
mise run build:renderer
mise run format:check
```

The V2 test suite contains a focused source contract for reviewed forbidden
phrases. It is a regression guard for known implementation narration, not a
substitute for human review of meaning and context.
