# Design

Shared CSS already supports this: `.fy-feature-header` is a wrapping flex row; `.fy-feature-header > .fy-feature-actions { margin-left: auto }` right-aligns buttons once a left sibling exists.

JSX: move `FeatureTabs` inside the header. Skills: always mount the header.

Skills `page.css`: scope `width: auto` to toolbar category tabs only.

Agent copy: `src/v2/pages/agents/intros.ts` keyed by `AgentCatalogId`. Render with existing `.fy-feature-intro` under a `section` labelled 「产品介绍」, after identity, before Codex installer / 支持的功能.

Sources: Qoder CN docs, TRAE Work docs, WorkBuddy/CodeBuddy docs on workbuddy.cn, xAI Grok Build, Anthropic Claude Code, OpenCode.
