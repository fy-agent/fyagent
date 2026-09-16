# Review and bounded migration decisions

## Architecture / source comparison

The supplied Cherry snapshot is the reference, not a mutable latest branch.
Its OAuth runtime coalesces renewal, conditionally persists rotated tokens,
and cancels the first 401 response before one replay. FyAgent now implements
those responsibilities inside its existing credential lock, native vault,
generation CAS and Provider attempt. It does not import the Electron token
store, create a second OAuth owner, or transfer native consumer credentials.

Cherry separates gateway desired enablement from the bound listener and
serializes start/stop reconciliation. FyAgent retains the smaller existing
ProxyService activation/compensation transaction instead of adding another
reconciler or transient lease framework. Listener status is observed from the
actual owner, and effective Provider selection uses the same authority as
forwarding. Saved/default credentials do not establish a running route.

Cherry's Codex/Grok adapters normalize Responses at the authenticated transport
boundary. FyAgent reuses its existing Responses/Claude converters, streaming
tool events, usage accounting and model sanitizer, with a small final policy
owner after overrides. Compact schemas remain separate. Model route identity
and upstream model identity remain distinct without another model catalog.

Cherry's own-login CLI switch may clear tool-specific auth files. That behavior
is deliberately not migrated: this task preserves the user's native/direct
workflow. Subscription binding uses local endpoints and markers, leaving
Codex/Grok native auth bytes, unrelated configuration and MCP settings intact.
The local boundary is the existing trusted loopback proxy, not a new public
gateway with Cherry's per-client key-management product surface.

## Scope / compatibility review

- No new runtime dependency, database migration, token store or service process.
- Existing xAI binding IDs and compatibility command retained; OpenAI uses a
  distinct custom Codex slot rather than the reserved native `openai` ID.
- Claude Code and Grok Build activate through existing transactional writers;
  Codex stays a draft until the existing Change Plan is confirmed and applied.
- Native consumer projection source files, secret backend implementation and
  API-key provider configuration flows are not rewritten.
- Proxy observation keeps the existing per-provider default connection slots.
  Explicit non-default accounts are supported by routing and binding; this
  task does not invent additional per-account overview connection rows.

## Review findings and prevention

### Cross-layer fixture contract: JSON versus SSE

The initial 401 replay fixture returned JSON after the OpenAI adapter requested
SSE. The existing converter correctly rejected the missing terminal event.
The fixture now honors the outbound stream flag and emits the shared complete
Responses event sequence. No semantic-success condition was weakened.

### Effective routing authority

The first observer read only the database current marker, whereas forwarding
uses the effective local selection. The observer now delegates to that same
owner. A regression deliberately makes the local selection disagree with the
database marker and requires disconnected until the selected binding is
restored. Locked/unreadable observation remains unknown, not connected.

### Implicit endpoint assumption across compensation

A throwaway bind/release probe was replaced by actual ephemeral listeners.
That is sufficient for ordinary roundtrip tests but not for a confirmed
Change Plan spanning a deliberate stop/restart: the newly assigned port changes
the target projection. Moving preview after the first bind did not fix the
later endpoint drift. The concurrency fixture now allocates its port through
ProxyService, stops it, and retains that confirmed endpoint for the intended
restart. Existing target-projection and rollback assertions stay unchanged.
This distinction is captured in the proxy-runtime SPEC.

### Repository gate propagation / baseline exceptions

The new command required an explicit renderer ACL count/membership assertion
and reviewed source identity updates. Four task-owned sealed Rust files were
reviewed after formatting; their platform guards and dispatch boundaries are
unchanged. Only their exact final SHA-256 entries were updated, with no scanner
or allowance changes.

Two pre-existing bookkeeping defects were found by the complete gate. The
`18d63c5c` helper-priority commit added eight test lines without updating the
`tests/miseTaskContract.test.ts` seal; that diff was reviewed and its single
manifest entry synchronized, without changing the test. The prior FDE task's
archived verification header contained a concrete workstation home path; that
one path was replaced with `<workspace-root>`. No FDE code or helper behavior
was changed. These are disclosed gate repairs, not proxy feature scope.

## Evidence limits

Listener tests use actual loopback HTTP and the real Provider, vault/CAS,
conversion and compensation owners, but synthetic grants and upstream replies.
Browser fixtures prove renderer interactions and native contract payloads, not
live provider authorization. Successful local adoption is not proof of paid
entitlement, quota, all model availability, signed desktop runtime acceptance,
or matching-host Windows behavior. No real user account was charged or used
for an unattended inference request.
