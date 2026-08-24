# Clarify CI force-update wording

## Goal

Remove stale squash-main wording from the defensive push `before` fallback
scenario now that canonical mainline policy is Merge Queue + merge commit.

## Requirements

- Keep the existing unreachable-push-`before` CI behavior unchanged.
- Describe force-update/history-rewrite as an abnormal/historical input, not the
  supported `dev/laiyongjie` synchronization path.
- Keep canonical dev synchronization in `github-merge-governance.md`: clean dev
  fast-forwards to final main; independent commits require a PR.

## Acceptance Criteria

- [x] `github-ci-workflow.md` contains no wording implying squash-main alignment
      is the current dev synchronization workflow.
- [x] No CI YAML/script/test behavior changes.
- [x] `git diff --check`, contracts, direct-session prearchive and post-archive
      contracts pass.
- [x] Task is archived before #142 is handed back to Merge Queue.
