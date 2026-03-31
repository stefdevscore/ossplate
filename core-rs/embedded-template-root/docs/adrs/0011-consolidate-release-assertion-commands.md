# ADR 0011: Consolidate Release Assertion Commands

## ADR-0011-01 Status

Accepted

## ADR-0011-02 Context

After release-state policy moved into one module, the remaining `assert-*` scripts were mostly thin wrappers over that shared logic. Keeping several near-empty entrypoints made verify, CI, and operator docs harder to read than necessary.

## ADR-0011-03 Decision

- `scripts/release-check.mjs` is the single release assertion CLI.
- It exposes explicit subcommands for release-state, scaffold-mirror, and publish-readiness checks.
- The shared policy still lives in `scripts/release-state.mjs`.

## ADR-0011-04 Consequences

- Verify and CI are easier to scan.
- Release assertions have one stable operator-facing entrypoint.
- The helper-script surface is smaller without changing release semantics.
