# Official primitive and performance evidence

## Radix Tabs

Official documentation:

https://www.radix-ui.com/primitives/docs/components/tabs

Radix Tabs supports controlled/uncontrolled state, automatic/manual activation, keyboard navigation and accessible trigger/content relationships. FyAgent already depends on the package, so wrapping it inside `FeatureTabs` avoids maintaining a second interaction implementation and avoids a new design system.

## React render purity

Official rule:

https://react.dev/reference/rules/components-and-hooks-must-be-pure

React requires render to remain idempotent and free of side effects. Calling state setters while computing the rendered tree makes behavior dependent on render retries and should be replaced by derived state or an effect/domain controller where synchronization is intentional.

## React lazy loading

Official API:

https://react.dev/reference/react/lazy

`lazy` defers loading a component module until it is first rendered and caches the loaded module. This directly fits the six first-level route modules and development-only UI Lab.

## TanStack Query inactive behavior

Official guide:

https://tanstack.com/query/latest/docs/framework/react/guides/disabling-queries

Query `enabled` controls automatic fetch/refetch behavior. FyAgent should prefer declarative route/activity enablement and normal cache ownership over keeping hidden pages mounted merely to preserve data.

## Playwright accessibility testing

Official guide:

https://playwright.dev/docs/accessibility-testing

Automated axe checks can catch a subset of accessibility defects but do not replace manual keyboard, focus, contrast and native WebView evaluation. Adding an axe dependency requires the repository's dependency/license/maintenance review; current role/keyboard tests remain mandatory regardless.

## Decision

- Reuse installed Radix Tabs.
- Use React lazy routes and pure render patterns.
- Use TanStack Query's existing activity/cache controls.
- Treat automated accessibility scanning as optional supplemental evidence, not a substitute or an automatic new dependency.
