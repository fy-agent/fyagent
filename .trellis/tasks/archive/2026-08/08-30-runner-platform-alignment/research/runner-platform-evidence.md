# Runner platform evidence — 2026-08-30

## Official evidence

1. GitHub announced `windows-11-vs2026-arm` as GA and scheduled the
   `windows-11-arm` alias migration for 2026-09-21 through 2026-09-30:
   https://github.blog/changelog/2026-08-20-windows-11-arm64-vs2026-image-generally-available/
2. The GitHub-hosted VS2026 ARM64 image includes Visual Studio major version
   18 and both ARM64 and x64/x86 VC tools components:
   https://github.com/actions/runner-images/blob/main/images/windows/Windows11-VS2026-Arm64-Readme.md
3. Microsoft documents bounded `vswhere` ranges such as `[1.0,2.0)`:
   https://github.com/microsoft/vswhere/wiki/Versions
4. Microsoft identifies `vswhere` as the supported Visual Studio instance
   discovery utility:
   https://learn.microsoft.com/en-us/visualstudio/install/tools-for-managing-visual-studio-instances
5. GitHub job/container actions are Linux execution mechanisms, not native
   Windows/macOS substitutes:
   https://docs.github.com/actions/using-jobs/running-jobs-in-a-container
6. Tauri documents Linux/macOS-to-Windows NSIS cross-compilation as less
   tested and a last resort when native VM/CI is unavailable:
   https://v2.tauri.app/distribute/windows-installer/

## Repository evidence

- Portable jobs currently request `macos-15` in CI and Release.
- ARM64 matrices repeat `windows-11-arm` across CI build/proof/sign/seal paths.
- `windows-msvc-env.mjs` and `system-check.mjs` bind discovery to
  `[17.0,18.0)` and the x64/x86 component.
- Fake Bash/static contract owners reject Linux even though they do not claim
  native behavior on non-Windows/non-macOS hosts.
- Archived `08-07-modernize-ci-and-release` research explicitly rejects
  `ImageOS`/`ImageVersion` as authoritative metadata. This task preserves that
  decision and records bounded VS/MSVC facts instead.
