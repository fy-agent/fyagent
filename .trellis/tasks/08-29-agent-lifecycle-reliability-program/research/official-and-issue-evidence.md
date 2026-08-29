# Official and issue evidence

## Repository issues

- #31: https://github.com/fy-agent/fyagent/issues/31
  - Requires listing multiple installations, explicit target choice and failure-safe updates.
- #47: https://github.com/fy-agent/fyagent/issues/47
  - Existing installation defaults to preserve/readback; multi-install drift requires user authority selection.
- #101: https://github.com/fy-agent/fyagent/issues/101
  - Agent directory is the capability SSOT; observation and apply/write are separate.
- #141: https://github.com/fy-agent/fyagent/issues/141
  - Current-main UAT revalidation ledger, including inert controls and Windows detection/version drift.
- #68: https://github.com/fy-agent/fyagent/issues/68
  - FyAgent Windows Authenticode release work; kept outside this program.
- #71: https://github.com/fy-agent/fyagent/issues/71
  - FyAgent's own recoverable update channels; kept outside this program.

Issue metadata is currently inconsistent in places: #47 and #101 contain P0 wording while their labels/milestones are P1/v0.5. Execution should follow the concrete safety requirements and reconcile priority metadata separately instead of silently choosing one interpretation.

## Windows inventory sources

- Microsoft Uninstall registry contract:
  https://learn.microsoft.com/en-us/windows/win32/msi/uninstall-registry-key
  - Provides installer-owned fields such as DisplayName, DisplayVersion and product-code keyed records.
- Microsoft application registration / App Paths:
  https://learn.microsoft.com/en-us/windows/win32/shell/app-registration
  - App Paths is the preferred registered executable location; both per-user and machine registration exist.
- Windows PackageManager:
  https://learn.microsoft.com/en-us/uwp/api/windows.management.deployment.packagemanager
  - Supports enumerating package identity, publisher, version and install location for packaged applications.

Conclusion: no single source is authoritative for all desktop formats. FyAgent needs a bounded evidence aggregator and explicit conflict handling, not a wider fixed-directory scan.

## macOS replacement source

- Apple FileManager replacement contract:
  https://developer.apple.com/documentation/foundation/filemanager/replaceitem(at:withitemat:backupitemname:options:resultingitemurl:)
  - Apple documents replacement as preserving data-loss safety semantics and supports a backup item.

Conclusion: use a staged replacement/rollback transaction around a verified selected bundle. A permission failure during update must not silently select a different destination.

## Auth sources

- Claude Code CLI:
  https://docs.anthropic.com/en/docs/claude-code/cli-reference
  - Official login/logout/status commands; status supports machine-readable outcome.
- OpenCode CLI:
  https://opencode.ai/docs/cli/
  - `opencode auth login/list/logout` are provider credential operations, not one global account state.
- Grok CLI:
  https://docs.x.ai/build/cli/reference
  - Official login/logout are documented; no equivalent structured global auth-status command is documented in the reviewed reference.

Conclusion: adapter capabilities differ. Unsupported verification remains unknown/awaiting-user and must never be fabricated from credential-file existence.

## Frontend sources

- Radix Tabs:
  https://www.radix-ui.com/primitives/docs/components/tabs
  - Controlled/uncontrolled operation, activation modes and full keyboard navigation.
- React render purity:
  https://react.dev/reference/rules/components-and-hooks-must-be-pure
  - Side effects and state changes should not occur during render.
- React lazy:
  https://react.dev/reference/react/lazy
  - Defers module code until the component is first rendered.
- TanStack Query enabled/disabled behavior:
  https://tanstack.com/query/latest/docs/framework/react/guides/disabling-queries
  - Hidden/inactive surfaces can stop automatic fetch/refetch through declarative enablement.

Conclusion: FyAgent can solve Tabs, route loading and inactive-query behavior with already adopted primitives rather than another interaction framework.
