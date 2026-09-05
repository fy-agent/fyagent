# Renderer and Build Input Security

## 1. Scope / Trigger

Read before changing provider identity hints, common configuration merging,
the offline deep-link inspector, standalone preview HTML processing, or runtime
dependency boundaries. Native authorization remains behind typed Ports.

## 2. Signatures and Owners

```ts
// src/utils/providerCapabilities.ts (leftover renderer owner)
isCopilotEndpoint(value: string): boolean;

// scripts/preview-html.mjs (build-only parse5 adapter)
htmlElements(source, names);
htmlAttribute(element, name);
scriptContent(source, element);
```

`src/utils/providerConfigStructural.ts` owns sanitize/merge/remove/subset
behavior. `scripts/build-v2-preview.mjs` owns distribution asset confinement
and inlining. `deplink.html` is an offline input inspector, not a native
capability or trusted source of executable configuration.

## 3. Contracts

### Identity is structured, not a substring

The Copilot URL hint uses `URL`, requires HTTPS, rejects username/password,
and recognizes only `githubcopilot.com` or a dot-delimited subdomain. A string
in a path, query, userinfo or suffix cannot establish identity. This is only
UI classification; native provider policy still decides credential routing.

### Structured configuration owns only own data

Sanitize and merge exclude `__proto__`, `constructor` and `prototype`
recursively. Shared writes create enumerable writable configurable data
properties; inherited setters are not executed. Merge never mutates inherited
target objects and remove ignores inherited keys. Preserve replacement-array
and recursive plain-object semantics; do not substitute another merge library
without proving compatibility for JSON and TOML consumers.

### Text inspection never becomes HTML execution

Deep-link parameters, decoded configuration, error copy and decoded scripts
are displayed using native `textContent`/text nodes and `replaceChildren`.
Do not compose dynamic `innerHTML`, event attributes or script nodes from those
values. Decode/parse is not a sanitization step. Ordinary credential previews
use a complete mask, including short values; original JSON remains an explicit
local inspector disclosure with a credential warning, not a telemetry/export
or ordinary list display. Preserve this distinction in future UI changes.

### HTML syntax belongs to parse5

The preview builder uses parse5 source locations for actual HTML-namespace
elements. Comments, raw-text lookalikes and inert template contents do not
become module/stylesheet entries. Attribute values use HTML decoding; replace
the actual source attribute range rather than the first matching string.
An executable script without its closing tag or a document without an explicit
head insertion point fails closed. Implicit parser-created nodes are not
invented source ranges.

Only an inline classic script explicitly marked `data-fyagent-file-redirect`
is removed as the owned file redirect. Matching text is not ownership; an
unmarked script mentioning the preview URL stays intact. This parser is not
an HTML sanitizer and does not authorize untrusted build source. Existing
path/asset/module constraints remain in the builder.

### Executable architecture evidence

`.dependency-cruiser.cjs` enforces runtime cycles, unresolved imports and
generation/layer direction across `src` and `scripts`. The tool must load the
project TypeScript compiler; missing-compiler partial scans are invalid.
`tsPreCompilationDeps: false` means runtime dependency evidence, not a proof
that every type-only reference is acyclic. Import-specific AST/ACL tests remain
necessary. Dependency-cruiser and parse5 are development dependencies, not a
new renderer runtime/framework.

## 4. Validation & Error Matrix

| Condition                                                             | Required result                                                       |
| --------------------------------------------------------------------- | --------------------------------------------------------------------- |
| Copilot text appears in unrelated hostname/path/query                 | UI hint false; no inferred authentication.                            |
| Merge encounters inherited object/setter or forbidden key             | Do not mutate/execute it; create own safe data or skip forbidden key. |
| Inspector input contains tags, JSON strings or executable script text | Display literal text; no new executable DOM node.                     |
| HTML contains comment/template/raw-text fake entry tags               | Ignore those as asset entries.                                        |
| Marked redirect is external/module rather than inline classic         | Reject preview generation.                                            |
| Runtime graph has a cycle/unresolved/upward import                    | Gate fails; fix owner/import, not a broad exclusion.                  |

## 5. Good / Base / Bad Cases

- Good: parse a hostname once, let native policy authorize the operation, and
  render all externally supplied diagnostic values as text.
- Base: raw JSON can be inspected locally under an explicit disclosure; this
  does not make it safe for ordinary logs or analytics.
- Bad: regex-parse arbitrary HTML, delete scripts by substring, merge into
  inherited properties, or treat a truncated dependency scan as clean.

## 6. Tests Required

Run `mise run test:unit` plus V2/build/browser gates as applicable. Owners:
`tests/utils/copilotEndpoint.test.ts`, `providerConfigStructural.test.ts`,
`tests/deeplinkPlayground.test.ts`, `tests/previewHtml.test.ts`,
`tests/architecture/dependencyGraph.test.ts` and
`tests/v2/scripts/build-v2-preview.test.ts`.
Assert spoofed hosts, inherited setters/objects, nested forbidden keys,
decoded-script text, full short-key masks, malformed tags, range fidelity,
owned-marker removal and graph negative fixtures. A tool upgrade may fix test
asset inlining explicitly; it must not weaken local asset-source assertions.

## 7. Wrong vs Correct

Wrong: `url.includes("githubcopilot.com")`; `target.innerHTML = userValue`.

Correct: `isCopilotEndpoint(url)` for the UI hint; `element.textContent = value`
for inspection; native typed policy for actual credential authority.
