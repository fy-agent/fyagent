These files are public, synthetic regression-test inputs reused from the
FyAgent 0.4.10 isolated Skills review. The private key is a test-only key for
the loopback server; it is not a user credential or a production signing key.

Tests never install this CA, modify hosts, change the system proxy, or use
real GitHub. Only the instance-local test client trusts the CA and routes the
logical GitHub archive URL to its own ephemeral loopback listener. All other
DNS is rejected. Runtime paths use an isolated temporary home and memory DB.

ZIP SHA-256: 8066be9f3c182d912d61870ef0efec6ee932750c6bb06fc56a3d120a234a6668

`synthetic-skill.zip.b64` stores the same synthetic ZIP as plain Base64 text
so repository text checks can inspect the fixture without a binary exemption.
The test decodes it with the existing Base64 dependency, verifies the decoded
ZIP SHA-256 above before use, then sends those exact bytes from the TLS server
through the production download and archive extraction path.

The archive contains `audit-skills-main/audit-skill/SKILL.md` (an updated
synthetic Skill manifest) and `audit-skills-main/audit-skill/payload.txt` with
the three bytes `new`. The text representation changes no archive content.
