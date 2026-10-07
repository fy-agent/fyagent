These files are public, synthetic regression-test inputs reused from the
FyAgent 0.4.10 isolated Skills review. The private key is a test-only key for
the loopback server; it is not a user credential or a production signing key.

Tests never install this CA, modify hosts, change the system proxy, or use
real GitHub. Only the instance-local test client trusts the CA and routes the
logical GitHub archive URL to its own ephemeral loopback listener. All other
DNS is rejected. Runtime paths use an isolated temporary home and memory DB.

ZIP SHA-256: 8066be9f3c182d912d61870ef0efec6ee932750c6bb06fc56a3d120a234a6668
