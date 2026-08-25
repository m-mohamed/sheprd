# Product foundation

Sheprd is a narrow project-to-workspace adapter for Herdr. It should remain
boring: discover a canonical Git checkout, focus or create a workspace, apply a
small explicit recipe, and report readiness.

## Non-goals

- no terminal multiplexer;
- no hidden agents or swarm scheduler;
- no model router;
- no task database;
- no acceptance authority;
- no retired peer-agent integration;
- no legacy receipt-backed fleet mode.

## Active boundaries

- Ratatui factory: operator command-and-control.
- Tuxedo: private task and done-signal truth.
- HQ: runbooks and Sol/Luna launcher.
- Sheprd: project discovery and workspace entry.
- Herdr: live runtime state.
- Git/checks: code and machine-verifiable evidence.

## Design rules

1. Herdr owns live IDs and runtime state.
2. Sheprd commands are explicit and JSON-friendly.
3. A recipe can shape a newly created workspace, never repair a live one.
4. Dirty repository work is never reset or cleaned automatically.
5. The Sol/Luna launcher owns its own bounded topology and private receipts.
6. Human acceptance remains required before merge, push, or task completion.
