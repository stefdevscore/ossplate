# Architecture

`ossplate` is one product distributed through three package channels.

## ARCH-01 Runtime Shape

- Rust in [`core-rs/`](../core-rs) is the behavioral core.
- JavaScript in [`wrapper-js/`](../wrapper-js) is a package adapter.
- Python in [`wrapper-py/`](../wrapper-py) is a package adapter.
- The scaffold payload shipped in the wrappers is a distribution asset, not another implementation.

The product commands are:

- `version`
- `validate`
- `sync`
- `create`
- `init`
- `publish`

## ARCH-02 Current Rust Slices

The Rust core is now split into a few explicit slices:

- core execution in `main.rs`: CLI parsing and top-level dispatch
- `sync`: bounded identity-bearing metadata validation and rewrite logic
- `release`: publish command semantics and adapter invocation boundaries
- `scaffold`: template discovery, projection, hydration, and identity override flow
- verification: tests and release checks that enforce the intended boundaries

This is still one product architecture. It is not three equal application stacks across Rust, TypeScript, and Python.

## ARCH-03 Responsibilities

### ARCH-03A Rust

Rust owns:

- command semantics
- project identity loading from `ossplate.toml`
- metadata ownership rules
- scaffold creation and initialization
- operator-facing publish orchestration

Rust is the only layer that should know what the product means.

### ARCH-03B JavaScript And Python

The wrappers own:

- packaged binary lookup
- target resolution
- local binary override support
- environment setup
- forwarding stdout, stderr, and exit code unchanged

They do not own product rules, metadata policy, or alternate command behavior.

### ARCH-03C Scaffold Payload

The scaffold payload owns the generated-project baseline:

- manifests
- docs
- workflows
- wrapper launchers
- packaged binaries needed for installed-wrapper scaffold operations

It is curated by `scaffold-payload.json` and shipped so installed wrappers can still run `create` and `init`.

## ARCH-04 Ownership Boundaries

`ossplate sync` owns only bounded identity-bearing surfaces. Today that includes:

- Cargo, npm, and Python metadata fields
- wrapper package README identity
- the root README identity block
- workflow display names between `ossplate:workflow-name` markers

It does not own:

- workflow logic
- auth setup
- arbitrary prose outside bounded markers
- wrapper-specific product behavior

## ARCH-05 Forward Path

The forward-looking slice model lives in [Hexagonal Shell](./hexagonal-shell.md).

Keep the rules simple as the product grows:

- add product behavior in Rust
- keep JS and Python adapter-local
- expand scaffold ownership only where the boundary is explicit
- treat verification as architecture enforcement, not generic QA

## ARCH-06 Related Decisions

- [ADR 0001: Rust Core, Thin Wrappers](./adrs/0001-rust-core-thin-wrappers.md)
- [ADR 0002: Sync Owns Bounded Identity Surfaces](./adrs/0002-sync-owns-bounded-identity.md)
- [ADR 0003: Ship A Curated Scaffold Payload](./adrs/0003-curated-scaffold-payload.md)
- [ADR 0004: Release Orchestration Stays Core-Owned](./adrs/0004-release-orchestration-stays-core-owned.md)
- [ADR 0005: Verification Enforces Source And Installed Contracts](./adrs/0005-verification-enforces-source-and-installed-contracts.md)
- [ADR 0006: Rust Core Uses Explicit Product Slices](./adrs/0006-rust-core-uses-explicit-product-slices.md)
