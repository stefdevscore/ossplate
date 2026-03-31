# ADR 0010: Scaffold Mirrors Are Generated Packaging Assets

## ADR-0010-01 Status

Accepted

## ADR-0010-02 Context

Checked-in wrapper scaffold mirrors created parity churn and invited accidental direct edits. The real product requirement is narrower: package builds must be able to generate wrapper scaffold payloads from the root source checkout, and installed distributions must still carry those generated payloads.

## ADR-0010-03 Decision

- The root source checkout is authoritative.
- Wrapper scaffold payloads are generated at package/build time from `scaffold-payload.json`.
- Verification and CI must fail clearly if tracked wrapper scaffold mirrors reappear or if scaffold generation from canon stops working.

## ADR-0010-04 Consequences

- Maintainers edit root sources only.
- Wrapper package builds must stage scaffold payloads before packaging.
- Verification shifts from tracked-mirror parity to generation-from-canon integrity.
