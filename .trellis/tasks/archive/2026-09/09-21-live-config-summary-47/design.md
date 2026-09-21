# Design

Extend get_provider_summary in place with a closed live object. Native always
returns it; the TypeScript property stays optional for existing mocks/old hosts,
while the desktop adapter supplies an unreadable fallback for missing or invalid
live payloads. States: configured/not_configured/missing/unreadable; target is the
requested Provider app. exists is true/false/null; only configured has a public
partial connection of baseUrl/modelId/protocol, with null for unspecified fields.

Use ProviderService::quick_setup_write_targets for existence/disclosure and
ProviderService::read_live_settings for reads. No direct external endpoint call,
credential resolution, file writer or DB-current projection. Keep projection in
an isolated command child module, reusing existing credential leaf/header/TOML
collection and URL collision policy. Bad live state and failed target metadata
become a neutral unavailable observation, leaving DB summaries usable.

Parent owns Page/FirstUseGuide/AgentModelsSection. #35 owns provider service and
credential modules; this package does not modify them. Session-scoped Trellis
pointer uses live-config-summary-47 and never changes root's active pointer.
