# Repository Root and Tool Configuration

## 1. Scope / Trigger

Read before adding root files or relocating build/test/graph configuration.
This contract owns placement and discovery; the [Task Runner](./task-runner-contract.md)
still owns supported commands, host guards and effects. Product modules follow
[Renderer Directory Structure](../frontend/directory-structure.md).

## 2. Signatures and entries

| File                                      | Discovery / caller                                                                       |
| ----------------------------------------- | ---------------------------------------------------------------------------------------- |
| `config/vite.config.ts`                   | `vite --config config/vite.config.ts`; build and preview use the same explicit file.     |
| `config/vitest.config.ts`                 | All run/watch/desktop-mock/native-fetch aliases pass `--config config/vitest.config.ts`. |
| `config/playwright.config.ts`             | Functional browser CLI selects it explicitly.                                            |
| `config/playwright.performance.config.ts` | Serial production profiling CLI selects it explicitly.                                   |
| `config/postcss.config.cjs`               | Vite and Vitest `css.postcss` explicitly select the `config/` directory.                 |
| `config/dependency-cruiser.cjs`           | The architecture test passes the exact absolute config path and repository cwd.          |
| `eslint.config.mjs`                       | Remains at root for standard CLI/editor ancestor discovery.                              |
| `tsconfig.json`                           | Remains at root as the TS project and editor discovery boundary.                         |

Public operations remain `mise run dev:renderer`, `build:renderer`, `typecheck`,
`lint`, `test:unit`, `test:browser` and `test:performance`. No root forwarding
stubs or second configuration source are allowed.

## 3. Contracts

- Package manifests, locks, tool version authorities, Git metadata, AGENTS,
  licensing and project/contribution/security documentation remain at root.
  A file is not temporary merely because it is JavaScript or TypeScript.
- Explicit configurations resolve checkout/source directories with `import.meta.url`
  and Node URL/path APIs, never a workstation literal. Moving a config must not
  move `src`, its environment directory, `@` aliases, `dist`, test collection,
  fixture setup or assets. Both Playwright suites have explicit repository
  `webServer.cwd` and absolute repository `testDir`.
- Vite retains the established Rollup entry grouping, budgets and Tauri env
  prefix. PostCSS retains the same Autoprefixer configuration. Vitest retains
  renderer versus contracts projects; this is not a new product generation.
- TypeScript includes `config/**/*.ts`; ESLint covers TS and CJS configuration;
  formatting includes the same configuration directory. Dependency/deprecation
  scans include `config/`, not only deleted root files.
- CI classifies each current config independently. Retired names remain only
  in deletion/rename classification and historical evidence, not executable
  fallback imports or root proxies. Labels include current code/test paths.
- Confirmed ignored Finder metadata or task-owned scratch outputs may be
  removed individually. Never use blanket clean/reset to make the root neat.

## 4. Validation and failure matrix

| Condition                                 | Required result                                                                         |
| ----------------------------------------- | --------------------------------------------------------------------------------------- |
| New unexplained root code file            | Root-governance test fails; assign an owner or justify an actual discovery requirement. |
| Build reads config from the wrong cwd     | Real Vite loader/production build fails; fix paths, not a root re-export.               |
| Browser config moved                      | `testDir` and server cwd remain repository anchored; four sizes/WebKit still collect.   |
| Graph config moved                        | Same nonempty TS/runtime graph and negative fixtures are checked.                       |
| Old path appears in a historical Git diff | Correct domain classification, not unknown/no-op.                                       |
| Tooling test needs Vite/esbuild           | Run the loader in Node, not jsdom; never replace global typed arrays to satisfy it.     |

## 5. Good / Base / Bad

Good: move one authoritative config and change all supported explicit callers.
Base: retain root ESLint/tsconfig discovery because editors need it.
Bad: move everything by extension, delete a useful config as temporary, or keep
two real implementations connected by compatibility stubs.

## 6. Tests required

`tests/architecture/rootGovernance.test.ts` checks root code inventory, all six
real configurations, the actual Vite loader, PostCSS location, public aliases
and browser cwd/testDir. CI classifier tests exercise each file alone; graph
tests prove real coverage, not an empty scan. Re-run strict type/lint/format,
all unit tests, renderer build, browser and production gates; full prearchive
and integration checks follow the task workflow. Preserve effective SPEC/JSONL
links and do not rewrite Git history.

## 7. Wrong vs correct

Wrong: moving Playwright config while leaving `testDir: './tests/browser'`
and an implicit server cwd, causing execution under `config/`.
Correct: derive the repository directory with Node URL APIs and use it in
both `testDir` and `webServer.cwd`; the CLI chooses the only config explicitly.
