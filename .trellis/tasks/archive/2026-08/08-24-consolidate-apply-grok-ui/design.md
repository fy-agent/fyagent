# Design

## Owned paths

- `src/v2/pages/models/apply/**`
- narrow integration seam inside `src/v2/pages/models/**` if it does not own shared ports
- focused V2 tests/styles for Apply UI
- current legacy locale/resource/tests directly required by #108

Do not edit shared platform port/composition files, Rust, database, command registration or Git state.

## Apply props

The component consumes a `ChangePlan`, nullable `ChangeJobSnapshot`, busy/error state and callbacks `onConfirm`, `onRegenerate`, `onClose`. It does not own persistence or simulate backend progress.

## View mapping

- Plan preview: neutral; Secret-blocked confirmation disabled.
- Running: neutral progress/timeline from real events.
- Succeeded/warning: configuration result plus explicit “尚无真实使用证据”.
- Writer failed with confirmed baseline: non-green failure.
- Mismatch/unavailable/recovery required: unknown/recovery styling and copy.
- Expired/stale/consumed/invalid digest: regeneration only.

Use one reducer/view-model owner, accessible live status and existing V2 primitives. No new shared chrome is warranted unless another current route already needs it.

## Grok migration

Compare #108’s intended file changes with current main and apply only product naming/copy/test deltas. Keep current locale schema and current Provider behavior intact.
