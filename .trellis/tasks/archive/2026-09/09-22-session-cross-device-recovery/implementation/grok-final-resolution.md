# Final Grok 4.7 dispositions

Actual Cursor init: Grok 4.7 256K High; reviewer exited 0. Findings independently checked against current production source.

- P1 readback mismatch: accepted. `apply_readback` now records `Ambiguous`, matching reconciliation, and invalidates prior stronger evidence when native history contradicts it. A real production orchestrator test checks the ambiguous stage, no repeat write, and invalidation after a previously verified request stage.
- P2 empty selection: accepted. Empty `snapshotIds` now fails before receipt claiming or native writing. “All” callers must enumerate snapshots. Test fixture requests now list their actual package IDs; a counted-writer test requires zero writes and zero receipts for an empty selection.
- No additional receipt store or redundant parallel importer found. Existing provider-specific native projections remain necessary for exact source semantics and version gates.

Final canonical tests are recorded separately; the review exit alone is not acceptance evidence.
