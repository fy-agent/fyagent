# Thinking Guides

Thinking guides are short implementation-preparation checklists. They help find
the owning contract and the right design questions; they do not define product
DTOs, paths, error codes, current versions, or feature behavior.

| Guide | Use before |
| --- | --- |
| [Code Reuse Thinking Guide](./code-reuse-thinking-guide.md) | Creating a component, helper, service, dependency, adapter, or repeated implementation. |
| [Cross-Layer Thinking Guide](./cross-layer-thinking-guide.md) | Changing data that crosses storage, Rust, Tauri IPC/events, renderer ports, state, or UI. |

When a decision changes a signature, serialized shape, authority boundary,
failure matrix, platform behavior, or required tests, update the owning
[backend](../backend/index.md) or [frontend](../frontend/index.md) code spec.
One-time research and execution evidence belongs in the active Trellis task.
