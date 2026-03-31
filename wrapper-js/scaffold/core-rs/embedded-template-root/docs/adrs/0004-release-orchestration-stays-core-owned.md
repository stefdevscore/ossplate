# ADR 0004: Release Orchestration Stays Core-Owned

## ADR-0004-01 Status

Accepted

## ADR-0004-02 Context

`ossplate` release behavior spans version policy, registry sequencing, rerun safety, and local recovery. The publish mechanics still rely on Node helpers because they already encode tested registry-specific behavior, but the product should not let release policy drift into wrapper packages or ad hoc CLI glue.

## ADR-0004-03 Decision

- Rust owns the `publish` command contract and release orchestration boundary.
- Registry-specific mechanics may remain in helper scripts as adapters.
- Local operator publish is recovery-oriented and host-limited, not a substitute for the full automated release.

## ADR-0004-04 Consequences

- `main.rs` stays thin while the Rust release slice owns publish semantics.
- Helper scripts can evolve without changing the product boundary.
- Release docs and tests must describe rerun-safe behavior and the host-limited local publish model explicitly.
