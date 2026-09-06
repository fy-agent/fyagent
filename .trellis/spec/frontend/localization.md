# Renderer Language and Localized Copy

## Scope and current contract

The current product renderer uses the existing Simplified Chinese interface,
`src/index.html` declares `zh-CN`, and feature presentation maps closed backend
states to reviewed Chinese copy. It has no language picker or locale runtime.
The retired application and four unused translation bundles are not a supported
alternative interface; do not restore their providers through shared imports.

The Chinese/English/Japanese user manuals are maintained documents, not proof
that the application implements three languages. Legacy locale candidate IDs
remain candidate-only in the acceptance matrix and must not be reported as
covered by a deleted locale-key test. Existing saved metadata is not rewritten
just because the current UI does not consume an old preference.

## Ownership and failure behavior

Labels, empty/error states and confirmations belong to the relevant product
presentation module. Keep backend reason codes closed and map unknown/failure
conditions to safe, useful copy; do not present raw vendor errors, stack traces,
secrets or HTML as translation text. See [User-Facing Copy](./user-facing-copy.md).

Language selection is a separate product change: it must establish one closed
language set, actual static resources, selection/persistence/fallback owner,
complete key/interpolation parity and keyboard/layout tests before adding a
picker. Do not add a translation framework just to preserve retired tests, and
do not claim unsupported-language fallback from a JSON file that is not loaded.

Good: update the current Chinese status mapping and its semantic-role test.
Bad: put an English-only fallback in a business error, dynamically import an
untrusted locale value, or count old translated screenshots as current support.

## Required verification

The current copy AST check, accessible names, real text contrast, long-content
container tests and feature behavior tests run in `mise run test:unit` and
`mise run test:browser`. Newly supported languages require exact resource and
selection tests as part of the same single runner, not a generation fork.
