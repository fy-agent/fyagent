# mise development flow

The standard version files, `mise.toml`, `mise.lock`, `.mise/tasks/`, and task
scripts define the local tool and command surface. Their contract tests verify
tool sources, uv/Python behavior, host-native execution, argument transport,
effects, composition, and maintenance safety. Retained development-environment
and task-runner notes under `.trellis/spec/` are optional AI-assistance
references rather than command prerequisites.

## Entry points

After reviewing a new checkout, use this standalone sequence:

```bash
mise trust
mise run bootstrap
mise run system:check
mise run dev
```

`mise trust` is a manual developer security decision; no repository task runs
it automatically. Routine work then uses `mise run <task>`, and
`mise run check` is the complete current-host pre-commit gate. The lock covers
every development host listed in `mise.toml` `settings.lockfile_platforms`.
Shipped desktop product evidence remains Windows and macOS GitHub Actions jobs.
GitHub Actions
deliberately installs and runs its native toolchain without mise. The generated
[task reference](../mise-tasks.md) is the complete command catalog and must be
regenerated from task metadata rather than edited by hand.
