# Daily Memory Selection and Missing File Contract

## 1. Scope / Trigger

Read before changing daily-memory selection, missing-file feedback, rereads
or draft preservation. The owner is `src/pages/memory/Page.tsx`; the existing
port and dirty/write rules remain in [Prompts and Memory](./prompts-memory.md).

## 2. Signatures

`DailyView` waits for the initial daily list and then mounts `DailyWorkspace`.
The workspace stores the initially opened filename as `selectedFile`.
`DailyEditor` receives the selected filename and authoritative content; `null`
means a successful missing-file read. Explicit Save still uses the existing
`MemoryPort.writeDailyFile(filename, content)` and authoritative reread.

## 3. Contracts

- Once the initial list has settled, its initial opened filename becomes the
  explicit selection. Later list changes do not silently select another file.
- A removed selected file remains selected until an explicit user transition
  or confirmed delete. External deletion is not a navigation instruction.
- Missing copy identifies the selected `YYYY-MM-DD`. Only a filename matching
  the current local date may say `今天`.
- A successful reread returning `null` changes the missing indicator without
  resetting the editor, current draft or dirty baseline. A read failure is
  not evidence that a file is absent.
- Initial selection, missing reads and external deletion do not create files.
  Explicit Save writes only the selected filename and current draft, retaining
  authoritative reread and distinct post-write warning behavior.

## 4. Outcome Matrix

| Authoritative result             | View and write outcome                           |
| -------------------------------- | ------------------------------------------------ |
| Selected file exists             | Existing editor without missing notice           |
| Today selected and missing       | Today notice; no automatic write                 |
| Past/future selected and missing | Selected date notice; no automatic write         |
| External removal while dirty     | Same selection/editor/draft; missing notice      |
| Explicit Save after removal      | Write selected file, reread, clear missing state |

## 5. Good / Bad

Good: initialize selection after the first list settles and derive missing
status from the latest successful file read, separately from editable state.

Bad: keep selection null and repeatedly fall back to `list[0]`, or cache file
existence at editor mount. Both can misrepresent a later external deletion.

## 6. Tests and Evidence

`tests/renderer/pages/memory/Page.test.tsx` uses the production page, existing
query hooks and stateful injected ports. Cover today/past/future, existing and
missing files, external removal/list refresh, dirty draft and selected Save.
Inspect that missing/refresh paths have no write/delete effect. This is
renderer fixture evidence; native filesystem and screenshot acceptance remain
separate.

## 7. Prevention

Do not turn a missing record into implicit creation or an automatic switch to
today. Preserve user selection and the existing explicit transition/dirty guard.
