# Thinking Guides

> **Purpose**: Expand your thinking to catch things you might not have considered.

---

## Why Thinking Guides?

**Most bugs and tech debt come from "didn't think of that"**, not from lack of skill:

- Didn't think about what happens at layer boundaries → cross-layer bugs
- Didn't think about code patterns repeating → duplicated code everywhere
- Didn't think about edge cases → runtime errors
- Didn't think about future maintainers → unreadable code

These guides help you **ask the right questions before coding**.

---

## Available Guides

| Guide                                                         | Purpose                                                           | When to Use                         |
| ------------------------------------------------------------- | ----------------------------------------------------------------- | ----------------------------------- |
| [Code Reuse Thinking Guide](./code-reuse-thinking-guide.md)   | Prefer existing/adopted/open-source capabilities; share early     | Before writing new reusable code    |
| [Cross-Layer Thinking Guide](./cross-layer-thinking-guide.md) | Think through data flow across layers                             | Features spanning multiple layers   |

---

## Quick Reference: Thinking Triggers

### When to Think About Cross-Layer Issues

- [ ] Feature touches 3+ layers (Rust host, Tauri IPC, renderer port/facade, React)
- [ ] Data format changes between layers
- [ ] Multiple consumers need the same data
- [ ] You're not sure where to put some logic
- [ ] You are adding a Tauri command, event, DTO field, or config field
- [ ] UI / command code starts casting raw payload fields directly
- [ ] Product UI may belong in V2 (`src/v2`) rather than leftover legacy `src/` code
- [ ] Native window geometry (maximize / min-size / work area) and renderer
      chrome (Overlay drag) are changing together
- [ ] Agent Catalog install/auth or Codex managed-account identity is changing
      (closed `agentId+action` only; `credential_id` ≠ workspace id; native
      `auth.json` only when the live store is `file`)
- [ ] WorkBuddy / QoderWork CN / TRAE Work CN install detection or launch is
      changing (closed macOS bundle id and Windows PE ProductName together;
      never infer from `~/.workbuddy` and never `CreateProcess` on Windows)
- [ ] An interactive `mise run` task or `scripts/tasks/` process spawn is
      changing (interactive requires `raw`; POSIX `kill(-pid)` and Windows
      `taskkill /T /F` must be specified together)
- [ ] A hardware serial / ASR / focus-typing path is changing (native owns
      the COM reader; React only reads snapshots; device SiliconFlow and
      desktop Agent configs stay separate)

→ Read [Cross-Layer Thinking Guide](./cross-layer-thinking-guide.md).
  For V2 routes or shell chrome, read the
  [V2 Shell Contract](../frontend/v2-shell.md). For host maximize/min-size,
  read the [Main Window Layout Contract](../backend/main-window-layout.md).
  For desktop-agent identity, read
  [External Agent P0 Safety](../backend/external-agent-p0.md) and
  [Windows Runtime Security](../backend/windows-runtime-security.md).
  For mise `dev` / process trees, read
  [Repository Task Runner](../backend/task-runner-contract.md).

### When to Think About Code Reuse

- [ ] **You're writing any new component, service, helper, hook, parser, adapter, or CSS**
- [ ] You're writing similar code to something that exists
- [ ] A sibling route or later module is likely to need this
- [ ] The current repository/dependency stack may already provide the needed capability
- [ ] No current owner exists and an external open-source module/component may avoid a bespoke implementation
- [ ] You're considering adding a new npm package or Rust crate
- [ ] You're adding a new field to multiple places
- [ ] **You're modifying any constant or config**
- [ ] **You're creating a new utility/helper function** ← Search first!
- [ ] **You're adding a new UI component or page chrome** ← If another module will use it, put it in `shared/` now
- [ ] **You've discovered code with a concrete second consumer** ← Propose/promote one shared owner now
- [ ] Two files read the same untyped payload field with local casts
- [ ] Multiple branches update the same derived state from `kind` / `action`

→ Read [Code Reuse Thinking Guide](./code-reuse-thinking-guide.md).
  For renderer placement and V2 feature chrome, read the
  [Frontend Reuse Contract](../frontend/reuse.md). For Rust/Tauri owners and
  dependency decisions, read the [Backend Reuse Contract](../backend/reuse.md).

### When Verifying AI Cross-Review Results

- [ ] Reviewer claims "user input can be malicious" → Check the actual data source (internal manifest? user config? external API?)
- [ ] Reviewer flags "missing validation" → Is the data from a trusted internal source?
- [ ] Reviewer says "behavior change" → Read the code comments — is it intentional design?
- [ ] Reviewer identifies a "bug" in test → Mentally delete the feature being tested — does the test still pass? If yes → tautological test

**Common AI reviewer false-positive patterns**:

1. **Trust boundary confusion**: Treating internal data (bundled JSON manifests) as untrusted external input
2. **Ignoring design comments**: Flagging intentional behavior documented in code comments as bugs
3. **Variable misreading**: Not tracing a variable to its actual definition (e.g., Map keyed by path vs name)

**Verification rule**: Every CRITICAL/WARNING finding must be verified against
the actual data source, call chain, tests, and documented design before
prioritizing it.

---

## Pre-Modification Rule (CRITICAL)

> **Before changing ANY value, ALWAYS search first!**

```bash
# Search for the value you're about to change
rg -n --fixed-strings "value_to_change" <relevant-paths>
```

This single habit prevents most "forgot to update X" bugs.

---

## How to Use This Directory

1. **Before coding**: Skim the relevant thinking guide
2. **During coding**: If something feels repetitive or complex, check the guides
3. **After bugs**: Add new insights to the relevant guide (learn from mistakes)

Guides contain short cross-task triggers, questions, and pointers only. Put
concrete product signatures, DTO fields, commands, security rules, validation
matrices, and test requirements in the owning `backend/` or `frontend/`
code-spec instead of duplicating them here.

---

## Contributing

Found a new "didn't think of that" moment? Add a reusable trigger to the
relevant guide, or update the owning code-spec when the learning is a concrete
implementation contract.

---

**Core Principle**: 30 minutes of thinking saves 3 hours of debugging.
