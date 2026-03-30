# Agent Operations

This guide is safe to ship in generated repos. It describes the repo-local command loop rather than `ossplate` bootstrap operations.

## OPS-01 High-Signal Loop

Use this sequence:

1. `cargo run --manifest-path core-rs/Cargo.toml -- validate --json` to inspect repo health in machine-readable form.
2. `cargo run --manifest-path core-rs/Cargo.toml -- sync --check --json` to confirm owned metadata is aligned without mutating state.
3. `cargo run --manifest-path core-rs/Cargo.toml -- sync --plan --json` to inspect proposed repairs before mutation.
4. `cargo run --manifest-path core-rs/Cargo.toml -- sync --json` to apply bounded repairs and capture the changed files.
5. `cargo run --manifest-path core-rs/Cargo.toml -- verify --json` when the full repo gate is needed in machine-readable form.
6. `./scripts/verify.sh` when a shell-oriented full gate is preferred.

The goal is to keep automation inside the repo's supported ownership boundary instead of rewriting files heuristically.

## OPS-02 What These Commands Mean

- `validate --json` reports whether the repo is internally consistent and returns structured issues.
- `sync --check --json` reports drift without mutating files.
- `sync --plan --json` reports the exact owned files and synced content that would be written.
- `sync --json` applies that bounded repair and returns the same changed-file surface.
- `verify --json` runs the same full gate and returns structured per-step results.
- `verify.sh` runs the same source, packaging, and release-facing verification contract in shell form.

## OPS-03 Safe Mutation Model

Prefer this order:

1. update canonical repo identity or owned metadata inputs
2. run `cargo run --manifest-path core-rs/Cargo.toml -- validate --json`
3. inspect `cargo run --manifest-path core-rs/Cargo.toml -- sync --check --json` or `-- sync --plan --json`
4. run `cargo run --manifest-path core-rs/Cargo.toml -- sync --json` only when the bounded repair is intended
5. rerun `cargo run --manifest-path core-rs/Cargo.toml -- validate --json`
6. run `cargo run --manifest-path core-rs/Cargo.toml -- verify --json` for a structured full-gate result or `./scripts/verify.sh` for the shell form

## OPS-04 What Not To Assume

Do not assume:

- wrapper packages own product behavior
- generated scaffold mirrors are editable source
- package metadata can be hand-edited safely without validation
- release state can be inferred from one manifest alone

Use the repo's own machine-readable contract instead.
