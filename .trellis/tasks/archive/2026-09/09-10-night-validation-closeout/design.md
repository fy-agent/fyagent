# Repair boundary

The published PR contains a reviewed health screenshot but its two inventory
assertions still expect the previous count. Its archived PRD and worktree field
also retain concrete workstation home paths. These are post-archive validation
defects, owned by the existing inventory test and archived task metadata.

Update the exact count, explicitly bind the added screenshot to its reviewed
digest, and use portable home placeholders. Keep all mutation/negative cases.
The CI browser failures follow an external package-index hash mismatch and
missing browser binaries; preserve that evidence and rerun hosted checks on the
repaired head without weakening package verification or browser assertions.

Work takes place on an isolated branch based on the published #187 head. Git
merge-tree and ancestry checks do not mutate any existing checkout. Worktree
status and diff hashes are saved separately in a local-only audit packet.
