# Implement

1. Remove or empty `tests/currentDocsContract.test.ts`; drop from `release-check.mjs`.
2. Slim `taskDocs.test.ts`; drop duplicate Windows-Portable doc scan if it only duplicates deleted tests.
3. `.gitignore` `.tmp/`.
4. Update specs that required the deleted tests.
5. Only untrack binaries if raster/docs contracts are updated in the same change.

```bash
pnpm exec vitest run tests/taskDocs.test.ts tests/desktopSecurityBoundary.test.ts tests/localBuildBoundary.test.ts
```
