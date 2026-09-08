# Rewrite user-facing copy for clarity

## Goal

Make FyAgent's production interface and public documentation understandable
without exposing implementation notes, internal state-machine terminology, or
generic AI-generated marketing prose. A reader should be able to tell what the
product does, what an action changes, what happened, and what to do next.

## Background

- The production renderer is `src/v2/**` and currently uses hardcoded Chinese
  copy.
- Several current screens expose engineering language such as “真实配置回读”,
  “乐观成功状态”, “安装准备度”, “零写入预览”, “计划身份”, “Change Plan”, and
  “Change Job”. These phrases describe implementation safeguards rather than
  user outcomes.
- The root README family includes broad vision language and implementation
  validation details before some readers reach the concrete product path.
- Community criticism of “AI slop” is not a reliable phrase blacklist. The
  stable problem is copy that is generic, over-explained, mechanically
  symmetrical, promotional without evidence, or detached from the user's task.
- Safety behavior, rollback behavior, validation, and factual product limits
  must remain unchanged even when their presentation is simplified.

## Requirements

### R1 — Rewrite production interface copy

- Audit user-visible strings under `src/v2/**`, including headings,
  descriptions, empty states, notices, dialogs, progress text, tooltips, and
  accessible labels.
- Replace implementation narration with user-facing state, consequence, and
  next-step copy.
- Keep necessary product terms such as Provider, Skill, MCP, API Key, model ID,
  configuration path, backup, and rollback when users must recognize or act on
  them.
- Remove internal diagnostics that do not help users act, including backend
  event sequence numbers.
- Preserve all business logic, write boundaries, security checks, rollback
  behavior, and authoritative refresh/readback behavior.

### R2 — Audit leftover renderer copy

- Search the four registered locale files and leftover user-visible components
  for the same implementation-language and generic-copy patterns.
- Change all four locales together when a leftover translation key changes.
- Do not perform a broad legacy renderer refactor.

### R3 — Rewrite public documentation where needed

- Audit root public Markdown files and `docs/**`; exclude archived Trellis
  tasks, work journals, generated agent instructions, and private runtime
  artifacts.
- Rewrite the Chinese, English, and Japanese root READMEs so their opening,
  current capabilities, limits, setup path, and contribution information are
  concrete and consistent.
- Rewrite other public docs only where the prose is vague, promotional,
  mechanically repetitive, or narrates the writing/design process instead of
  helping the reader complete a task.
- Preserve technical precision in development docs, historical facts in
  changelogs/release notes, legal text, attribution, security instructions, and
  license meaning.
- Do not add claims that cannot be established from repository behavior or
  cited product sources.

### R4 — Persist a writing contract

- Add a frontend SPEC that defines acceptable user-facing copy, prohibited
  failure modes, review questions, examples, and validation expectations.
- Link the SPEC from the frontend index and pre-development checklist so future
  UI and public-documentation work must read it.
- Treat renderer copy and public repository documentation as product surfaces,
  while allowing exact technical language in internal contracts and developer
  references.

### R5 — Add regression protection

- Add a focused V2 copy contract that rejects the known implementation phrases
  from production presentation files while allowing internal type names, wire
  contracts, and developer-only code to remain unchanged.
- Update existing unit/browser expectations for rewritten copy.
- Run applicable V2, documentation, formatting, type, and repository checks
  before merge.

## Scope

### In scope

- `src/v2/**` user-visible copy.
- Clear matching offenders in leftover renderer components and
  `src/i18n/locales/{en,ja,zh,zh-TW}.json`.
- Root public Markdown files and `docs/**`, with primary attention to README,
  user manuals, guides, release-note introductions, and contributor/support
  entry points.
- `.trellis/spec/frontend/**` for the durable contract.
- Focused tests and test expectations directly coupled to changed copy.

### Out of scope

- Visual design, layout, navigation architecture, feature behavior, backend
  contracts, serialized values, or API identifiers.
- Archived Trellis tasks, work journals, generated assistant instructions, and
  historical task evidence.
- Rewording license texts or changing legal/compliance meaning.
- Translating the production V2 renderer or migrating it to the leftover i18n
  system.
- A punctuation-only purge. A phrase or punctuation mark is changed only when
  it harms clarity in context; there is no blanket ban on em dashes, headings,
  three-item lists, or other superficial “AI detector” signals.

## Acceptance Criteria

- [x] Production V2 no longer presents “真实配置回读”, “乐观成功状态”,
      “安装准备度”, “零写入预览”, “计划身份”, “后端事件序号”, “真实 Change
      Job”, or “真实 Change Plan”.
- [x] Success messages state the completed outcome; failure and uncertain
      states state what is known and give a useful next step without claiming
      success.
- [x] Confirmation dialogs explain the affected configuration and backup or
      rollback consequence in user terms, without leaking implementation
      tokens.
- [x] Root READMEs in Chinese, English, and Japanese lead with concrete current
      value and setup, clearly separate current capability from future
      direction, and remain mutually consistent.
- [x] A repository-wide audit of public Markdown is recorded; files without a
      clarity problem are left unchanged rather than mechanically rewritten.
- [x] The new frontend copy SPEC is indexed and includes executable review
      criteria plus bad/good examples.
- [x] A focused automated test prevents the identified implementation phrases
      from returning to production V2 presentation code.
- [x] Relevant unit, browser, type, format, and repository quality gates pass,
      or any environment-only gap is explicitly recorded with evidence.

## Open Questions

None. The user explicitly authorized task creation, implementation, archival,
push, pull request creation, and merge to `main`, and subsequently specified
that all work must occur directly on `dev/laiyongjie` without a new worktree.
