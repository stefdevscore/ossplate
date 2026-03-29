# ADR 0009: Centralize Release State Policy

## ADR-0009-01 Status

Accepted

## ADR-0009-02 Context

Release-state expectations had spread across several small scripts: version alignment, runtime package expectations, scaffold parity, package shape, and publish-readiness logic. That raised maintenance cost because release rules had to be changed in multiple places.

## ADR-0009-03 Decision

- Shared release and publish-readiness policy lives in `scripts/release-state.mjs`.
- CLI entrypoints should be thin wrappers over that policy.
- Release-state expectations must be testable without depending on live network state where a fake seam is sufficient.

## ADR-0009-04 Consequences

- Release policy changes can be made in one module instead of several overlapping scripts.
- Verification stays strong while the script surface gets simpler.
- Tests can exercise the release policy directly.
