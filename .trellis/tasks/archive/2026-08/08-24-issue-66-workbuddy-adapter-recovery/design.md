# Design notes

`workbuddy_models_update` is a closed UCP operation with one business step,
`save_workbuddy_models`, and two native resources: the primary models document
and its recovery backup. The existing schema-v20 columns keep their historical
names but store only fixed safe target codes for this adapter; no schema change
is required.

The WorkBuddy service exposes a crate-private lock-held facade. Planning uses a
pure in-memory document transform to validate the request and capture safe
counts plus private baseline bytes/revision; it neither writes nor issues the
legacy overwrite token. Execution holds the same mutation lock across private
precheck, atomic admission, internal overwrite-token consumption, existing
writer call, and exact file readback.

The UCP proof map keeps the full request and exact baseline snapshot in memory.
The database stores only random proof/epoch IDs and approval-scoped digests over
non-sensitive public metadata. On restart the proof is unavailable, so a ready
plan is stale and an interrupted post-write job becomes recovery-required;
neither path replays the writer.

The V2 shared Change Plan surface becomes operation-aware. WorkBuddy add,
update and delete all enqueue an external plan into that same controller;
legacy direct-save and delete/overwrite confirmation UI is not used by the V2
WorkBuddy path.
