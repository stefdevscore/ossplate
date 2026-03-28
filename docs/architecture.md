# Architecture

`ossplate` is intentionally small, but its boundaries are already explicit.

## Core Shape

- Rust in [`core-rs/`](../core-rs) is the only source of product logic.
- JavaScript in [`wrapper-js/`](../wrapper-js) is an adapter that resolves the packaged binary and forwards arguments.
- Python in [`wrapper-py/`](../wrapper-py) is an adapter that resolves the packaged binary and forwards arguments.
- The scaffold payload bundled into the wrappers is a distribution asset, not a second implementation.

## Responsibilities

### Rust Core

The Rust core owns:

- command parsing
- project identity loading from `ossplate.toml`
- validation logic
- metadata synchronization
- scaffold creation and initialization

It is the only layer that should know the semantics of project identity and owned metadata surfaces.

### JS And Python Wrappers

The wrappers own:

- packaged binary lookup
- platform/architecture target resolution
- local binary override support
- forwarding stdout, stderr, and exit code

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

`ossplate sync` owns only bounded identity-bearing surfaces.

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

## Scaling Direction

If the tool grows, keep the current rule:

- add product behavior in Rust
- treat wrappers as delivery adapters
- expand scaffold ownership only where the boundary is explicit and non-destructive

That gives the project a clean path toward a fuller hexagonal structure without forcing that complexity into the current starter.
