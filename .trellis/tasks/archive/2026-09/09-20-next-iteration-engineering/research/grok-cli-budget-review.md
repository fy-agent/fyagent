# CLI budget execution review

Terminal Grok CLI, grok-4.6; isolated npm fixture used npm 11.6.2. Root acceptance is separate.

**No. The confirmed disk budget already names the frozen nested artifacts; npm still does not have to install that set.**

Preview/confirm/start share one `GrokNpmManifest` (root + platform + ordinary deps, including `@iarna/toml@3.0.0`). The start command does not. Grok argv is still `i -g <root>@<version>` only. That is the remaining identity hole. It is wiring, not a missing installer.

## Already fixed (do not re-litigate)

Metadata now fail-closes on unknown names, non-string specs, empty/zero sizes, and non-empty transitive `dependencies`. `@iarna/toml` `^3.0.0` / `~3.0.0` / `3.0.0` / `=3.0.0` is frozen to `3.0.0` and that exact version is integrity-checked on each mirror.

```498:512:~/.codex/worktrees/fyagent-next-cli-space/fyagent/src-tauri/src/services/tooling/grok_npm.rs
pub(super) fn resolve_declared_dependency(
    name: &str,
    spec: &str,
) -> Result<&'static str, GrokNpmPlanError> {
    match name {
        "@iarna/toml" => {
            if spec == "^3.0.0" || spec == "~3.0.0" || spec == "3.0.0" || spec == "=3.0.0" {
                Ok("3.0.0")
            } else {
                Err(GrokNpmPlanError::UnsupportedDependency)
            }
        }
        _ => Err(GrokNpmPlanError::UnsupportedDependency),
    }
}
```

`load_manifest_from_registry` sums `root + platform + deps` with `checked_add`. `storage_budget` then uses `required_free_space` (`checked_mul(3)`). That is an unpacked-size product reserve, not a vendor minimum.

Preview stores `PreparedPlanPayload::CliNpm(manifest)`; confirm consumes that exact `PreparedInstallTarget`; start passes the same manifest into Claude/Grok. Metadata fetches are JSON only; tarballs are not pulled before the disk gate.

Claude with no ordinary deps is already in the success shape: `plan_for_registry` + `--include=optional` + root pin. The hole is Grok’s ranged ordinary dep.

## Remaining issue

`GrokNpmInstallPlan` has no dependency field. `plan_for_registry` copies root integrity / platform name / platform integrity and **drops** `manifest.dependencies`. `npm_argv_for` pins only the root:

```246:269:~/.codex/worktrees/fyagent-next-cli-space/fyagent/src-tauri/user-helper/src/grok_npm.rs
    pub fn npm_argv_for(&self, tool: OfficialNpmTool) -> Vec<String> {
        let mut argv = vec![
            "i".to_string(),
            "-g".to_string(),
            format!("{}@{}", tool.package(), self.version),
            // registry flags only
        ];
        if tool == OfficialNpmTool::Claude {
            argv.push("--include=optional".to_string());
        }
        // no platform spec, no @iarna/toml spec
```

Two execution paths then drop even that plan:

1. **macOS Grok** ignores `plan_for_registry` and rebuilds `for_execution(target_version, registry, allow_scripts)` (`grok.rs` 921–986). Confirmed nested identity never reaches argv.
2. **Windows helper** encodes an 80-byte control of version + registry + allow-scripts only (`encode_plan_control` 397–411). Trailing bytes must be zero (`decode_plan_control` 450–455). Helper argv is reconstructed from that truncated plan (`windows.rs` 2428–2440, 650–667). Integrity env vars exist on the plan and are not sent.

npm’s documented install rule is: a package-spec installs that package **and whatever its package.json ranges resolve to at install time**, unless the published tarball itself carries shrinkwrap/lock. Host-side “we resolved `^3.0.0` → `3.0.0`” does not change npm’s resolver.

