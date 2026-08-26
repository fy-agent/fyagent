# Code Reuse Thinking Guide

> **Purpose**: Stop and think before creating new code — does the capability
> already exist in FyAgent, in an adopted dependency, or in a suitable
> open-source module/component; and if it is genuinely new, should it become a
> shared project owner immediately?
>
> Frontend implementation rules live in
> [Frontend Reuse](../frontend/reuse.md). Backend implementation rules live in
> [Backend Reuse](../backend/reuse.md).

---

## The Problem

**Duplicated code is the #1 source of inconsistency bugs.**

When you copy-paste or rewrite existing logic:

- Bug fixes don't propagate
- Behavior diverges over time
- Codebase becomes harder to understand

---

## Reuse Decision Order

Use this order before choosing a bespoke implementation:

1. **Existing FyAgent owner** — reuse or minimally extend the current shared
   component, service, parser, helper, facade, primitive, or platform owner.
2. **Already-adopted dependency/framework primitive** — prefer capabilities
   already present in `package.json`, `Cargo.toml`, the standard library, or the
   current framework stack instead of adding a parallel implementation.
3. **Mature open-source module/component** — when the first two levels do not
   solve the problem, research maintained external candidates before writing a
   local replacement.
4. **Shared project adapter/owner** — if an external primitive needs FyAgent
   semantics, wrap or compose it once at the correct shared boundary rather
   than exposing package-specific glue to every caller.
5. **Bespoke implementation** — only after the earlier options are unsuitable.
   Record the concrete reason in the task/design/review artifact when the choice
   is non-trivial or likely to be questioned later.

This is a preference order, not permission to add dependencies blindly. A
small existing owner is preferable to a large new framework, and security /
architecture boundaries remain stronger than reuse convenience.

---

## Before Writing New Code

### Step 1: Search First

```bash
# Search for similar function names
rg -n "functionName" <relevant-paths>

# Search for similar logic
rg -n "keyword" <relevant-paths>
```

Search the dependency manifests too. If the repository has no suitable owner,
research external candidates using primary sources such as the project/package
official documentation and repository rather than assuming a library exists or
is still maintained.

### Step 2: Review Open-Source Candidates

Before adding a new dependency, verify at least:

- the required capability is actually supported by the reviewed version;
- license is compatible with the repository's distribution model;
- project/release activity and ownership are credible enough for the use case;
- security/provenance and known advisory exposure are acceptable;
- macOS / Windows production support and any Linux development-host needs fit;
- runtime/bundle/build cost and transitive dependency footprint are reasonable;
- the candidate fits existing UI/domain boundaries instead of introducing a
  second competing framework or owner;
- the public API is stable enough that one shared FyAgent adapter can contain
  churn if upstream changes.

If those checks fail, move to the next candidate or justify a local solution.
Do not replace "reinventing the wheel" with "adding a dependency for every
one-line helper".

### Step 3: Ask These Questions

| Question                                       | If Yes...                                                         |
| ---------------------------------------------- | ----------------------------------------------------------------- |
| Does a similar function exist?                 | Use or extend it                                                  |
| Is this pattern used elsewhere?                | Follow the existing pattern                                       |
| Does an adopted dependency already solve it?   | Reuse that primitive through the current project boundary          |
| Is there no local/adopted solution?             | Research suitable maintained open-source candidates                |
| Could this be a shared utility?                | Create it in the right place                                      |
| Am I copying leftover `src/` UI into `src/v2`? | **STOP** — reuse V2 shared/widgets; read [V2 Shell](../frontend/v2-shell.md) and [Frontend Reuse](../frontend/reuse.md) |
| Will another route or module use this new component? | Put it in `src/v2/shared/ui` or `shared/features` on the first commit |
| Am I copying code from another file?           | **STOP** - extract to shared                                      |

### Step 4: Promote Reusable Discoveries Early

