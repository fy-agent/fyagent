# Design

## Owned paths

- `src-tauri/src/services/change_plan/**`
- `src-tauri/src/database/dao/change_plan.rs`
- focused Rust tests colocated with those owners

Do not modify `database/schema.rs`, `database/backup.rs`, `commands/mod.rs`, `lib.rs`, capability manifests, frontend files or Git state.

## Domain state

- Plan: ID, digest, target safe summary, created/expiry timestamps, consumed timestamp, separate DB/device baseline IDs, baseline digest, Secret capability result.
- Job: ID, Plan ID, state, terminal result/reason, safe timestamps and usage evidence `not_observed`.
- Event: monotonically ordered append-only phase/reason/timestamp.
- Status set: `planned`, `running`, `succeeded`, `warning`, `failed`; ambiguous terminal authority is represented by a recovery-required result/reason rather than green success.

## Apply algorithm

1. Under existing Provider mutation lock, reload Plan.
2. Validate digest, TTL, unconsumed state, current baselines and Secret capability.
3. In one DB transaction consume Plan, create Job and append first event.
4. Call a lock-held Provider switch primitive once.
5. Read back DB current, device current, target definition and live projection.
6. Persist terminal Job/Event. A recovery query only repeats step 5/6 and never step 4.

The implementer may expose integration seams for the later worker but must not register them globally.

## Secret capability

Fail closed unless existing saved data proves no new credential material is needed. The future
`projectionDigest` contract retained from #112 is design-only in this task:

- canonicalize the projected document with RFC 8785;
- domain-separate the digest input from every other FyAgent digest domain;
- encode the SHA-256 result as exactly 64 lowercase hexadecimal characters, without a
  `sha256:` prefix; and
- exclude the `projectionDigest` field itself from the projected document before hashing.

This task does not add an unused SecretRef model, Keychain backend, plaintext fallback, or Secret
UI. The Change Plan ledger's credential-neutral internal digests are separate contracts and must
not be represented as this future `projectionDigest`.

## Display sanitization

Reject blank/control-containing values and lexical paths: leading `/` or `\\`, drive prefix `[A-Za-z]:`, UNC, any path separators and case-insensitive `file:`. Return `Provider`; otherwise truncate to 80 Unicode scalar values.