Today `@iarna/toml` 3.x has only `3.0.0`, so a live Grok install can _happen_ to match the budget. That is coincidence, not a lock. A later `3.0.1` would keep the confirmed budget on `3.0.0` while `npm i -g @xai-official/grok@…` installed `3.0.1`.

## Smallest viable integration (no new installer)

Keep the existing `npm i -g` owner. Put the **already-resolved exact ordinary deps** on that same argv, and make the helper plan control carry the same closed identity.

1. `plan_for_registry` copies `manifest.dependencies` into `GrokNpmInstallPlan` (allowlist stays `@iarna/toml` only; anything else remains `UnsupportedDependency`).
2. `npm_argv_for` appends `name@exact` after the root spec. Claude with an empty dep list stays unchanged.
3. Bump `GROK_NPM_PLAN_CONTROL_VERSION` inside the **existing 80-byte** frame (do not grow it; it is tied to `BRIDGE_CONTROL_BYTES`). Encode one closed dep tag + exact version in the current zero tail. Unknown tag / leftover non-zero bytes stay fail-closed.
4. macOS Grok `execute_official_npm` must call `plan_for_registry(&confirmed_manifest, …)` instead of `for_execution`. Otherwise the argv change never ships on macOS.

This is not a new helper action, not a lockfile writer, not overrides, not `npm ci`.

Bounded fake-registry fixture on this machine (`npm 11.6.2`, isolated `--prefix/--cache/--registry`, fake `parent@1.0.0` depending on `child@^1.0.0`, published `child@1.0.0` and `child@1.0.1`):

| argv                            | nested child                                             |
| ------------------------------- | -------------------------------------------------------- |
| `i -g parent@1.0.0`             | `child@1.0.1`                                            |
| `i -g parent@1.0.0 child@1.0.0` | `child@1.0.0` **deduped** (hoisted, nested, and shallow) |

So an extra **exact** spec on the existing global install command _does_ freeze that nested edge on current npm, via arborist dedupe, not by magic. Do not treat a bare extra global as a generic lock: it worked here because `1.0.0` satisfies `^1.0.0` and both specs are in one `npm i -g` tree. The closed allowlist is what keeps that from becoming a package manager.

Do not add `@iarna/toml` as a hardcoded helper global. The helper only sees the 80-byte plan; a Grok release without that dep would then install a package the confirmed budget did not name.

## Adjacent invariant (only as it touches this identity)

macOS preflight queries real `npm prefix -g` and `npm config get cache`, plus FyAgent scratch. It does not query `tmp`. npm 7+ documents `tmp` as unused; cacache lives under `cache`. Windows **host** preflight checks Local/Roaming AppData, not those npm paths. Windows **helper** preflight checks prefix writability only. That does not replace the argv pin; it is the volume set the 3× reserve is applied to.

## Smallest falsifiable acceptance test

Unit, no live Grok/Claude install:

1. Manifest with `@iarna/toml` `^3.0.0` → `plan_for_registry` argv contains `@xai-official/grok@1.2.3` **and** `@iarna/toml@3.0.0`, and does not contain `@latest`.
2. Claude manifest with empty `dependencies` → argv has no `@iarna/toml`.
3. Control v2 round-trips that extra spec; v1 payload with a dirty tail still returns `Missing`; unknown dep tag fail-closes.
4. A Grok macOS plan built the way `execute_official_npm` now builds (`for_execution(version, registry, allow)`) **must fail** a new assertion that argv includes the confirmed ordinary dep. That is the test that catches the current macOS drop.

Optional local fixture (temp prefix, fake registry, never `@xai-official/grok` / `claude-code`): `npm i -g parent@1.0.0` yields nested `child@1.0.1`; `npm i -g parent@1.0.0 child@1.0.0` yields nested `child@1.0.0` deduped. If that second command ever nested `1.0.1` beside a top-level `1.0.0`, extra argv is not a pin and this design should be rejected rather than papered over with a lockfile.

**Stop.** Antigravity can implement the argv + 80-byte control carry; do not invent a resolver.
