# Desktop Visual Hierarchy and Dialogs

## 1. Scope / Trigger

Read before changing V2 typography, shared confirmation/dialog layout, sidebar
selection shape, or a page-specific visual override. The existing navigation,
catalog/master-detail structure and workflows are not redesigned by a token change.

## 2. Signatures and Owners

`app/styles/tokens.css` owns the role scale: page title 22px, section/dialog
title 16px, body 14px, control 13px, caption 12px; body leading 1.6, heading
leading 1.35, medium weight 500 and semibold 600. Pages use the role tokens
instead of browser-default oversized headings. This is the reviewed desktop
scale, not an instruction to suppress browser zoom or user text scaling.

```ts
Dialog({
  open, onOpenChange, title, description?, children?, actions?,
  size?: "standard" | "comfortable" | "wide",
  initialFocusRef?: RefObject<HTMLElement>,
})
ConfirmDialog({ open, title, description, pending?, onConfirm, onCancel })
```

Both components remain in `shared/ui/primitives.tsx`; Radix owns modality,
focus trapping and announcements. Widths are capped at 480/720/900px and the
viewport minus 32px. Login uses comfortable; rich editors use wide.

## 3. Contracts

- Dialog title and description have explicit role classes. Keep one scrolling
  content region (header + optional body) and a nonshrinking action footer.
  Long descriptions must not push actions outside the viewport.
- A confirmation with no additional content has no empty body or generic
  sentence repeating that confirmation is required. Its description explains
  the actual action and consequence.
- Dialog consumers use `initialFocusRef` for cancel-first focus, not React
  `autoFocus` inside a newly mounted portal. React can focus that descendant
  before Radix's opening event and bypass trigger capture. The wrapper records
  focus on opening, restores only a connected/non-hidden target on closing,
  and does not install a document listener for each closed dialog.
- A dialog without description explicitly omits `aria-describedby`; when a
  description exists, keep Radix's own paired ID. Do not override only one
  side of that pairing or silence missing-description warnings.
- A hidden persistent surface never presents its dialog portal or restores
  focus into a hidden/inert tree. Pending confirmations cannot close or submit.
- Sidebar hosts use the shared pill radius. The measured SelectionLens keeps
  copying host geometry; do not replace the shared measurement or add an
  independent selected background. Capture visual evidence after it settles.
- Primary forward actions are visually distinct from secondary/cancel actions.
  Color alone never replaces labels, disabled state, or current-selection ARIA.

## 4. Validation & Error Matrix

| Condition                                 | Required behavior                                                           |
| ----------------------------------------- | --------------------------------------------------------------------------- |
| Dialog has no body                        | Render header and actions without empty padded content.                     |
| Content exceeds viewport                  | Content scrolls; footer remains reachable.                                  |
| Cancel-first interaction                  | Focus requested ref after recording the trigger; preserve Tab trap and Esc. |
| Description absent                        | No dangling ARIA description or Radix warning.                              |
| Target removed/hidden while dialog closes | Do not focus stale or invisible content.                                    |
| Sidebar selected item moves               | One overlay follows final host box/radius, including reduced motion.        |
| Page lacks explicit title sizing          | Role fallback prevents default 2em headings.                                |

## 5. Good / Base / Bad Cases

Good: the account page uses the same 22px heading as Models and Memory; a
short confirmation does not allocate an editor-sized empty body. Base: the
MCP editor legitimately uses wide while account selection uses comfortable.
Bad: a page overrides every shared control, adds a second focus trap, or uses
`autoFocus` to race the opening event in a controlled dialog.

## 6. Tests Required

- `tests/v2/shared/Dialog.test.tsx`: description pairing, no filler, cancel
  focus, trigger restoration, pending lock and hidden portal behavior.
- `tests/v2-browser/experience.spec.ts`: actual computed title/button scale,
  capped widths, visible footer, selected host/lens geometry and screenshots.
- Existing multi-viewport shell, account, model/editor and keyboard tests;
  `mise run typecheck:v2`, `lint:v2`, `test:v2`, `test:v2:browser`.
- Browser fixture images do not prove native window chrome, platform fonts on
  every host, all contrast pairs, or subjective final-product acceptance.

## 7. Wrong vs Correct

Wrong: a closed dialog keeps a global focus listener, a confirmation repeats
"please confirm" in a padded body, or a screenshot freezes a moving lens and
is treated as its final geometry.

Correct: delegate focus to Radix with an explicit initial ref, render only
decision-relevant content, and check settled geometry before visual review.
