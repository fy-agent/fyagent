# Renderer Localization and Locale Schema Contract

## 1. Scope / Trigger

Read this contract before adding/changing a visible translation key, supported
language, language selector, persisted language preference, locale import,
interpolation/pluralization shape, or fallback behavior.

Primary owners:

- `src/i18n/index.ts`
- `src/i18n/locales/{zh,en,ja,zh-TW}.json`
- `tests/config/localeKeyParity.test.ts`
- components/settings that call the exported language-change API

Wording quality and evidence strength are governed by
[User-facing Copy](./user-facing-copy.md). This file owns locale mechanics and
schema parity, not product claims.

## 2. Signatures

The supported language union is exactly:

```ts
export type Language = "zh" | "zh-TW" | "en" | "ja";
```

Current behavior:

```text
default language: zh
persisted preference: localStorage["language"]
browser language: normalized into the closed Language union
fallback language: en
```

The locale resources are imported statically by `src/i18n/index.ts`. Public
language changes go through the exported i18n/change-language owner, which
updates both active language and the reviewed persisted preference. Components
do not write arbitrary language strings directly.

Locale JSON is one recursive object schema whose leaf values are strings.
Arrays, numbers, booleans, null leaves and functions are forbidden.

## 3. Contracts

### Language selection

- Initial language precedence is: valid persisted preference, mapped browser
  preference, then `zh` default. An invalid stored value is ignored; it does
  not become a dynamic locale/import path.
- Browser mapping is explicit: Simplified Chinese maps to `zh`, Traditional
  Chinese variants map to `zh-TW`, Japanese to `ja`, and supported English to
  `en`. Unmapped values use the default/fallback contract rather than substring
  guessing in components.
- Language changes use only the closed union and persist one canonical value.
  Never use display labels such as “中文” or browser raw tags as storage keys.
- `en` remains the runtime translation fallback when a lookup misses. Fallback
  is a resilience layer, not permission to omit keys from another locale.

### Locale key schema

- `zh` is the canonical key-set baseline in the current parity test.
- `en`, `ja`, and `zh-TW` must contain exactly the same flattened key set as
  `zh`: no missing keys and no extra/dead keys.
- Every locale leaf is a string. Nested objects are allowed only as namespaces.
- A key rename/removal updates every locale and every code reference in the
  same change. Do not leave an old key in one language as a compatibility
  alias unless a migration/consumer actually requires it.
- Namespace/key names are stable semantic identifiers, not full source-language
  sentences. Rewording translation text should not force key churn.

### Component use

- All user-visible static labels, empty/error states, validation, buttons,
  menus, confirmations and status explanations use translation keys.
- Backend reason codes remain closed data and map to locale keys in the feature
  adapter/presentation layer. Do not render raw backend English or use a reason
  code itself as polished copy.
- Interpolation variables are named, bounded presentation values. Never pass
  raw HTML, secret material, path, command output or untrusted vendor text to a
  translation that renders as markup.
- Do not build keys from arbitrary server/user strings. Dynamic selection uses
  a closed mapping with a deterministic fallback.
- Keep tests/selectors semantic. Do not rely on one language's visible text
  when role/label/test identity can remain stable across locales.

### Adding a language

A new language is one coordinated contract change:

1. extend the `Language` union and browser mapping;
2. add a static locale resource with exact key/leaf parity;
3. add the selector label and persisted-value handling;
4. extend parity and language-selection tests;
5. review layout/overflow/font/focus for the new copy;
6. update this Spec and any packaging/accessibility evidence.

Adding only a JSON file or selector option is incomplete.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Stored language is outside the closed union | Ignore it and use browser/default mapping; do not dynamic-import it. |
| Browser tag is unsupported | Use reviewed default/fallback behavior. |
| Locale misses or adds a key relative to `zh` | `localeKeyParity` fails. |
| Locale leaf is array/number/boolean/null/object-without-leaves | Parity/schema test fails. |
| Component renders a new raw literal visible to users | Review/test failure; add semantic key to every locale. |
| Backend reason has no translation mapping | Render reviewed generic localized fallback and add the missing closed mapping; never raw stack text. |
| Dynamic key is built from untrusted text | Reject in review; map closed values explicitly. |
| Translation interpolates secret/path/raw HTML | Security/content regression. |
| Language changes but persistence fails | Keep active language usable and do not claim durable preference; handle storage failure safely. |

## 5. Good / Base / Bad Cases

- **Good:** add `agents.auth.timedOut` to all four locale files, map the closed
  reason code, and assert parity.
- **Good:** normalize `zh-Hant-TW` through the central browser mapping to
  `zh-TW`.
- **Base:** a runtime lookup unexpectedly misses; English fallback keeps the UI
  usable while parity tests identify the defect before release.
- **Bad:** add a key only to English, store `navigator.language` verbatim,
  dynamic-import `locales/${value}.json`, build `t(serverReason)`, or pass raw
  error/path/HTML as interpolation.

## 6. Tests Required

```bash
mise run typecheck:v2
mise run test:unit -- tests/config/localeKeyParity.test.ts
mise run test:v2
mise run test:v2:browser
```

Required assertions:

- `tests/config/localeKeyParity.test.ts` proves exact key parity and string-only
  leaves for all four current locales;
- initial language precedence for valid/invalid persisted preference and each
  reviewed browser-language mapping;
- changing language updates the active i18n instance and canonical persisted
  value, including safe storage-failure behavior;
- every newly introduced feature state/reason has a closed localized mapping
  and no raw backend fallback;
- no dynamic locale import/key from untrusted values;
- representative layouts remain usable in all languages with keyboard/focus
  and non-truncated critical actions;
- copy tests assert meaning/role where possible rather than one locale's exact
  prose.

## 7. Wrong vs Correct

Wrong:

```ts
const locale = localStorage.getItem("language") ?? navigator.language;
const messages = await import(`./locales/${locale}.json`);

toast.error(t(error.reason));
```

Correct:

```ts
const language = resolveSupportedLanguage({
  persisted: localStorage.getItem("language"),
  browser: navigator.language,
});

await changeLanguage(language);
toast.error(t(reasonToTranslationKey[parseClosedReason(error.reason)]));
```
