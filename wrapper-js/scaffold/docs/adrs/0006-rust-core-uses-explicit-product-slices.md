# ADR 0006: Rust Core Uses Explicit Product Slices

## ADR-0006-01 Status

Accepted

## ADR-0006-02 Context

As `ossplate` grew, `core-rs/src/main.rs` accumulated multiple behaviors: CLI dispatch, metadata sync, scaffold projection, and release orchestration. Keeping all of that in one file would make the hexagonal shell aspirational instead of real.

## ADR-0006-03 Decision

- The Rust core uses explicit product slices for core execution, sync, scaffold synthesis, and release orchestration.
- `main.rs` remains the CLI entrypoint and top-level dispatcher.
- Slice extraction is structural; it should not create alternate product semantics.

## ADR-0006-04 Consequences

- The codebase can grow by adding or refining slices instead of expanding one monolithic entrypoint.
- Docs can map architecture claims to concrete Rust modules.
- Future architecture checks can validate slice boundaries more directly.
