# Design

Use the existing AgentAuthPort for official Grok CLI handoff. Move the single
session hook to shared/features; keep Agent cards as routing summaries. `/auth`
owns a focused official CLI panel with logout confirmation and explicit unknown
native login state. No new OAuth, generic command, secret handling or native writes.

Managed account consumer advertisements and account-page login shortcuts must
exclude unavailable projections. Native Grok summaries expose unknown request
source/preserved-session and refresh only while the projection gate is false.

Keep managed mutations serialized, but retain their results by connection ID.
Failure/partial result retry reopens the existing preview flow with the current
revision. Other targets' successful results persist. Existing recovery control
receives only the consumer's closed file targets.

Strict transport binds start responses to requested Agent/intent and poll/stop
responses to session ID; shared hook also rejects cross-session/cross-generation
results. Failure to recover an active session blocks duplicate starts until an
explicit recovery retry succeeds.
