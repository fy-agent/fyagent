# Session Keyboard Context Contract

## 1. Scope / Trigger

Read before changing Sessions keyboard shortcuts or their regression tests.
The production owner is `src/pages/sessions/Page.tsx`; route lifetime belongs
to [Navigation](./navigation.md), and focus/modal behavior belongs to
[Dialog Lifecycle](./dialog-lifecycle.md).

## 2. Signatures

`SessionsPage` uses the existing `usePersistentVisibility(): boolean`, a page
root ref, the existing import dialog state and `handleOpenExport`. It does not
introduce a global shortcut registry or change session import/export ports.

## 3. Contracts

- A mounted, hidden keep-alive page must remove its window shortcut listener.
  Returning to it cannot open a delayed import/export dialog.
- Visible Sessions handles Ctrl/Cmd+I/E only when it owns the key context.
  Respect already prevented events, composition, repeated or modified keys.
- Check both the event target and actual focus. Text inputs, textarea, select,
  editable content, focus outside the page, and an active dialog/alertdialog
  retain their context. Portaled dialogs are included.
- An open page-owned dialog suppresses these shortcuts. Hidden/closed dialogs
  do not permanently suppress the visible page's shortcut.
- Preserve selection, search and unsaved import paths across route visibility
  changes; unregistering listeners does not unmount or reset the page.

## 4. Outcome Matrix

| Context                                | Outcome                                         |
| -------------------------------------- | ----------------------------------------------- |
| Hidden Sessions                        | No import/export action and no `preventDefault` |
| Visible page with ordinary page focus  | Existing import/export preview                  |
| Editable input or dialog owns focus    | No shortcut interception                        |
| Return after keypress on another route | Existing state; no delayed dialog               |

## 5. Good / Bad

Good: attach listeners only while visible and no page dialog is open, then
verify the actual event/focus context before consuming the event.

Bad: attach once on mount, or use route mounting as proof that the page is
visible. Hiding a persistent page preserves its component and effects.

## 6. Tests and Evidence

`tests/renderer/app/session-shortcuts.test.tsx` exercises the production
Sessions page, shared dialogs and `PersistentPrimaryOutlet` with controlled
OS/data ports. Cover hidden/visible transitions, Ctrl/Cmd, editable contexts,
active dialogs, prevented events and retained selection/draft. Related route
and migration tests preserve prior contracts. Renderer fixtures do not prove
native window dispatch or actual import/export I/O.

## 7. Prevention

Do not widen a localized shortcut fix into a new shell event system. New
feature shortcuts must adopt their existing visibility/context owner before
claiming a key or calling `preventDefault`.
