These files are public, synthetic regression-test inputs reused from the
FyAgent 0.4.10 isolated Skills review. The private key is a test-only key for
the loopback server; it is not a user credential or a production signing key.

Tests never install this CA, modify hosts, change the system proxy, or use
real GitHub. Only the instance-local test client trusts the CA and routes the
logical GitHub archive URL to its own ephemeral loopback listener. All other
DNS is rejected. Runtime paths use an isolated temporary home and memory DB.

The original CA and leaf expired on 2026-10-06. Both synthetic certificates
were reissued with a fixed test validity of 2020-01-01 through 2040-01-01 to
avoid a two-day lifetime making later regression runs fail at TLS setup.
The original RSA 2048 server test key is retained. The CA remains a signing
CA with path length zero; the leaf remains an end entity with serverAuth and
the github.com DNS SAN. No production certificate verification is disabled.
The test server reports handshake errors and its result is checked before
backup/pending business assertions; the original archive bytes are unchanged.

ZIP SHA-256: 8066be9f3c182d912d61870ef0efec6ee932750c6bb06fc56a3d120a234a6668

`synthetic-skill.zip.b64` stores the same synthetic ZIP as plain Base64 text
so repository text checks can inspect the fixture without a binary exemption.
The test decodes it with the existing Base64 dependency, verifies the decoded
ZIP SHA-256 above before use, then sends those exact bytes from the TLS server
through the production download and archive extraction path.

The archive contains `audit-skills-main/audit-skill/SKILL.md` (an updated
synthetic Skill manifest) and `audit-skills-main/audit-skill/payload.txt` with
the three bytes `new`. The text representation changes no archive content.

The governance scanner exempts only `src-tauri/src/services/skill/update/fixtures/server-key.pem` from its private-key rule when its SHA-256 matches the pinned fixture digest; changed bytes or any other path remain reportable, including in history scans.
