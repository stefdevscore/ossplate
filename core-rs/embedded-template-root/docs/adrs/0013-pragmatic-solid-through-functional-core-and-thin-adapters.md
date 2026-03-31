# ADR 0013: Pragmatic SOLID Through Functional Core And Thin Adapters

## ADR-0013-01 Status

Accepted

## ADR-0013-02 Context

`ossplate` is mostly command orchestration, file transformation, packaging, and process execution. A class-heavy object model would add ceremony, while unchecked utility-style code would blur responsibilities and concrete dependencies too quickly.

## ADR-0013-03 Decision

- Apply SOLID pragmatically rather than through heavy OOP.
- Keep pure transforms and local policy functional where possible.
- Put IO, process execution, environment lookup, and packaging seams behind thin adapters or narrow orchestration boundaries.

## ADR-0013-04 Consequences

- `main.rs` stays a composition root rather than a product blob.
- Rust slices, wrapper adapters, and release helpers should get smaller reasons to change over time.
- New abstractions should exist to clarify ownership or isolate volatility, not to simulate classes.
