# Continuous integration flow

The current `.github/workflows/ci.yml`, repository classifier/aggregate
scripts, and their contract tests define triggers, change classification, job
selection, native runner evidence, cancellation, and the stable
`CI / Required` result. Retained CI workflow notes under `.trellis/spec/` are
optional AI-assistance review material.

## Pull request and merge-group flow

```text
explicit base SHA + head SHA
  -> repository-owned change classifier
  -> known responsibility domains
  -> affected jobs, or the lightweight docs/contracts path
  -> one required aggregate result
```

Path ownership lives in the classifier rather than being duplicated across
workflow filters. A new path that has no mapping is a classification failure,
not an implicit full build or an implicit skip. Control-plane changes force the
full domain set.

## Branch-push flow

Pushes to the development release branch and to `main` run the complete CI
surface. A force-update whose previous tip is no longer in the clone still
runs that full surface: the workflow substitutes `head` as an empty classifier
base instead of failing `Classify Changes`.

Use `mise run check` for the complete current-host local gate. Local success
does not produce a GitHub required check or matching-architecture native
evidence.
