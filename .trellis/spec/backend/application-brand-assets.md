# Application Brand Asset Contract

## 1. Scope / Trigger

Read this contract before changing the FyAgent application icon, regenerating
Tauri icons, changing the About icon, or editing the macOS tray template. It
does not apply to third-party provider, Claude, OpenAI, screenshot, or DMG
background assets.

The application icon crosses renderer, Tauri bundle, Windows shell, and macOS
menu-bar boundaries. A valid change updates every consumer from one
approved source while preserving the established FyAgent application identity
and unrelated artwork.

## 2. Signatures

The maintained vector, canonical package source, and generation entry point
are:

```text
geometry: assets/fyagent-y-gate.svg
source:   assets/fyagent.png
format:   PNG, 1024x1024, RGBA with transparent corners
command:  mise run assets:icons -- --source assets/fyagent.png --apply
```

The direct consumers are:

```text
src-tauri/tauri.conf.json                         Tauri bundle icon list
src-tauri/tauri.windows.conf.json                 Windows setup/uninstaller ICO
src/assets/icons/app-icon.png                     renderer About icon
src-tauri/src/lib.rs                              embedded macOS 3x tray template
src-tauri/icons/tray/macos/statusTemplate.png     1x template
src-tauri/icons/tray/macos/statusTemplate@2x.png  2x template
src-tauri/icons/tray/macos/statusbar_template_3x.png 3x template
```

## 3. Contracts

- Preserve the audited For You Gate geometry in `assets/fyagent-y-gate.svg`.
  The generator verifies its exact digest and passive SVG contract before it
  deterministically renders `assets/fyagent.png`. Do not redraw, recolor,
  crop, or recomposite the geometry.
- `assets/fyagent.png` is the authoritative 1024 RGBA input to the Tauri
  generator. The task must first render and validate that repository path,
  then invoke Tauri with that PNG path; bypassing it with the SVG does not
  satisfy the package-source contract.
- Use the repository's Tauri CLI to generate the standard desktop, Windows
  Store, Android, and iOS files. Do not hand-maintain parallel resizers for
  those outputs.
- Keep every existing generated path, including `64x64.png`, unless a reviewed
  Tauri/toolchain migration explicitly changes the inventory.
- Copy `src-tauri/icons/32x32.png` byte-for-byte to
  `src/assets/icons/app-icon.png` for the About surface.
- `src/v2/shared/assets/fyagent-y-mark-transparent-128.png` is a separately
  reviewed V2 chrome mark. It is identity-sealed in the raster inventory and
  is not produced or overwritten by `assets:icons`. An application-icon
  regeneration must leave it unchanged unless a separate reviewed visual
  update includes it.
- macOS template images are the technical monochrome exception to the color
  preservation rule. Crop to the source alpha bounds, fit proportionally in
  an 18pt content box centered on a 24pt canvas, and emit black RGBA at 24,
  48, and 72 pixels. Preserve antialiased alpha; Tauri/macOS supplies the
  light/dark rendered color.
- Tauri's ICNS encoder may emit identical chunks in a nondeterministic order.
  Canonically sort complete, untouched ICNS chunks after generation, validate
  every PNG-backed chunk, and require the canonicalized complete-chunk bytes to
  be stable across repeat generation. This stricter container assertion may
  reject a pixel-equivalent file whose PNG compression bytes differ.
- Do not change `src-tauri/icons/dmg-background.png`, third-party provider
  artwork, screenshots, the established FyAgent `identifier`, deep-link
  schemes, data directories, internal package names, or `LICENSE` as part of a
  future icon-only update. The 2026 clean-break rename is an application
  identity change, not an icon-generation rule.

## 4. Validation & Error Matrix

