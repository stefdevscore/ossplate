# Architecture

`ossplate` is intentionally small. The design goal is simple: one real CLI, three distribution channels.

## Runtime Shape

- Rust in [`core-rs/`](../core-rs) is the product.
- JavaScript in [`wrapper-js/`](../wrapper-js) is a package adapter.
- Python in [`wrapper-py/`](../wrapper-py) is a package adapter.
- The scaffold payload bundled into the wrappers is a distribution asset, not another implementation.

The main commands are:

- `version`
- `validate`
- `sync`
- `create`
- `init`
- `publish`

## Responsibilities

### Rust

- command parsing
- project identity loading from `ossplate.toml`
- validation logic
- metadata synchronization
- scaffold creation and initialization

Rust is the only layer that should know the semantics of project identity and owned metadata surfaces.

It also owns operator-facing source workflows such as local publish orchestration. That keeps registry sequencing and recovery behavior in the same product surface instead of scattering it across wrapper-specific tooling.

### JavaScript and Python

The wrappers own:

- packaged binary lookup
- platform/architecture target resolution
- local binary override support
- forwarding stdout, stderr, and exit code unchanged

Python publishes platform-specific wheels because each wheel bundles one native `ossplate` executable for its target.

They should not implement separate business logic, metadata rules, or command behavior.

### Scaffold Payload

The scaffold payload owns the generated-project baseline:

- manifests
- docs
- workflows
- wrapper launchers
- packaged binaries needed for installed-wrapper scaffold operations

It is curated by manifest and shipped as part of the wrapper artifacts so `create` and `init` work from installed distributions.

## Ownership Boundaries

`ossplate sync` owns only bounded identity-bearing surfaces. The details are in the ADRs, but the practical rule is simple: if a surface is not explicitly bounded, `sync` should not rewrite it.

Today that includes:

- Cargo, npm, and Python metadata fields
- wrapper package README identity
- the root README identity block
- workflow display names between `ossplate:workflow-name` markers

It does not own:

- workflow logic
- auth setup
- arbitrary prose outside bounded markers
- separate wrapper-specific product behavior

## Why It Scales

If the tool grows, keep the current rules:

- add product behavior in Rust
- treat wrappers as delivery adapters
- expand scaffold ownership only where the boundary is explicit and non-destructive

That gives the project a clean path toward a fuller hexagonal structure without forcing that complexity into the current starter.

## Related Decisions

- [ADR 0001: Rust Core, Thin Wrappers](./adrs/0001-rust-core-thin-wrappers.md)
- [ADR 0002: Sync Owns Bounded Identity Surfaces](./adrs/0002-sync-owns-bounded-identity.md)
- [ADR 0003: Ship A Curated Scaffold Payload](./adrs/0003-curated-scaffold-payload.md)
