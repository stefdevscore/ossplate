# ADR 0010: Scaffold Mirrors Are Generated Packaging Assets

## ADR-0010-01 Status

Accepted

## ADR-0010-02 Context

`wrapper-js/scaffold/` and `wrapper-py/src/ossplate/scaffold/` are necessary packaging inputs for installed-wrapper scaffold support, but they are mirrored payloads derived from the root source checkout. Treating them like normal source trees creates parity churn and invites accidental direct edits.

## ADR-0010-03 Decision

- The root source checkout is authoritative.
- The scaffold mirrors are generated packaging assets derived from `scaffold-payload.json`.
- Verification and CI must fail clearly when those mirrors drift from source.

## ADR-0010-04 Consequences

- Maintainers should edit root sources, then regenerate mirrors.
- Mirror integrity becomes an explicit verification concern.
- A future move to untracked/generated-only mirrors stays possible without changing the authority model again.
