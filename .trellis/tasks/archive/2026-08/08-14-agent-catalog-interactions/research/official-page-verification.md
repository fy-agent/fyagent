# Official page verification (2026-08-14)

## Scope

Verify official HTTP(S) destinations needed by the Agent directory without
turning vendor pages into installer inputs or implying native integration.

## Evidence

- Qoder CN: `https://qoder.com.cn/qoderwork` is served by the official Qoder CN
  domain, but its current content routes into the wider Qoder product surface.
  `https://qoder.com.cn/download` explicitly lists QoderWork CN downloads. The
  product decision should prefer the destination that matches the button label;
  neither URL authorizes FyAgent-managed installation.
- TRAE Work: `https://www.trae.cn/` is the official TRAE landing page and links
  to the dedicated TraeWork surface at `https://work.trae.cn/`. A button named
  for TRAE Work should use the dedicated product surface when the catalog
  contract is next revised.
- WorkBuddy: `https://www.workbuddy.cn/` is the official product/download page.
- Claude Code CLI: Anthropic's official setup documentation is
  `https://docs.anthropic.com/en/docs/claude-code/getting-started`.
- Claude Desktop: Anthropic's official desktop download page is
  `https://claude.com/download`.

## Conclusions

- Claude requires two explicit renderer actions and therefore cannot be
  represented by the catalog's current single `officialUrl` field without a
  reviewed contract revision or a narrowly owned frontend mapping.
- The V2 Settings port already accepts only validated external HTTP(S) links;
  all actions should continue through that port so browser-preview rejection
  and native-system-browser behavior remain consistent.
- Codex has no official-page action in the requested product behavior. Its
  detail must consume the existing trusted installer facade and must never feed
  a vendor URL into the installer command.

## Uncertainty

- Qoder's preferred label and destination need to remain aligned if the vendor
  continues changing the QoderWork-specific surface. Recheck immediately before
  shipping a catalog URL change.
