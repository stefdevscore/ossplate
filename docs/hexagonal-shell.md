# Hexagonal Shell

This document defines the scaling model for `ossplate`.

The goal is not to force framework ceremony onto a small tool. The goal is to keep one product architecture centered in Rust, preserve thin wrappers, and make growth predictable.

## HEX-01 Core Rules

- one product architecture
- Rust is the behavioral source of truth
- JavaScript and Python are adapter packages with local structure only
- boundaries should be explicit enough to test and enforce
- ports are justified only where the boundary is external, volatile, or high-risk

## HEX-02 Target Slices

The shell groups the product into five slices:

### HEX-02A Core Execution

Owns:

- command parsing
- command dispatch
- use-case orchestration
- invariants around owned metadata and scaffold operations

### HEX-02B Scaffold Synthesis

Owns:

- source-root discovery for create/init
- projection rules from source checkout to generated project
- identity override application before sync
- guardrails around target layout and source-tree boundaries

### HEX-02C Package Metadata Sync

Owns:

- `ossplate.toml` as shared identity source
- Cargo, npm, and Python metadata synchronization
- wrapper README identity synchronization
- bounded workflow display-name synchronization
- drift detection and sync-check behavior

### HEX-02D Registry Release Orchestration

Owns:

- publish command semantics
- release-state and publish-readiness boundaries
- local operator publish behavior
- registry sequencing rules
- rerun safety

### HEX-02E Verification

Owns:

- source verification
- unit and integration tests
- package artifact assertions
- installed-artifact smoke checks
- live registry E2E
- future architecture checks

## HEX-03 Per-Package Rule

### HEX-03A Rust

Rust carries the real internal architecture. Product behavior, invariants, metadata policy, scaffold policy, and publish policy should grow here first.

### HEX-03B TypeScript

The TypeScript package stays adapter-local. It may own packaged binary resolution, environment setup, and delegation into the Rust binary. It should not become a peer product core.

### HEX-03C Python

The Python package follows the same rule. It may own target-specific wheel handoff, packaged binary lookup, environment setup, and delegation. It should not become a second product core.

## HEX-04 Current Implementation Status

The shell is partly implemented already:

- core execution exists in `core-rs/src/main.rs`
- package metadata sync exists in `core-rs/src/sync.rs`
- release orchestration exists in `core-rs/src/release.rs`
- scaffold synthesis exists in `core-rs/src/scaffold.rs`
- verification exists through `scripts/verify.sh`, artifact checks, CI, and live E2E

What is still pending is deeper enforcement, not basic slice existence:

- cleaner code-to-slice mapping in docs and tests
- more direct architecture checks
- stronger slice-local test organization

## HEX-05 Dependency Direction

Use this dependency direction:

- wrappers and delivery edges -> core execution
- scaffold synthesis -> core execution rules where needed, but not wrapper behavior
- package metadata sync -> core execution and owned identity rules
- release orchestration -> core execution plus external registry/process boundaries
- verification -> every slice, but only as observer and enforcer

The important interpretation is:

- one shell for the product
- small adapter-local structure in JS/Python
- not three equal hexagonal architectures

## HEX-06 Ports And Adapters

Use ports only where they buy real stability:

- process execution for external publish tools
- registry existence and publish boundaries
- scaffold source vs shipped scaffold boundaries
- wrapper-to-core runtime handoff

Do not introduce ports just to decorate stable local logic like simple metadata rewrites or local file transformations.
