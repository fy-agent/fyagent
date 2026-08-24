# Implementation plan

1. Diff #108 and #113 against current main to isolate approved copy/presentation results.
2. Apply Grok locale and focused test changes surgically.
3. Define pure Apply view-model/component contracts against integration-owned types.
4. Add state/accessibility/repeat-click/native-only-negative tests without fake product data.
5. Report required page/port wiring to integration; do not modify shared registration.

## Progress log

- 2026-08-24: claimed by `gpt-5.6-sol-medium`; baseline and ownership frozen by root.
- 2026-08-24: real Change Plan Apply UI and four-locale Grok copy were integrated; restored-baseline P1 presentation was fixed during final review.

## Deliverables

- [Apply workspace](../../../src/v2/pages/models/apply/ApplyWorkspace.tsx)
- [Apply view model](../../../src/v2/pages/models/apply/view-model.ts)
- [Grok provider form](../../../src/components/providers/forms/GrokBuildProviderForm.tsx)

## Acceptance evidence

- V2 lint/typecheck and `314/314` unit tests passed, including `21/21` Apply view-model/component cases.
- Fresh browser `116/116`, renderer build and four-locale i18n tests passed.
