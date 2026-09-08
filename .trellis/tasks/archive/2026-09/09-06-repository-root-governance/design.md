# Native tool configuration discovery

Use config/{vite,vitest,playwright,playwright.performance,postcss}.config.\* and
config/dependency-cruiser.cjs as real implementations. The exhaustive source /
destination / keep-at-root table is parent research/root-files.md. No forwarding
root, dynamic loader or duplicate configs. Retain eslint.config.mjs and
tsconfig.json to preserve standard editor/CLI discovery and project boundaries.

All public pnpm/mise task names remain stable. Internally use the tools' existing
--config options; this includes unit watch, native-fetch, desktop mock and
both Playwright servers. Explicitly anchor root, aliases, envDir, build output,
testDir and server cwd to the original locations via node:url/path. Vite and
Vitest both point PostCSS search to config/, preserving installed autoprefixer.
No automatic fallback to another config when the selected one is missing.

Update tsconfig include and ESLint file globs so moved configuration code remains
checked. Keep the two Vitest environment projects and all browser projects;
record pre/post collection. Preserve Node flags, MSW setup, task argument denial,
host-native target guards and release mocks. No uncontrolled caller --config
pass-through is added to the task API.

Move CI classifiers, labeler, dependency/deprecation scan inventories and
architecture tests in the same work batch. Scope-correct structural summaries
are regenerated only for inspected affected files. Search current docs/SPEC
links and live JSONL contexts for old paths; historical prose stays historical.

Add a focused root-governance contract to the existing architecture/tooling test
suite: known root discovery roles permitted, new unexplained code/config files
rejected, moved paths must exist and old config paths must not be active. Use
existing fs/TypeScript/CI utilities rather than a new repository-management tool.

Exact .DS_Store cleanup is allowed after ignore/ownership verification. Unknown
untracked files are not swept away; no git clean -fdx, reset or history rewriting.
Rollback preserves the complete config+consumer set together, not half a move.
