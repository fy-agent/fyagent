# Design

Use Agent lifecycle policy as the only surface/action authority. Route Claude's CLI installation through Tooling and its existing job/observation lifecycle. Reuse the reviewed npm registry policy and subprocess/ordinary-user boundary rather than cloning a shell installer. The official root package and host-specific package must match an exact-version manifest compiled into the application; mirrors are transport, not trust anchors.

Reuse `auth_actions.rs`'s fixed `claude auth login`, `claude auth status` and bounded parser. Vendor-owned login may write Claude's own credentials/keychain; do not manufacture a JSON credential backup on macOS or promise that FyAgent controls vendor rollback. FyAgent's own configuration writes use the safety child's shared owner.

Lifecycle errors remain distinct from login or inference reachability. Updating preserves an observed owner and never silently downgrades a newer installation. Formal Windows uses an ordinary-user adapter or refuses before process launch; no elevated fallback.

No new dependencies are planned. Claude package structure differs from Grok, so package-specific policy remains closed while mechanisms with a concrete second consumer may be extracted narrowly.
