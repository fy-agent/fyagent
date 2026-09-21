# Identity cold-create concurrency regression

Status: implementation complete; final focused validation passed. Identity product files are released to the coordinator.

## Reproduced cause

The coordinator's production migration run passed 162 tests and failed three. This package owns only the identity failure; Hermes/Gemini remain with the coordinator. The identity failure occurred while opening `identity.lock`, before machine fingerprint or namespace computation. Each of the twelve workers already injects its own synthetic directory and fingerprint, so the `test-hooks` visibility change is not established as the cause.

A new platform regression starts twelve callers against one previously absent lock, retaining the temporary directory until every result is collected. Before the fix, the first round reproduced multiple `NotFound` / OS error 2 results while other callers opened the same actual file instance. No directory or file was deleted by the test. This identifies a concurrent cold-create failure on the local macOS filesystem. An isolated run of the original higher-level test passed, confirming that one successful run was insufficient to close the observed intermittent defect.

## Smallest change

The macOS descriptor-relative primitive now uses `O_CREAT|O_EXCL` to select one creator. For a nonexclusive caller, `AlreadyExists` switches to an ordinary read/write open without creation flags. `O_NOFOLLOW`, `O_CLOEXEC`, private mode, EINTR handling, pinned-directory access and lock-instance validation remain. Exclusive staging still refuses collisions. There is no process-global mutex, expanded platform support or dependency change.

The new regression performs 32 fresh cold-creation rounds with twelve threads each and requires every result to identify the same file instance. The existing namespace and child-process tests continue to check actual serialized state transactions. Lock-open errors now retain only error kind and numeric OS code, without paths or raw platform messages, so a future failure does not discard its decisive diagnostic.

## Actual validation

- Before fix: original `concurrent_threads_keep_every_namespace_and_one_installation` passed in isolation.
- Before fix: new `concurrent_lock_creation_opens_one_file_instance` failed with captured `NotFound` / errno 2, as above.
- After fix: `mise run rust:test session_manager::migrate::identity` was blocked before executing tests by concurrent `capability.rs:172`, undefined `provider_id` in `detect_version`. This package did not edit that file.
- After the coordinator fixed that capability compilation error, the final canonical `mise run rust:test session_manager::migrate::identity` executed the actual production library tests: **24 passed, 0 failed**. This includes 32 cold-create rounds, the original twelve-thread namespace regression, six child-process transactions, corruption/copy/replacement checks and pinned-directory publication checks.
- Only the two owned identity files were formatted with the locked rustfmt. No full suite was run by this package; the coordinator retains the final integrated gate.

No real identity, session store, credential, model or Windows execution was used. Earlier report speculation that EINTR explained the intermittent open failure is superseded by this captured cold-create failure; EINTR handling remains correct defensive behavior.
