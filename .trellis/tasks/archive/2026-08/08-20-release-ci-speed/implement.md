# Implement

1. Cherry-pick/port 0.4.2 notarize-once files onto this branch.
2. Relax eligibility + remote collector + tests (`devReleaseEligibility`, `devReleaseRemote`, `releaseWorkflow`).
3. Add registry-only cargo cache + Release pnpm cache; pin `cache: false` on rust toolchain and no `target/` paths.
4. Update spec + `docs/fyagent/development/ci-release/release.md`.

```bash
pnpm exec vitest run tests/devReleaseEligibility.test.ts tests/devReleaseRemote.test.ts tests/releaseWorkflow.test.ts tests/ciWorkflow.test.ts
```
