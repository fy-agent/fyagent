# Code Reuse Thinking Guide

> **Purpose**: Stop and think before creating new code — does it already exist,
> and if it is new, will another module need it?
>
> Frontend default: reuse existing owners; if a new component will be used by
> another module, put it in `shared/` on the first commit. See
> [Frontend Reuse](../frontend/reuse.md).

---

## The Problem

**Duplicated code is the #1 source of inconsistency bugs.**

When you copy-paste or rewrite existing logic:

- Bug fixes don't propagate
- Behavior diverges over time
- Codebase becomes harder to understand

---

## Before Writing New Code

### Step 1: Search First

```bash
# Search for similar function names
rg -n "functionName" <relevant-paths>

# Search for similar logic
rg -n "keyword" <relevant-paths>
```

### Step 2: Ask These Questions

| Question                                       | If Yes...                                                         |
| ---------------------------------------------- | ----------------------------------------------------------------- |
| Does a similar function exist?                 | Use or extend it                                                  |
| Is this pattern used elsewhere?                | Follow the existing pattern                                       |
| Could this be a shared utility?                | Create it in the right place                                      |
| Am I copying leftover `src/` UI into `src/v2`? | **STOP** — reuse V2 shared/widgets; read [V2 Shell](../frontend/v2-shell.md) and [Frontend Reuse](../frontend/reuse.md) |
| Will another route or module use this new component? | Put it in `src/v2/shared/ui` or `shared/features` on the first commit |
| Am I copying code from another file?           | **STOP** - extract to shared                                      |

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

Frontend default (see [Frontend Reuse](../frontend/reuse.md)): reuse existing
owners; if a new component will be used by another module, put it in
`shared/` on the first commit. Do not wait for a third copy of page chrome.

**Share on the first commit when**:

- An existing shared owner already does this job
- Another current route, widget, or leftover feature will use it
- A sibling module is expected next (the other five product routes, Skills vs
  MCP, Prompts vs Memory, TRAE vs OpenCode, catalog vs feature lists)

**Keep it local when**:

- Only this page, with no plausible second consumer
- One-off form or single dialog
- Trivial one-liner where a shared wrapper would be heavier than the copy

**Don't**: treat "appears 3+ times" as the frontend trigger for tabs, search,
lists, assignment rows, or other chrome sibling routes already have.

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
- [ ] No copy-pasted logic that should be shared
- [ ] New multi-module UI landed in `shared/`, not `pages/<route>/`
- [ ] Feature tabs / search / lists use `FeatureTabs` / `FeatureSearch` /
      `FeatureList` instead of a page-local fork
- [ ] No repeated untyped payload field extraction outside a shared decoder
- [ ] Constants defined in one place
- [ ] Similar patterns follow same structure
- [ ] Reducer/action transitions live in one reducer or command dispatcher
