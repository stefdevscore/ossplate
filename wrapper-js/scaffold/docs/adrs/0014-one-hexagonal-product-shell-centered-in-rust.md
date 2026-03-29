# ADR 0014: One Hexagonal Product Shell Centered In Rust

## ADR-0014-01 Status

Accepted

## ADR-0014-02 Context

`ossplate` ships through three language ecosystems, which makes it easy to drift into describing three separate architectures. That would be misleading. The product has one behavioral core, one set of invariants, and one shell that grows through explicit slices.

## ADR-0014-03 Decision

- `ossplate` uses one hexagonal product shell.
- Rust is the only behavioral core inside that shell.
- JavaScript and Python remain thin delivery adapters with local structure only.

## ADR-0014-04 Consequences

- New product behavior goes into Rust first.
- Wrapper changes should stay operational unless the boundary itself changes.
- Architecture docs can describe one shell clearly instead of implying three peer product cores.
