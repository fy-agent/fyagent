# PR governance readback

- Replacement PR: https://github.com/fy-agent/fyagent/pull/135
- Draft created: yes
- Initial Required CI: pass on `5b2d904b03549621f2caea9497ac6c7dcbcf23a5`; push run `32663818280`, pull-request run `32663856167`
- Governance-head Required CI: pass on `3c27acaf97f2e1a052552a7f5e392debf64fc549`; push run `32664954837`, pull-request run `32664957087`
- Latest finalization-head Required CI: pending; authoritative live check on PR #135
- Commit count versus latest main: 1 at governance head; fresh latest-head readback pending
- Product diff digest before/after governance amend: `cd38c07602e747179444641b6b77da9db1d70876f5507d2ccca3e21be421d018` → `cd38c07602e747179444641b6b77da9db1d70876f5507d2ccca3e21be421d018`
- Ready for Review: pending
- Requested reviewer: `python-rust` (request deferred to handoff)
- `main` merged: no
- Release published: no

| Source PR | Expected final state | Comment/readback |
|---|---|---|
| #108 | closed, merged=false | [migration comment](https://github.com/fy-agent/fyagent/pull/108#issuecomment-5388316929); closed after initial Required |
| #109 | closed, merged=false | [existing closeout](https://github.com/fy-agent/fyagent/pull/109#issuecomment-5387511324); provenance only, no new state mutation |
| #112 | closed, merged=false | [migration comment](https://github.com/fy-agent/fyagent/pull/112#issuecomment-5388317447); already closed before this governance pass |
| #113 | closed, merged=false | [migration comment](https://github.com/fy-agent/fyagent/pull/113#issuecomment-5388317703); closed after initial Required |
| #114 | closed, merged=false | [migration comment](https://github.com/fy-agent/fyagent/pull/114#issuecomment-5388318316); already closed before this governance pass |
| #115 | closed, merged=false | [migration comment](https://github.com/fy-agent/fyagent/pull/115#issuecomment-5388318585); closed after initial Required |
| #130 | closed, merged=false | [migration comment](https://github.com/fy-agent/fyagent/pull/130#issuecomment-5388319210); derived PR closed after initial Required |

Out-of-scope Drafts #132 (standalone SecretRef) and #134 (standalone executor) remained open and unchanged.

Final handoff state is `awaiting_human_review` only after the latest-head Required gate passes and PR #135 is read back Ready. GitHub is authoritative for that post-commit state; this file is not merge authorization.
