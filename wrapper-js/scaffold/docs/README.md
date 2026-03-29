# Documentation

`ossplate` ships one CLI across Cargo, npm, and PyPI without maintaining three product implementations.

## DOC-01 Canonical Path

- [Architecture](./architecture.md): current product shape and ownership boundaries
- [Hexagonal Shell](./hexagonal-shell.md): forward-looking slice model and current implementation status
- [Adoption Guide](./customizing-the-template.md): how to adopt, create, init, and rename safely
- [Testing](./testing.md): the real verification contract, CI, and live install confidence path
- [Releases](./releases.md): operator release flow, rerun behavior, and local publish recovery

Read those five first. Everything else is either reference material or a decision record.

## DOC-02 Reference / Reports

- [Live E2E](./live-e2e.md): installed-registry smoke flow and capture behavior
- [Package Size Report](./package-size-report.md): cross-package size summary and tradeoffs
- [JavaScript Package Size Report](./javascript-package-size-report.md): npm/runtime-package footprint analysis
- [Python Package Size Report](./python-package-size-report.md): wheel footprint analysis and target sizing

Use these when debugging a specific operational topic rather than learning the product model.

## DOC-03 Decision Records

- [ADR Index](./adrs/README.md): canonical decision trail ordered by product shape, ownership contracts, and release/verification concerns
- [ADR 0001: Rust Core, Thin Wrappers](./adrs/0001-rust-core-thin-wrappers.md)
- [ADR 0006: Rust Core Uses Explicit Product Slices](./adrs/0006-rust-core-uses-explicit-product-slices.md)
- [ADR 0002: Sync Owns Bounded Identity Surfaces](./adrs/0002-sync-owns-bounded-identity.md)
- [ADR 0003: Ship A Curated Scaffold Payload](./adrs/0003-curated-scaffold-payload.md)
- [ADR 0007: Split Scaffold Payload And Source-Checkout Contracts](./adrs/0007-split-scaffold-payload-and-source-checkout-contracts.md)
- [ADR 0010: Scaffold Mirrors Are Generated Packaging Assets](./adrs/0010-scaffold-mirrors-are-generated-packaging-assets.md)
- [ADR 0008: Stage Neutral Runtime Artifacts For Wrapper Packaging](./adrs/0008-stage-neutral-runtime-artifacts-for-wrapper-packaging.md)
- [ADR 0004: Release Orchestration Stays Core-Owned](./adrs/0004-release-orchestration-stays-core-owned.md)
- [ADR 0009: Centralize Release State Policy](./adrs/0009-centralize-release-state-policy.md)
- [ADR 0011: Consolidate Release Assertion Commands](./adrs/0011-consolidate-release-assertion-commands.md)
- [ADR 0005: Verification Enforces Source And Installed Contracts](./adrs/0005-verification-enforces-source-and-installed-contracts.md)
