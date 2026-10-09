# Retired Agent Health page

I19 retires the standalone `/health` page and its dedicated Port, DTO, query,
native collector, command, ACL and SELECT-only DAO. Old hashes use the existing
unknown-route replacement to `/agents`; there is no replacement status center.

Account status uses the existing authentication summary and its authoritative
Managed Auth facts. Installation, configuration, models, proxy circuit breaking,
request logs, usage and diagnostics remain with their domain owners. Account
connection counts do not establish installation or remote request availability.

The accepted tradeoff removes centralized cross-software troubleshooting and
the exact recent-request snapshot. Retiring this on-demand page does not prove
CPU, memory or startup gains. Navigation and performance budgets remain owned
by the frontend Navigation and Quality contracts. Historical acceptance records
retain their original version facts.
