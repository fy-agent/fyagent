# FyAgent development knowledge

This directory contains maintained architecture explanations and operator
runbooks. Current code, configuration, tests, and workflows are the executable
authority. These documents explain how those pieces connect, but they do not
create a second implementation contract.

For ordinary work, start with the responsibility below, then inspect its
current implementation and tests. Retained notes under `.trellis/spec/` are
optional AI-assistance reference material. They are not required to
contribute, build, check, run CI, or release FyAgent. Archived tasks and Git
history are evidence only for an explicit historical investigation.

## Responsibility map

| Area                                 | Maintained explanation                                                                 | Executable evidence                                                                                     |
| ------------------------------------ | -------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| Knowledge and architecture ownership | [Architecture ownership](architecture/ownership.md)                                    | Current code, configuration, tests, workflows, and generated task metadata                              |
| FyAgent Windows installer            | [Windows installer](windows/installer.md)                                              | `src-tauri/nsis/`, Windows Tauri configuration, installer contracts, and native runner jobs             |
| Windows runtime security             | [Windows and Codex Desktop flow](windows/codex-desktop.md)                             | Windows runtime code, native adapters, Rust tests, and Windows contract tests                           |
| Codex Desktop lifecycle              | [Windows and Codex Desktop flow](windows/codex-desktop.md)                             | Codex Desktop services/adapters plus Rust and renderer tests                                            |
| Six-agent install chain              | [Agent install contract](agent-install-contract.md)                                    | `src-tauri/src/agent_install/`, `agent_install_*` commands, and their tests                             |
| Codex provider configuration         | [Codex provider flow](configuration/codex-provider.md)                                 | Provider services, Tauri commands, typed renderer facades, and their tests                              |
| WorkBuddy configuration              | [WorkBuddy flow](configuration/workbuddy.md)                                           | WorkBuddy services, typed renderer flows, and their tests                                               |
| CI                                   | [CI flow](ci-release/ci.md)                                                            | `.github/workflows/ci.yml`, classifier/aggregate scripts, and contract tests                            |
| Release                              | [Release flow](ci-release/release.md)                                                  | `.github/workflows/release.yml`, release scripts, and contract tests                                    |
| Local tools and tasks                | [mise development flow](tooling/mise.md) and [generated task reference](mise-tasks.md) | Version/lock files, `mise.toml`, `.mise/tasks/`, task scripts, and task/docs/environment contract tests |
| Validation and evidence              | [Validation guide](validation.md)                                                      | Targeted tests, `mise run check`, and matching native or remote evidence                                |

## Version words that remain intentional

Do not remove a version string merely because it contains `v1` or another
number. Real compatibility identities remain current, including:

- the `fyagent://v1/import` deep-link protocol;
- release and build metadata schema identities;
- WorkBuddy's third-party `/v1` API path;
- pinned toolchain, Action, runner, operating-system, and installer-tool
  versions.

Product-stage labels and fixed past-release narratives are not current
authority. Historical public release notes remain historical records and are
not rewritten as part of current development documentation.