Implementation work often reveals reusable code that was not obvious during
planning. When a capability has multiple current consumers or a concrete
near-term sibling consumer, do not leave the first implementation trapped in a
page/service-local file and wait for copy-paste pressure.

- Frontend: propose/promote one shared component/helper at the appropriate
  `shared/**` owner on the first implementation.
- Backend: propose/promote one crate-scoped service/helper/facade owner and keep
  the public surface minimal; "shared/public project component" does **not**
  mean making internal Rust modules broadly `pub`.
- Cross-layer: keep one semantic owner and adapt at boundaries; do not create a
  second business rule merely because callers use different transports.

If sharing would introduce speculative parameters or a fake abstraction for a
single one-off case, keep it local and document why a second consumer is not
yet concrete.

---

## Common Duplication Patterns

### Pattern 1: Copy-Paste Functions

**Bad**: Copying a validation function to another file

**Good**: Extract to shared utilities, import where needed

### Pattern 2: Similar Components

**Bad**: Creating a new component that's 80% similar to existing

**Good**: Extend existing component with props/variants

### Pattern 3: Repeated Constants

**Bad**: Defining the same constant in multiple files

**Good**: Single source of truth, import everywhere

### Pattern 4: Repeated Payload Field Extraction

When two or more consumers read the same Tauri, event, or configuration payload
field, first locate the existing owner: a V2 feature port, a legacy API facade,
a domain type, or a schema. Put shared decoding there instead of another local
cast. For the wire-contract rules, read
[Frontend Type Safety](../frontend/type-safety.md); for V2 placement, read
[V2 Shell](../frontend/v2-shell.md).

---

## When to Share

Repository default: reuse existing owners first. If a genuinely new capability
has a concrete second consumer, make one shared owner on the first
implementation rather than waiting for a third copy.

**Share on the first commit when**:

- An existing shared owner already does this job
- Another current route, widget, or leftover feature will use it
- A sibling module is expected next (the other five product routes, Skills vs
  MCP, Prompts vs Memory, TRAE vs OpenCode, catalog vs feature lists)
- Two backend services need the same parsing, filesystem, HTTP, archive,
  platform, validation, or orchestration primitive

**Keep it local when**:

- Only this page, with no plausible second consumer
- One-off form or single dialog
- Trivial one-liner where a shared wrapper would be heavier than the copy

**Don't**: treat "appears 3+ times" as the trigger when a second real consumer
is already known. Also do not generalize a one-off solely to satisfy an
abstraction rule.

---

## After Batch Modifications

When you've made similar changes to multiple files:

1. **Review**: Did you catch all instances?
2. **Search**: Run `rg` to find any missed
3. **Consider**: Should this be abstracted?

### Reducers Should Use Exhaustive Structure

When state is derived from action-like values (`action`, `kind`, `status`,
`phase`), prefer one reducer over scattered `if/else` updates so the transition
table stays in one place. Display code should not re-implement pieces of that
table. For renderer state ownership, read
[State Management](../frontend/state-management.md).

---

## Checklist Before Commit

- [ ] Searched for existing similar code
- [ ] Checked already-adopted dependencies/framework primitives before adding a new one
- [ ] If no suitable local/adopted solution existed, researched maintained open-source candidates from primary sources
- [ ] Any new dependency passed capability, license, maintenance, security/provenance, platform, footprint, and architecture-fit review
- [ ] No copy-pasted logic that should be shared
- [ ] A newly discovered multi-consumer capability has one proposed/shared owner rather than parallel local copies
- [ ] New multi-module UI landed in `shared/`, not `pages/<route>/`
- [ ] Feature tabs / search / lists use `FeatureTabs` / `FeatureSearch` /
      `FeatureList` instead of a page-local fork
- [ ] No repeated untyped payload field extraction outside a shared decoder
- [ ] Constants defined in one place
- [ ] Similar patterns follow same structure
- [ ] Reducer/action transitions live in one reducer or command dispatcher
- [ ] Any bespoke solution chosen over viable reuse has a concrete recorded reason
