# Recovery completion decision

Jev compared the bounded recovery completion choices and recommended
`atomic_db_finish` in `jev-recovery-decision.json`. GPT-6 accepted the narrow
transaction after reading the actual exit path: deleting the backup before
clearing `enabled` leaves an unrecoverable gap if the second operation fails.
Simply reversing those statements can leave a disabled app with an orphaned
backup that the normal retry path skips.

The accepted implementation restores owned files first, then changes only the
current application's enabled flag and deletes its backup in one database
transaction. Failure injection must prove rollback preserves both records and
that a retry completes. Jev's recommendation is advisory; the source review and
fixture results determine acceptance.

The separate legacy-proof repair must verify ownership before upgrading a
logical backup, persist the verified preimages and owned hashes before the first
file restore, and retain that proof across interruption. An external edit that
fails the initial check must leave the logical backup unchanged.
