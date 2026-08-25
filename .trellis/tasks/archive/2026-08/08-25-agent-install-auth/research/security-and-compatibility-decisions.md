# Research — Security and compatibility decisions

Reviewed: 2026-08-25

## Decision 1 — One executable installer core

The repository's executable-installer spec already declares the Codex Desktop implementation to be the first instance of a global contract. A new Qoder/TRAE/WorkBuddy downloader would duplicate source parsing, cancellation, staging, Windows helper/PackageBridge and macOS DMG transaction logic.

Decision: generalize Codex installer orchestration/policy seams and add product adapters. Concrete DMG/MSIX deployment code is reused only for matching artifact formats; a different evidenced vendor format gets a narrow closed adapter under the same core, not a second orchestration framework and not a generic executable runner.

## Decision 2 — Do not add remote publication fields as a trust gate

FyAgent's current contract intentionally delegates package-format/signature/dependency compatibility admission to native OS installers while retaining product-owned sources, local artifact identity, bounded transport and post-install operational verification.

Decision: this task must not reintroduce remote SHA/publisher/version/Team-ID/Gatekeeper comparison as a package admission gate, even if a vendor happens to publish those fields.

## Decision 3 — First-party CN distribution is policy, not geolocation

Selecting a Chinese product variant/source by IP geolocation can silently change product identity, account system or data-residency semantics. A first-party CN download is valid when the canonical catalog product is itself the CN variant or when the vendor documents it as an equivalent mirror.

Decision: catalog/product policy chooses source family; runtime reachability only reports whether that chosen first-party source is reachable. No third-party mirror fallback.

## Decision 4 — Website internals are discovery evidence, not runtime API

QoderWork/TRAE Work/WorkBuddy pages use first-party JS/CDNs, but a bundle-internal current URL can change without compatibility promises.

Decision: automatic install requires stable manifest/update/redirect evidence. A versioned URL discovered today is insufficient.

The runtime descriptor must additionally bind the checked release to an opaque backend revision and revalidate it at start, and every redirect target must stay inside the product's explicit first-party HTTPS host allowlist. This preserves the current Codex installer's checked-release coherence and avoids turning a remote redirect into a locator capability.

## Decision 5 — Preserve formal Windows fail-closed behavior

Claude now officially supports native Windows installation without Administrator, but FyAgent itself is elevated in formal Windows releases. Running Alice-controlled CLI/shims directly from elevated Bob/FyAgent would violate the frozen Shell-user contract.

Decision: a future Windows action may cross only an authenticated, closed ordinary-user helper. No generic command/path bridge. If that cannot be proven in this iteration, report unavailable.

## Decision 6 — Agent-owned auth stays agent-owned

Reading other tools' auth files creates secret-handling, format-drift and account-ownership liabilities while most vendors already expose their own login flow.

Decision: FyAgent invokes official CLI/app login and only consumes documented structured status. No status API means unknown.

## Decision 7 — OpenCode is provider-owned auth

OpenCode's `/connect` is explicitly a provider credential workflow and supports heterogeneous auth mechanisms.

Decision: no global OpenCode login badge or shared token import. The product action is connect/manage providers.

## Decision 8 — Codex managed-account identity is not workspace identity

CC Switch #5885 demonstrates that two users can share `chatgpt_account_id` / workspace identity. FyAgent's existing CodexOAuthManager currently uses that value as account key.

Decision: introduce a credential identity key and keep workspace/account ID as routing metadata only. Prefer a verified stable user subject; otherwise use a persisted FyAgent credential UUID rather than guessing.

## Decision 9 — Do not vendor Codex keyring internals in P1

OpenAI's current Codex source supports file/keyring/auto/ephemeral storage with platform-specific keyring behavior. Copying those internals into FyAgent would create a second credential-store implementation that must track Codex changes and OS keyring edge cases.

Decision: file projection reuses FyAgent's established atomic writer; other modes use Codex-owned login unless a stable public import/switch API is verified.

## Decision 10 — Token SSOT remains the managed auth store

Token copies in Provider rows or multiple auth files create refresh-rotation drift. Current FyAgent already has per-account refresh locks and a managed refresh-token store.

Decision: Provider binds by ID only. Any successful refresh/reconciliation updates the managed store; access tokens remain memory-only.

