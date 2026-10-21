# Design contract

Use a dedicated native omp configuration service and typed FeaturePort; renderer supplies logical provider/model fields; native code resolves the current profile and effective files. Preserve the existing FyAgent UI language, tokens and dialog/preview behavior. Add actual catalog/model capability and installation discovery without accidentally enabling MCP/Skills assignment through PRODUCT_DIRECTORY-derived lists.

Resolve effective profile and .yml/.yaml files according to omp v18.12.5. Model providers live in models.yml, optional default role in config.yml. Patch only selected provider/model fields. Byte-preserving YAML changes must be verified with realistic fixtures; generic serde_yaml reserialization loses comments and is insufficient. Existing valid auth/key references remain opaque, command references are never executed by editing, environment-key references need actionable validation. Native OAuth agent.db is outside the provider editor.

Preview is zero-write and binds normalized intent, native-owned effective paths, profile and complete revisions. Apply rereads under the writer lock, verifies all touched files, backs up exact preimages, commits atomic replacements and rereads. Partial failures restore only this operation's still-current bytes; external edits are preserved. Failures return errors; successful apply returns redacted readback and per-file written flags. Reuse existing native storage/secret/revision owners where applicable.

Windows version detection uses the existing authenticated ordinary-user helper with a fixed OmpProbe action. Filesystem commands run outside the UI thread. New YAML collections are populated before insertion into the CST, including existing flow-style documents; existing nodes remain local edits.

Tests cover .yml/.yaml precedence, legacy JSON preservation, profiles/overrides, invalid YAML, unknown fields/comments/key references, model IDs containing slash, idempotency, concurrent changes and two-file failure recovery. Omp CLI proof uses an isolated test home with fake credentials and no generation request. Full source integration is performed by the parent via cherry-pick.
