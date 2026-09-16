# Prompt Preset Catalogue

## 1. Scope / Trigger

Read before changing the static FDE catalogue, its browser, or copying a preset
into an application library. Native CRUD, live-file authority and seven-app
identity remain in [Prompts and Memory](./prompts-memory.md).

## 2. Signatures and owners

`src/pages/prompts/presets.ts` owns `FDE_PROMPT_PRESETS`, the closed six-value
`PromptPresetCategory`, `PROMPT_PRESET_CATEGORIES` and:

```ts
searchPromptPresets(
  query: string,
  category: PromptPresetCategory | "all",
): readonly PromptPreset[];
```

`PromptPreset` has stable `id`, `name`, `category`, `description`, `tags`,
`inputs`, `approach`, `deliverables`, `evaluation`, `example`, and composed
`content`. `PresetBrowser.tsx` owns search/filter/preview selection only;
`Page.tsx` owns application, editor, discard intent and native mutations.
There is no new Port, native command, preference or persistence schema.

## 3. Contracts

- The catalogue is original static content, not an application library, a
  vendor-endorsed template or a measured model benchmark. Each scenario has
  specific inputs, implementation choices, deliverables, acceptance cases and
  an example. The common contract covers evidence, untrusted external text,
  tool approval and data minimization without demanding a fixed report format.
- Search matches every whitespace-separated, case-insensitive term in the
  scenario metadata/body, intersected with the chosen category. Do not search
  credentials, native user records or only the shared boilerplate.
- Use shared `FeatureTabs` / `FeatureTabPanel` with `layout="workspace"`,
  `FeatureSearch`, `FeatureList`, `SplitPanes` and `Button`. The preset panel
  mounts on selection and unmounts on exit. The library/editor stays mounted
  while browsing, so its draft is not discarded by a view-only tab change.
- Preview/search is available without native access. `使用此预设` requires
  an existing successful library query and no active mutation. Browsing or
  selecting a catalogue row never invokes a native write.
- Applying a preset creates a new draft containing copies of name,
  description and content. It does not use the catalogue ID as a library ID,
  mutate an existing row, save, enable, install MCPs or interpolate secrets.
- Replacing a dirty draft uses the existing `preset` discard intent and
  shared ConfirmDialog. Cancel preserves the draft; confirm copies the preset
  and returns to the library. App/route changes retain their existing guards.
  Capture the Use button's origin with its persistent preset-tab return anchor:
  the button unmounts on confirmation, and the shared dialog redirects an
  inactive tab anchor to its currently active sibling instead of stealing focus.
- Save uses `prompt-` plus 16 cryptographically random bytes encoded as 32 hex
  characters for new renderer-created records (`crypto.getRandomValues`),
  preserves existing IDs on edit and sets new records `enabled: false`.
  Do not require the secure-context-only `crypto.randomUUID` API or use a
  timestamp as uniqueness. Repeated copies cannot replace an enabled record. `upsert` and authoritative
  rereads are unchanged; enable remains a separate explicit operation.

## 4. Validation & Error Matrix

| Condition                                    | Required behavior                                                         |
| -------------------------------------------- | ------------------------------------------------------------------------- |
| No native library data / initial read failed | Browsing works, use is disabled; no synthetic native success.             |
| Search/category matches nothing              | Empty result with clear-filter action; no stale preview.                  |
| Browse while editor is dirty                 | Preserve the editor and its route blocker.                                |
| Apply while dirty                            | Confirm before replacing; cancel keeps user content.                      |
| Use or save while busy                       | Existing write lock prevents duplicate native operations.                 |
| Save fails                                   | Retain editable draft and error; no enable or success claim.              |
| Save succeeds, reread fails                  | Retain the existing uncertainty warning, not an invented effective state. |
| Copy same preset again                       | Independent disabled record with a new opaque ID.                         |

## 5. Good / Base / Bad Cases

Good: preview an environmental-report preset, edit project details, save a new
disabled Claude record, then separately choose whether to enable it. Base:
browser preview can read all templates but cannot save to a desktop library.
Bad: auto-seed every application, overwrite a preset-named existing record, or
invoke a suggested integration merely because its name appears in prompt text.

## 6. Tests Required

`tests/renderer/pages/prompts/presets.test.ts` checks unique IDs, complete
scenario fields, category/search intersections and shared safety boundaries.
`Page.test.tsx` covers zero-write preview, dirty cancellation/confirmation,
repeated copies, all seven libraries, disabled saves and native-only browsing.
Keep existing failed-save/readback, route guards and live-file tests.
`tests/browser/fde-presets.spec.ts` checks bounded readable preview, filters,
empty results and native-only behavior in Chromium/WebKit. Static structure
and browser fixtures do not establish model output quality or native writes.

## 7. Wrong vs Correct

Wrong: `upsert(app, { ...preset, id: preset.id, enabled: true })` on selection.
Correct: create an unsaved draft, explicitly save with a fresh 128-bit opaque ID and
`enabled: false`, then let the existing enable/readback workflow act separately.
