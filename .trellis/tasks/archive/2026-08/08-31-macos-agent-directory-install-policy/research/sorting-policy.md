# Agent Directory Sorting Policy

Date: 2026-08-31

## 1. User outcome

After a scan completes:

1. installed products appear before products that are not installed;
2. among installed products, QoderWork, TRAE Work and WorkBuddy appear first;
3. uninstalled products stay below installed products.

The implementation must not lie about rows whose state could not be determined.

## 2. Canonical order

The existing product order remains the stable tie-breaker:

```text
QoderWork CN
TRAE Work CN
WorkBuddy
Grok Build
Codex
Claude Code
OpenCode
```

The backend catalog and shared product directory must not be globally reordered by runtime state.

## 3. Bucket order

```text
0 installed_domestic
1 installed_other
2 unresolved
3 not_installed
```

### Installed

Both states prove an installation exists:

```text
installed
installed_not_runnable
```

### Unresolved

These states do not prove absence:

```text
pending
unknown
unavailable
technical error
no current successful result
current scan failed while a stale result is retained
```

### Not installed

Only an authoritative current readiness with exact `not_installed` enters the final bucket.

## 4. Stable sort key

For each current catalog entry:

```text
key = (bucket_rank, canonical_index)
```

No name/locale/version comparison participates.

The helper sorts a copied array. It does not mutate the query result, backend catalog or `PRODUCT_DIRECTORY`.

## 5. Scan lifecycle

### Initial load

```text
idle -> scanning:
  render canonical order

individual rows settle:
  update card evidence/actions only
  do not reorder

scan complete:
  compute and commit one final order
```

### Rescan

```text
previous completed order exists
  -> start rescan
  -> keep previous committed order
  -> update card refresh indicators
  -> commit a new order once the scan completes
```

### Lifecycle action

After a successful install/update action, the existing hook performs an authoritative readiness reread. If no scan is active, that patch may immediately recalculate the committed order. This makes a newly installed product move into an installed bucket without forcing another complete scan.

## 6. Stale-result rule

The scan store intentionally retains the previous result if a later request fails. Two different projections are required:

```text
card display/configuration:
  may show previous installed result
  marks refresh failure
  remains configurable under existing contract

runtime ordering:
  current failure means unresolved
  does not assert current installed fact
```

This preserves useful stale UI without presenting stale evidence as current sorting truth.

## 7. Domestic metadata

Add the classification to existing shared product metadata:

| Agent ID | Priority |
| --- | --- |
| `qoderwork` | domestic |
| `trae-work` | domestic |
| `workbuddy` | domestic |
| all others | standard |

Do not infer from `CN` in a label, URL domain, catalog position or variant ID. Do not create a second list inside `AgentDirectory`.

## 8. Worked examples

### Example A

```text
QoderWork: not_installed
TRAE Work: installed
WorkBuddy: not_installed
Grok: installed
Codex: installed
Claude: not_installed
OpenCode: installed
```

Result:

```text
TRAE Work
Grok
Codex
OpenCode
QoderWork
WorkBuddy
Claude
```

The uninstalled domestic products do not move above installed non-domestic products.

### Example B

```text
QoderWork: installed_not_runnable
TRAE Work: unknown
WorkBuddy: installed
Grok: error
Codex: not_installed
Claude: unavailable
OpenCode: installed
```

Result:

```text
QoderWork
WorkBuddy
OpenCode
TRAE Work
Grok
Claude
Codex
```

### Example C — stale failure

Previous scan said Codex installed. Current scan fails for Codex while all other rows succeed.

Result:

- Codex card may remain configurable and show stale installed data with refresh-failed state.
- Codex sorts in `unresolved`, below currently confirmed installed rows and above confirmed not-installed rows.

## 9. Focus/accessibility

- Card React key remains product ID.
- Do not use array index as key.
- The reorder is one DOM move after scan completion, not repeated live reshuffling.
- If a card action has focus when order commits, the focused element should remain the same product/action.
- Screen-reader status announcements should report scan completion separately; do not announce every positional move.

## 10. Test matrix

### Pure classification

- all install states;
- installed-not-runnable;
- domestic/standard;
- current failure with/without stale readiness;
- no readiness before/after complete.

### Stable ordering

- every bucket represented;
- ties retain canonical order;
- all installed;
- all not installed;
- no installed;
- no current successful results;
- input immutability.

### State lifecycle

- first scan does not reorder until finish;
- row settlement does not reorder;
- rescan preserves committed order;
- rescan finish commits once;
- lifecycle `applyReadiness` after completion reorders;
- `applyReadiness` during scanning waits for completed order.

### UI/accessibility

- expected visible card sequence;
- focused action remains focused;
- links/actions still reference the correct product after movement;
- keyboard navigation follows the final visual/DOM order.

