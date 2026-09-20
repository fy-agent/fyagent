# Delivery design

Use two ordered PRs: FDE branch `codex/fde-delivery-integration` targets main;
subscription PR #192 is based on that branch and contains the incremental changes.
Merge the verified FDE commit into the existing subscription worktree, preserving
both histories and resolving shared configuration, frontend and contract changes.

Combined persistence uses a new schema version. A migration must recognize and
complete both independent schema-22 shapes, preserving OpenCode proxy settings,
FDE projects, resource generations and verification evidence. Never downgrade or
manually rewrite a real database version. Add fixture tests for both variants,
schema 21, repeatability and preserved data.

Keep official branch/version and installed app unchanged. Prior isolated UAT
artifacts remain historical candidates and must not be represented as the new
combined binary. Remote PR links and exact commit check results are delivery proof.

The failed frontend job at `6a4af16d` identifies two concrete workstation paths in
the subscription UAT guide. Replace them with semantic placeholders; retain the
privacy contract and all existing quality budgets.
