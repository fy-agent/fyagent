# Claude Code CLI distribution review

## Primary sources

- https://code.claude.com/docs/en/setup — official npm distribution; Node.js 22+ for the native optional-package launcher; no sudo; native, Homebrew and npm ownership differ.
- https://code.claude.com/docs/en/cli-reference — `claude auth login`, `claude auth logout`, `claude auth status`; status is JSON by default and exits 0 when authenticated, 1 otherwise.
- https://code.claude.com/docs/en/authentication — vendor credential storage, including macOS Keychain; FyAgent must not invent an auth.json or promise server-side OAuth revocation can be undone.
- https://registry.npmjs.org/@anthropic-ai/claude-code/2.1.261 and matching `claude-code-{darwin,win32}-{x64,arm64}/2.1.261` metadata — exact package names, engines, optional dependency versions and SHA-512 values.

## Findings and reuse

Reviewed root `@anthropic-ai/claude-code@2.1.261` declares Node >=22.0.0, `postinstall = node install.cjs` and exact-version native optional packages. The reviewed five integrity values live in `tooling/claude_npm_manifest.json`; runtime never asks a mirror for `latest` or treats a mirror's hash as its own trust anchor. The existing Grok registry order, bounded process runner, installation discovery, exact npm plan and ordinary-user helper are the reuse authorities. No dependency or custom OAuth client is added.

Registry selection is per operation; global npmrc, shell profiles and system package managers are not changed. npm optional packages remain enabled. On npm versions that require script allowlisting, only the selected official root package is admitted. Different installation owners are never silently converted to npm; a newer installed version is not downgraded to the bundled version.

Portable tests do not prove Windows native helper execution. Installation does not prove login or inference availability on the user's network. No real user credential file or account was used during the package review.

## Installer script and scoped registry review

The exact root tarball was downloaded read-only, checked against the compiled SHA-512, and `package/install.cjs` was read without executing it. It resolves the native optional package and hardlinks/copies its binary to `bin/claude.exe`; it does not implement OAuth or download a fallback executable. Missing optional packages leave a non-working stub. Its Rosetta branch explicitly requires arm64 Node on Apple Silicon, so both host adapters require Node's architecture to equal the product architecture rather than inferring it from the OS alone.

Official npm config/scope references: https://docs.npmjs.com/cli/v11/using-npm/config/ and https://docs.npmjs.com/cli/v11/using-npm/scope/ . A scoped registry can override the general registry. A read-only probe of `npm config get @anthropic-ai:registry --@anthropic-ai:registry=https://registry.npmmirror.com/` returned the supplied registry without changing npmrc. Both closed products therefore pass matching general and scope-specific registry flags for this invocation only.
