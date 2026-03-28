# ADR 0003: Ship A Curated Scaffold Payload

## Status

Accepted

## Context

`create` and `init` need scaffold content even when `ossplate` is installed from npm or PyPI. A broad repo snapshot would work, but it would also ship tests, maintainer-only files, and accidental junk.

## Decision

- Installed wrapper artifacts ship a curated scaffold payload.
- `scaffold-manifest.json` is the allowlist for that payload.
- Artifact tests assert both required content and excluded content.

## Consequences

- Installed distributions can create and initialize projects end to end.
- Package contents stay intentional.
- Adding new scaffold files requires updating the manifest and tests.
