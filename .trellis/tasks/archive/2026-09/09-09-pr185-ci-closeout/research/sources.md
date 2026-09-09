# Primary evidence

- Original PR: https://github.com/fy-agent/fyagent/pull/185
- Failed run: https://github.com/fy-agent/fyagent/actions/runs/34229637275
- W3C text contrast: https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html
- Playwright screenshot behavior: https://playwright.dev/docs/api/class-page#page-screenshot
- Playwright configuration composition: https://playwright.dev/docs/test-configuration
- GitHub merge policy/head matching: https://cli.github.com/manual/gh_pr_merge
- React asynchronous effect cleanup: https://react.dev/reference/react/useEffect
- Testing Library unmount and cleanup: https://testing-library.com/docs/react-testing-library/api/

The authoritative local code contracts are the renderer surface/appearance
specs and `tests/browser/support/visual.ts`. The local Trellis scripts govern
task archive validation; generic upstream documentation does not replace
their actual behavior. The work is based on original PR SHA
`2956da243d1e79f401d946852dadba8f30d28043` and main baseline
`2f264d2f89326601a33f610c72a9f0143306d066`.
