# Renderer and Build Input Security

## Scope and ownership

The real `src/index.html` and Vite multi-chunk build are the only application
entry. There is no offline generator, single-file test distribution, `file:`
redirect or user-operated HTML inspection page. Development and automated
browser fixtures use loopback HTTP. This does not remove native deep-link
parsing/security, which belongs to
[Deep-Link Import Security](../backend/deeplink-import-security.md).

## Contracts

Untrusted strings render as text, never HTML. URLs are parsed structurally and
validated at the appropriate native port; a domain substring is not an origin
check. Configuration objects are decoded from `unknown` before field access.
Own-key structural merge/removal must not mutate prototypes or inherited values.
Pure deep-link preview/redaction code lives in `domain`; decoding never grants
permission to import or run scripts. Native parsing, payload validation and
provider activation checks remain; the current renderer has no import event
consumer or confirmation UI. The native approval flag is not itself proof of
human consent. Any future UI must meet the linked native contract's explicit
confirmation and stale-result requirements before introducing an import Port.

Production asset paths remain confined to the Vite distribution. The route
chunk verifier walks the actual static entry closure, requires exactly seven
literal product route chunks and rejects a route leaked into eager startup.
Budget checks still apply after directory moves; removing a redundant entry
wrapper is not permission to increase JavaScript/CSS thresholds.

Runtime graphs use dependency-cruiser with TypeScript support verified, current
entry coverage, cycle/unresolved-edge checks and layer direction. Type-only
contracts are checked by TypeScript and do not become runtime dependencies.
Keep negative fixtures: a scanner that silently loses its parser must fail.
Source migrations update all effective import, test, build and SPEC references.

| Failure                                           | Required handling                                                                                        |
| ------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Invalid/non-object configuration                  | Explicit safe rejection; do not spread arbitrary input.                                                  |
| Dangerous own key or inherited setter             | Treat as data without prototype mutation or implicit execution.                                          |
| Untrusted URL or external redirect                | Enforce the owning port's exact protocol/host/admission contract.                                        |
| Deep link carries code/credentials                | Preserve redaction/native validation; never invent a current import UI or infer consent from URL fields. |
| Generator/standalone HTML is reintroduced         | Repository entry/retirement regression fails.                                                            |
| Dependency cycle, unresolved import or empty scan | Fail the architecture gate; do not add broad exclusions.                                                 |

Good: keep native import validation and portable redacted preview separately
tested; add explicit confirmation before any future renderer import Port.
Bad: ask users to run an HTML test page or
copy secret form values into an animation/debug snapshot.

## Tests required

Run the unified type/lint/unit checks, native deep-link tests and production
boot/route chunks. Domain deepClone, providerConfigStructural, Codex TOML and
deepLinkConfigPreview tests preserve parsing, encoding and secret handling.
The current renderer architecture and independent dependency graph tests must
both pass; no deleted-UI test may be cited as current behavior evidence.
