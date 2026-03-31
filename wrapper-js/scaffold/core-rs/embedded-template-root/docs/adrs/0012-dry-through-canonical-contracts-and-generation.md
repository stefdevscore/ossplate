# ADR 0012: DRY Through Canonical Contracts And Generation

## ADR-0012-01 Status

Accepted

## ADR-0012-02 Context

`ossplate` has to keep Rust, npm, PyPI, scaffold payloads, release checks, and installed-wrapper artifacts aligned. Repeating the same target lists, scaffold contracts, or release expectations in several places creates drift faster than normal application code because multiple packaging channels are involved.

## ADR-0012-03 Decision

- Prefer one machine-readable contract over repeated inline lists.
- Prefer generated packaging assets over hand-maintained mirrors where practical.
- Prefer one shared policy module over several overlapping assertion scripts.

## ADR-0012-04 Consequences

- New runtime, scaffold, or release rules should start from one canonical source.
- Duplication is only acceptable when it is a deliberate distribution artifact.
- Refactors should remove repeated policy first, not just repeated prose.
