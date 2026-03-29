# ADR 0003: Ship A Curated Scaffold Payload

## ADR-0003-01 Status

Accepted

## ADR-0003-02 Context

`create` and `init` need scaffold content even when `ossplate` is installed from npm or PyPI. A broad repo snapshot would work, but it would also ship tests, maintainer-only files, and accidental junk.

## ADR-0003-03 Decision

- Installed wrapper artifacts ship a curated scaffold payload.
- `scaffold-payload.json` is the allowlist for that payload.
- Artifact tests assert both required content and excluded content.

## ADR-0003-04 Consequences

- Installed distributions can create and initialize projects end to end.
- Package contents stay intentional.
- Adding new scaffold files requires updating the manifest and tests.
