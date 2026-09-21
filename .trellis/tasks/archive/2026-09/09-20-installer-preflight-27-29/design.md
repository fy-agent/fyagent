# Design

The missing behavior lives in the lifecycle admission boundary and Agent directory action controller. The existing inventory owns opaque targets, revisions and native revalidation; the existing Codex service owns its install plan and job. Add a read-only preflight projection through these owners, with strict typed IPC, then use one confirmation surface in the existing directory UI.

Relevant checks run again at action admission before download, while final transaction guards remain authoritative. No new scheduler, helper command, installer URL, renderer path or arbitrary command is introduced. CLI runtime and scope observations reuse Tooling; desktop disk checks reuse the existing Codex disk probe. A size estimate remains an estimate and cannot become artifact admission.

Expected files: Agent lifecycle/preflight/types/command/ACL and typed port for the native boundary; the directory and lifecycle hook for confirmation and retained recovery; focused Codex service/installer hook changes if required to expose its existing preflight and bind target continuity. Tests and focused lifecycle/directory specs are updated with the changed contract. Existing helpers, atomic replacement and rollback are retained, with tests added only for identified missing behavior.

The UI retains the product's existing restrained developer-tool typography, dialog, target picker, button and notice tokens. It shows human-recognizable versions and sanitized location labels, never capability IDs, account SIDs or command lines.

Portable tests do not establish Windows Alice/Bob identity, signed helper image/pipe ACL behavior, PackageManager, UAC refusal, vendor wizard completion, macOS signing/Gatekeeper or physical installation. These remain separate native acceptance requirements.
