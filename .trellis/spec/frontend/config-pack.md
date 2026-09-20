# Configuration Pack Dialog

## 1. Scope / Trigger

The Models header's ConfigPackButton lazily loads the shared ConfigPackDialog.
Read this before changing its selection, preview, confirmation or port.
Native format and persistence belong to [Portable Configuration Pack](../backend/config-pack.md).

## 2. Signatures

FeaturePorts.configPack provides list, pickFile, previewImport, apply,
previewExport, saveExport and cancel. The native adapter validates unknown
responses through domain/config-pack and never exposes raw exceptions.
The browser port fails explicitly as native-only.

ConfigPackButton/ConfigPackDialog accept `onFillModelForm?: ConfigPackFormFill`.
The typed callback is `(provider: Readonly<PortableProvider>) => void`; it
receives only app/name/endpoint/model/wireApi and populates a local model form.
The Models owner must connect it for production migration usability. Import
readback and stored candidates both offer this action, so closing/reopening
does not strand a saved draft. It preserves Chat/Responses and never persists,
adds credentials, switches a source or activates a connection. Model-form
credential entry and the independent apply preview/confirmation stay required.

## 3. Contracts

Use the existing Dialog/FeatureTabs/Button/Checkbox/Input and semantic tokens.
Every dialog has its actual origin; reopening creates a fresh session.
Only safe candidate fields enter Query under featureKeys.configPackCandidates.
Untrusted pasted text stays local and is cleared on teardown.

An import preview shows app/name/endpoint/model/wire API, every action and
pending API Key. Overwrite displays prior eligible fields. Changes to text or
choices immediately revoke confirmation and cancel the old native preview.
The confirmation payload contains only previewId and digest. No automatic
activation, overwrite, switch or write follows opening a file.

Apply parses exact returned providers and skipped count against final preview.
Only that result permits saved copy. Current connection is unchanged; credentials
and application require a later explicit operation. Export shows actual text
before the native save picker; picker cancellation is not success.

One synchronous lock prevents double clicks. Hidden/unmounted dialogs revoke
action callbacks and cancel their previews; late preview responses are cancelled.
A confirmed native save may finish after navigation and remains discoverable
through the ordinary candidate read. Raw errors never appear in the dialog.

## 4. Validation & Error Matrix

| Condition | UI |
| --- | --- |
| Unsupported or excess field, wrong DTO discriminant | safe error, no optimistic result |
| Protected same-name connection | skip/rename; overwrite absent |
| Text or decision edited after preview | confirm disabled until new preview |
| Pending native operation | no duplicate dispatch |
| Readback differs from final preview | safe readback failure, no saved claim |
| Closed surface receives late preview | cancel it, no new dialog or write |
| Browser runtime or cancelled picker | explicit unavailable/cancelled behavior |

## 5. Good / Base / Bad Cases

Good: show exact fields and pending credentials, then save the reviewed batch.
Base: native-only operation stays unavailable in browser preview.
Bad: reuse an old preview after editing, show native raw errors, or call invoke
from UI.

## 6. Tests Required

Domain/shared fixture, typed-port payload/error/readback tests and
ConfigPackDialog tests cover actual fields, selection, overwrite absence,
choice/text invalidation, double-submit, late response cancellation and picker
cancel. Keep route chunks/ACL/architecture, typecheck, lint and renderer build
checks. Browser fixture behavior is not native picker acceptance.
Test exact callback fields from actual import readback and reopened candidates,
including the Chat discriminant, and assert no native save/apply from that action.

## 7. Wrong vs Correct

Wrong: mutate the preview's fields locally and submit those fields to save.
Correct: edit choices -> native preview -> inspect final fields -> confirm its
unchanged ID/digest -> compare actual readback.
