# CC Switch compatibility updates — 2026-09-20

This update adapts selected fixes from
[CC Switch](https://github.com/farion1231/cc-switch) to FyAgent's existing
configuration and proxy paths. The source commits below are the immutable
reference for each change.

FyAgent baseline: `2c09c4be2c5f50b4060fd6f7a13a7be31c364c54`.
The complete upstream ancestry baseline remains
[CC Switch v3.19.2](./cc-switch-v3.19.2.md).

| Behavior                                                                                                | Upstream source                                                                                                                       | FyAgent owner                                                                                                |
| ------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| Preserve each application's retry and timeout settings during shutdown and listener-port updates        | [`11317c628f42920bac82aab0e1ce3e60876d6860`](https://github.com/farion1231/cc-switch/commit/11317c628f42920bac82aab0e1ce3e60876d6860) | `services/proxy.rs`                                                                                          |
| Preserve child-provider metadata, creation time, and ordering during universal-provider synchronization | [`e0c2fd2b1096c46a307b14aa5b93c1836be86ca0`](https://github.com/farion1231/cc-switch/commit/e0c2fd2b1096c46a307b14aa5b93c1836be86ca0) | `services/provider/universal.rs`                                                                             |
| Keep adjacent Codex commentary and tool calls in one assistant turn                                     | [`5e0f3442eadf0b0a6d90a6db5d82931f732688e4`](https://github.com/farion1231/cc-switch/commit/5e0f3442eadf0b0a6d90a6db5d82931f732688e4) | `proxy/providers/transform_codex_chat.rs`                                                                    |
| Skip empty reasoning placeholders while preserving real streamed text                                   | [`b78192e8fec3e062948237526d9c03ad08ea7831`](https://github.com/farion1231/cc-switch/commit/b78192e8fec3e062948237526d9c03ad08ea7831) | `proxy/providers/streaming.rs`                                                                               |
| Support xAI native Responses tool schemas, agent messages, model mapping, and integer tool arguments    | [`ef97ef95e717b1b5c47357c5a28ed78387138417`](https://github.com/farion1231/cc-switch/commit/ef97ef95e717b1b5c47357c5a28ed78387138417) | `proxy/providers/transform_codex_responses_xai_sanitize.rs` and the existing Codex request/response handlers |

Paths in the owner column are relative to `src-tauri/src/`.

The xAI compatibility functions and regression cases are reused at the existing
provider boundary. FyAgent selects the active endpoint by its parsed host and
effective protocol for both managed xAI accounts and native API-key providers.
Response handling restores tool namespaces and normalizes complete integer
arguments, including requests without namespace tools; argument deltas retain
their original contents. Configuration fixes use the current global-listener
and per-application settings owners.

Kimi's two official endpoints already use native Responses through FyAgent's
production Quick Setup path. Regression cases cover the renderer request,
native provider construction, explicit model selection, and saved configuration.

Upstream-derived code remains under the MIT license. Attribution and the full
license are retained in [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md)
and [LICENSES/MIT-CC-SWITCH.txt](../../LICENSES/MIT-CC-SWITCH.txt).
