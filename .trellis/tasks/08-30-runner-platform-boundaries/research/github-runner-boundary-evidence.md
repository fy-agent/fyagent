# Research: GitHub runner boundary evidence

## Checked facts

- GitHub's hosted-runner documentation lists explicit `ubuntu-24.04`,
  `windows-2025`, `windows-11-arm`, `windows-11-vs2026-arm`, and `macos-15`
  labels.
- GitHub announced the Windows 11 ARM64 Visual Studio 2026 image as generally
  available under `windows-11-vs2026-arm`.
- GitHub announced that `windows-11-arm` will migrate gradually to the VS2026
  image from 2026-09-21 through 2026-09-30. An explicit label avoids mixed
  image selection during that window.
- The published ARM64 VS2026 image manifest includes Visual Studio 2026 and
  both native ARM64 and x64/x86 VC tool components.
- `ImageOS` and `ImageVersion` remain implementation details rather than the
  repository's supported Release metadata contract. Existing FyAgent research
  and executable tests deliberately exclude them.

## Sources

- https://github.blog/changelog/2026-08-13-windows-server-2025-with-visual-studio-2026-and-windows-11-arm64-with-visual-studio-2026-images-are-now-generally-available-in-github-actions/
- https://docs.github.com/en/actions/how-tos/write-workflows/choose-where-workflows-run/choose-the-runner-for-a-job
- https://github.com/actions/partner-runner-images/blob/main/images/arm-windows-11-vs2026.md
- `.trellis/tasks/archive/2026-08/08-07-modernize-ci-and-release/research/github-runner-metadata-engineering.md`
- commit `d0af898a` (`fix(ci): harden native preflight contracts`)

## Decision

Use Ubuntu only for portable control-plane work, retain native platform jobs,
route Windows ARM64 explicitly to the VS2026 image, and expand reviewed local
VS discovery to 17.x/18.x. Do not reintroduce hosted-image implementation
variables or fabricate compiler provenance that is not bound to the actual
build process.
