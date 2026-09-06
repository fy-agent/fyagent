# Primary research and selected reuse

Research date: 2026-09-06. Project owns React18, Vite7, Vitest3, ESLint10,
Playwright1.62 and Motion12. Documentation for a different major is not a reason
to upgrade the application during a layout/configuration task.

| Primary source                                                          | Decision / limit                                                                                                                                                                       |
| ----------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| https://www.w3.org/TR/css-flexbox-1/#min-size-auto                      | Flex automatic content minima and scrollable minima explain why an unbounded intermediate wrapper breaks scrolling. Fix bounded layout ownership; overflow:auto alone is insufficient. |
| https://www.radix-ui.com/primitives/docs/components/scroll-area         | A styled scrollbar does not repair parent size ownership. Native scrolling is sufficient here; no additional scroll engine/package selected.                                           |
| https://www.radix-ui.com/primitives/docs/components/dialog              | Keep Radix modal/keyboard/focus responsibilities, including controlled and nested dialogs. Pass actual trigger ownership through the existing adapter.                                 |
| https://www.radix-ui.com/primitives/docs/guides/animation               | Reuse existing presence integration; no second focus trap or modal portal system.                                                                                                      |
| https://motion.dev/docs/animate                                         | Animation controls distinguish stop/cancel/complete. An internal content observer must not complete every entrance as a generic resize fix.                                            |
| https://developer.mozilla.org/en-US/docs/Web/API/Animation/cancel       | Native cancellation removes the effect and rejects finished; supersession must handle all track promises.                                                                              |
| https://developer.mozilla.org/en-US/docs/Web/API/ResizeObserver         | Content notifications are not equivalent to viewport resize; prevent self-observation/restart loops.                                                                                   |
| https://v7.vite.dev/config/                                             | Vite7 supports --config relative to cwd. Use its existing loader, not a custom forwarding root.                                                                                        |
| https://v7.vite.dev/config/shared-options#css-postcss                   | css.postcss accepts an explicit directory or inline object; preserve autoprefixer when relocating.                                                                                     |
| https://v3.vitest.dev/config/                                           | Vitest3 supports --config and inherited Vite options. Preserve distinct environment projects with one test entry.                                                                      |
| https://v3.vitest.dev/guide/projects                                    | Root config is not automatically a test project; confirm collected suites after moving paths.                                                                                          |
| https://playwright.dev/docs/test-configuration                          | testDir is relative to config. Keep functional versus serial performance separation.                                                                                                   |
| https://playwright.dev/docs/api/class-testconfig#test-config-web-server | webServer cwd defaults to config directory; explicitly preserve repository cwd after movement.                                                                                         |
| https://eslint.org/docs/latest/use/configure/configuration-files        | Keep standard root flat config for automatic CLI/editor lookup rather than introducing compensating overrides.                                                                         |
| https://www.typescriptlang.org/docs/handbook/tsconfig-json.html         | tsconfig defines project ownership; retaining the root boundary avoids ambiguous editor discovery.                                                                                     |

No new dependency is selected by this planning round. Scroll layout composes
the adopted react-resizable-panels adapter; modal motion uses existing
Motion/Radix/WAAPI; source/config inventory uses installed TypeScript/PostCSS
and existing scripts. Reuse still requires boundary/lifecycle verification: a
shared component's presence does not prove each caller's behavior.
