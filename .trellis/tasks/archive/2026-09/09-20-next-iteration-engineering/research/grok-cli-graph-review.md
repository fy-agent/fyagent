# Grok npm budget-graph review

Reviewed immutable `b58163d6`. The initial bounded run reached 12 turns without a final report; the same session resumed from supplied immutable source excerpts and returned this report in one turn without tools. Session: `01a0c15f-388f-7333-a322-43aebe6dfc97`. CLI configured `grok-4.6`; runtime usage named `grok-4.6-build`.

GPT-6 accepts the three metadata-admission gaps as concrete source findings. The suggested prefix-only optional-package admission is too broad and is not accepted: the repair must use actual closed recognized platform package names/suffixes and preserve valid vendor sibling packages. No claim is made that the illustrative dependency fields exist in current published packages.

**Verdict:** The budget metadata path does **not** account for or reject all installable dependency metadata. It only freezes root `dependencies` to `@iarna/toml@3.0.0`, adds that child’s unpacked size if `dependencies` is empty, and adds one platform tarball’s `dist.unpackedSize`. `npm i -g` still follows the rest of the published graph. This is extra **packages**, not the already-disclosed 3× unpacked-size reserve.

Helper argv (`user-helper/.../grok_npm.rs:314-342`) is `npm i -g <pkg>@<ver> [@iarna/toml@3.0.0]` with Claude `--include=optional` and no `--omit=peer`. Extra optionals/peers/transitives of the root or platform package are installed even though they never enter `GrokNpmManifest.dependencies` or the closed argv dep.

---

### 1. Applicable extra root `optionalDependencies` are ignored, not rejected

`parse_published_root` (`grok_npm.rs:546-563`) reads only `optionalDependencies[platform]` for a version string. Other optional keys are not admitted and not rejected. They never reach `resolve_declared_dependency`.

**Accepted fixture** (root document):

```json
{
  "name": "@xai-official/grok",
  "version": "1.0.25",
  "dist": { "integrity": "sha512-<64-byte-b64>", "unpackedSize": 18363 },
  "dependencies": { "@iarna/toml": "^3.0.0" },
  "optionalDependencies": {
    "@xai-official/grok-darwin-arm64": "1.0.25",
    "left-pad": "1.3.0"
  }
}
```

`parse_published_root` returns `Ok`; `declared_dependencies` is only `@iarna/toml`. `left-pad` is installable on every host. Sibling _known_ platform optionals are not this bug (os/cpu skip); this is an extra applicable optional.

### 2. Platform package graph is never inspected

`load_manifest_from_registry` (`grok_npm.rs:407-426`) checks platform `name` / `version` / `dist.integrity` / `dist.unpackedSize` only. No `dependencies` / `optionalDependencies` / `peerDependencies` check.

**Accepted fixture** (platform document):

```json
{
  "name": "@xai-official/grok-darwin-arm64",
  "version": "1.0.25",
  "dist": { "integrity": "sha512-<64-byte-b64>", "unpackedSize": 29292216 },
  "dependencies": { "node-addon-api": "8.0.0" }
}
```

Budget counts `29292216` only. `npm` still installs `node-addon-api` under the optional platform package. Same hole on Claude’s `@anthropic-ai/claude-code-*` platform doc.

### 3. Root/child `peerDependencies` and child `optionalDependencies` are ignored

Child reject is only `dep_doc.dependencies` (`grok_npm.rs:447-454`). Missing/empty `dependencies` is success. Root `peerDependencies` is never read. Argv does not pass `--omit=peer` (npm 7+ auto-installs peers). Product already special-cases npm 12 `--allow-scripts`.

**Accepted fixture** (`@iarna/toml@3.0.0` document):

```json
{
  "name": "@iarna/toml",
  "version": "3.0.0",
  "dist": { "integrity": "sha512-<64-byte-b64>", "unpackedSize": 99960 },
  "dependencies": {},
  "optionalDependencies": { "foo": "1.0.0" },
  "peerDependencies": { "bar": "1.0.0" }
}
```

Also accepted: a root with the known Grok `dependencies`/`optionalDependencies` shape plus `"peerDependencies": { "bar": "1.0.0" }`.

Whether live `@iarna/toml@3.0.0` currently has those fields: **unknown** (no registry fetch). The admission path would still accept them.

---

### Smallest closed admission (keep current Grok/Claude paths)

In `src-tauri/src/services/tooling/grok_npm.rs` only; do not add a resolver or installer.

1. **`parse_published_root`:** Reject non-empty root `peerDependencies`. Walk every `optionalDependencies` key: allow iff it is a closed platform package for this product (`name` is `platform` or shares the `{package}-` prefix already used for Grok/Claude platform names). Unknown keys → `UnsupportedDependency`. Do not copy optionals into `declared_dependencies`. Keep current-platform version lookup and ordinary `dependencies` → `@iarna/toml` freeze.

2. **`load_manifest_from_registry`:** Reuse one empty-graph helper on `platform_doc` and `dep_doc`: if `dependencies`, `optionalDependencies`, or `peerDependencies` is present and non-empty → `UnsupportedDependency`. Missing or `{}` stays OK.

That still admits: Grok root `@iarna/toml` + sibling platform optionals; Claude empty ordinary deps + platform optionals; `@iarna/toml@3.0.0` with an empty child graph. It rejects the three fixtures above.

## GPT-6 published-shape readback

The public npm registry was read on 2026-09-21 before handing the repair to Cursor. Grok 1.0.34 declares `@iarna/toml: ^3.0.0` and six exact-version platform optionals; Claude Code 2.1.278 declares an empty ordinary dependency map and eight exact-version platform optionals. Neither root declares peers. The recognized set must include the actual vendor siblings (including their foreign-platform names) without implying additional first-party operating-system support. Unknown packages and nonempty or malformed platform/child dependency maps must fail closed.

Sources: <https://registry.npmjs.org/@xai-official%2Fgrok/latest> and <https://registry.npmjs.org/@anthropic-ai%2Fclaude-code/latest>. Only package identity/version/dependency fields were extracted. No package was installed by this readback.
