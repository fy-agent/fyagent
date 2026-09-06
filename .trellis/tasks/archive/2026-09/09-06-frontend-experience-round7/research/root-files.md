# Root-file disposition and relocation contract

Inventory at `4f8973ef`. Root contains seven JS/TS/CJS tool configurations;
inspection found no unexplained temporary MJS/TS script among them. File
extension is not evidence that a file is temporary. Do not delete a working
tool configuration to improve the visual file count.

## Planned migration — one config directory, no forwarding stubs

| Existing file                    | Intended owner                          | Specific relocation risk                                                                                |
| -------------------------------- | --------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| vite.config.ts                   | config/vite.config.ts                   | Explicit --config; source/root/envDir/@ alias/outDir must stay unchanged                                |
| vitest.config.ts                 | config/vitest.config.ts                 | Explicit --config in all runners; root and both project setups remain repo-relative                     |
| playwright.config.ts             | config/playwright.config.ts             | testDir/webServer cwd are config-relative; preserve four Chromium projects + WebKit                     |
| playwright.performance.config.ts | config/playwright.performance.config.ts | Serial production server, root cwd, exact Vite config in build/preview; no dev-server substitution      |
| postcss.config.cjs               | config/postcss.config.cjs               | Explicit search directory in both Vite and Vitest; retain autoprefixer rather than silently dropping it |
| .dependency-cruiser.cjs          | config/dependency-cruiser.cjs           | Pass exact config path; repository tsconfig and source traversal must not become empty                  |

Vite7/Vitest3 support explicit config paths; Playwright documents config-relative
test discovery and webServer cwd. No additional config loader or build wrapper
is needed. Use existing package/mise APIs, with explicit built-in flags. Local
pnpm exec vite/vitest users must use the documented package task or exact
config flag after migration. Do not retain root aliases as a second authority.

## Keep at root — explicit discovery / workspace / public-entry contract

| Files                                                                     | Reason                                                                                                                       |
| ------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| eslint.config.mjs                                                         | Standard ESLint10/editor ancestor discovery; keeping this small real entry avoids separate editor/CLI override configuration |
| tsconfig.json                                                             | TypeScript/editor project boundary and @ alias; update include to config/\*_/_.ts                                            |
| package.json, pnpm-lock.yaml, pnpm-workspace.yaml                         | Package manager and workspace root/lock authority                                                                            |
| mise.toml, mise.lock, .node-version, .python-version, rust-toolchain.toml | Existing environment/toolchain discovery and reproducibility contracts                                                       |
| pyproject.toml, uv.lock                                                   | Existing Python workspace/environment contract, not scratch data                                                             |
| .gitignore, .gitattributes, AGENTS.md                                     | Git and agent discovery; governed metadata                                                                                   |
| README.md, README_EN.md, README_JA.md, CHANGELOG.md                       | Public repository entry and release documentation                                                                            |
| LICENSE, LICENSING.md, COMMERCIAL-LICENSE.md, THIRD_PARTY_NOTICES.md      | License/distribution entry points; not clutter to delete                                                                     |
| CONTRIBUTING.md, CODE_OF_CONDUCT.md, SECURITY.md, SUPPORT.md              | Contributor/security/support discovery and repository governance                                                             |

ESLint is relocatable in principle, but then automatic editor discovery would
need compensating configuration. Prefer its standard entry here; do not move
files simply to create more loaders. The real implementations of moved configs
live only under config/; eslint and tsconfig are intentional remaining roots.

Root .DS_Store is an ignored OS artifact, eligible for exact-file cleanup after
checking ownership/ignore status. Do not run git clean -fdx, recursive deletion of
unknown untracked files, lock regeneration, or removal of .trellis/.runtime.
Any newly appearing unknown root file must be classified again before commit.

## All consumer groups must move together

Known live consumers from the initial search:

- package.json dev/build/lint/unit/watch/browser/performance/desktop/native-fetch scripts;
  .mise/tasks/frontend.toml command documentation and downstream task docs.
- Both Playwright webServer commands and cwd; Vite/Vitest aliases, roots,
  PostCSS directory and Vitest extends:true projects.
- eslint.config.mjs scoped files and tsconfig.json include.
- scripts/ci/classify-changes.mjs, .github/labeler.yml, and classifier tests.
- scripts/tasks/dep0040-check.mjs active-source/suppression inventory;
  platform structure inventories when their exact tracked entries are affected.
- tests/architecture/dependencyGraph.test.ts, rendererConsolidation.test.ts,
  task-runner/release/dev-build command assertions and source-root traversal.
- Current frontend quality/directory/appearance SPEC and backend CI/tooling
  contracts; contributor/developer docs; effective task JSONL references.

Search again after movement. Preserve Git history and historical prose; update
effective links/injection manifests when the target truly moved. Existing old
historical unresolved links are not evidence of new drift and are not silently
deleted. New root governance test should reject unexplained root code/config
additions and stale removed entry paths, using existing fs/TypeScript tooling.

## Acceptance beyond a successful build

Capture the collected Vitest suites and Playwright tests before/after migration;
all previously covered renderer/domain/contracts remain discoverable. Verify
real seven-route production boot, prefixing of CSS with existing autoprefixer,
nonempty dependency graph, lint/type coverage of config files, correct CI change
classification and full command contracts. Keep security and performance gates
unchanged; no broad exclusion or baseline-budget increase to pass relocation.
