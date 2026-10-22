# Implementation and verification

1. Read root AGENTS and focused frontend/backend specs plus supplied official omp input and executor review notes. Inspect existing model, directory, capability, native storage and ACL patterns.
2. Implement the complete C feature inside this checkout with a dedicated service, typed commands/ports, software/catalog and model UI, capability-specific registration, installation/profile observations and deliberate save/restart feedback.
3. Add meaningful native, renderer and contract tests for the new behavior. Run Node 24.19.0 / pnpm 10.12.3 and Rust 1.97.1; use the repository native runner for Rust. Keep machine-specific execution evidence outside the repository.
4. Host independently reviews stable changes and verifies real omp against configuration produced by the implementation. Repair findings, update focused specs, then create a local commit with source/tests. Return SHA, test commands/counts and shared-file merge notes.

The user approved implementation and local commits. This record is implementation context, not a new consent gate. Scope is C; publication and combined acceptance belong to the parent.