| Condition                                                                         | Required result                                                                       |
| --------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| Vector is missing, digest-different, executable, or remotely linked               | Stop before rendering                                                                 |
| Rendered source is not 1024px RGBA, has opaque corners, or lacks the Y signal     | Stop before Tauri generation                                                          |
| Stored source differs from a fresh vector render                                  | Reject the change                                                                     |
| A previously tracked generated icon path is missing                               | Reject the inventory                                                                  |
| A generated PNG, ICO, or ICNS container cannot be decoded                         | Reject the output                                                                     |
| About icon differs from generated `32x32.png`                                     | Reject the renderer asset                                                             |
| Tray template has the wrong size, non-black visible RGB, or no partial alpha      | Reject the template                                                                   |
| Application-brand generator output includes third-party artwork, a screenshot, the DMG background, or the V2 128px chrome mark | Remove it from the generator write set; independently reviewed catalog/chrome assets belong to their own task |
| Static/build checks pass but native shell or Dock appearance is unobserved        | Keep native visual acceptance pending                                                 |
| Canonically sorted complete ICNS chunks differ across repeat generation            | Reject even if decoded pixels match; this pipeline requires canonical container-byte stability          |
| A tracked raster asset differs from the reviewed path-and-digest inventory        | Reject until it is decoded, visually reviewed, and the reviewed inventory is updated  |
| A Windows setup has a default/extra group or frames that differ from `icon.ico`   | Reject raw setup before upload; reject sealed setup before attestation or publication |

## 5. Good / Base / Bad Cases

- Good: one approved RGBA source regenerates all Tauri outputs, the About copy
  matches 32px exactly, the three tray templates pass their mask contract, and
  only application-brand files change.
- Base: a future approved vector revision updates the audited digest and
  regenerates `assets/fyagent.png`; consumer paths stay unchanged.
- Bad: only `icon.ico` is replaced, the color bitmap is embedded as a macOS
  template, or a broad image-directory rewrite modifies provider artwork.

## 6. Tests Required

- Decode the source and all generated PNG files; assert dimensions, RGBA mode,
  transparent corners, antialiased alpha, and a substantial blue/cyan Y signal.
- Verify the vector digest, 1024 viewBox, named Y/gate geometry, exact reviewed
  path, and absence of executable or remotely linked SVG content.
- Enumerate ICO sizes and assert the expected Windows frames. Decode every
  PNG-backed ICNS chunk, then compare the sorted complete chunk bytes when
  testing regeneration determinism. A future change to decoded-pixel
  equivalence requires a reviewed parser/test update; it is not the current
  contract.
- Require each raw and final Windows setup to contain exactly the canonical
  `icon.ico` frames, with no default, extra, or unreferenced icon resources.
  [Windows installer](./windows-installer.md#6-tests-required) owns the PE
  resource parser, adversarial layout limits, and final setup verifier details.
- Assert the About file is byte-identical to `32x32.png` and all configured
  paths resolve.
- Assert each tray template size, visible RGB, alpha range, and centered content
  bounds.
- Compare the application-brand generator write set against the pre-change
  checkout and assert exclusion assets are unchanged. Third-party catalog art
  may coexist in a wider feature diff only when it has independent scope,
  provenance, local mapping, and tests. Application-icon adoption is not
  trademark clearance or macOS native visual approval; keep those gates
  explicit.
- Keep `scripts/tasks/supported-platform-raster-assets.json` as an identity seal
  for the raster set that has already passed decoding, metadata, and visual
  review. The digest inventory detects unreviewed byte/path changes; it does not
  replace decoded-pixel validation or make arbitrary image payloads acceptable.
  The path set, regular non-symlink type, Git `100644` mode, and SHA-256 digests
  are exact in both directions. An inventory update must carry fresh decode,
  metadata, and visual-review evidence; changing only a digest is not
  acceptance evidence.
- Run `mise run assets:icons:check`, `mise run format:check`,
  `mise run typecheck`, `mise run build:renderer`, `mise run rust:check`, and a
  desktop bundle build appropriate to the host platform.
- Keep Windows installer/shortcut/taskbar/window and macOS Finder/Dock/app
  switcher/menu-bar inspection as explicit manual acceptance with screenshots.

## 7. Wrong vs Correct

Wrong:

```text
Copy one PNG over icon.png and assume every package surface inherits it.
```

Correct:

```text
Preserve the approved source, run the Tauri generator, derive the About and
macOS template assets, validate every consumer, then perform native visual
acceptance separately.
```
