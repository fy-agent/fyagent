# omp implementation review input

This technical summary preserves the relevant decisions from the implementation review. Private CLI events, local execution paths and account material are kept outside the repository. Official protocol sources are linked from [omp-official-input.md](omp-official-input.md).

- Register omp as a model configuration capability. Derived MCP/Skills assignment lists remain limited to capabilities implemented by the application.
- Resolve the active profile and effective `.yml`/`.yaml` before editing; preserve legacy JSON and let omp perform its own migration. Default profile directory overrides do not affect named profiles.
- Patch selected provider/model fields through a lossless document tree. Preserve other models, headers, unknown fields, comments and authentication references. `!command` is an opaque string reference; editing does not execute it.
- Save provider models independently from the optional default role. Preview binds native paths, complete preimages and the intended provider/model update. A two-file failure restores only bytes still owned by this save and preserves later external edits.
- Native read/preview DTOs contain redacted summaries and opaque revisions. A confirmation refers to an in-memory plan identity; credentials are not retransmitted in the confirmation or returned in previews.
- CLI acceptance checks the official binary version, effective config path, exact provider/model selector and default role, including stderr. Model listing proves parsing; authenticated generation is a separate acceptance step.

The dedicated omp service does not require extending the generic provider AppType. Its adapter remains distinct from Pi's JSON configuration and preserves additive provider semantics through targeted YAML updates.
