# Releases

Use this guide when cutting or recovering an `ossplate` release.

## Current Registry Model

- crates.io publishes `ossplate`
- npm publishes `ossplate` plus runtime packages
- PyPI publishes `ossplate`
- the CLI name is `ossplate`

Current npm runtime package names for this repository are:

- `ossplate-darwin-arm64`
- `ossplate-darwin-x64`
- `ossplate-linux-x64`
- `ossplate-windows-x64`

The checked-in runtime package folders still use target-oriented paths such as `wrapper-js/platform-packages/ossplate-win32-x64/`, but the published npm package name is derived from the public package identity plus the runtime suffix. For Windows that means:

- internal target and folder: `win32-x64`
- published runtime package: `ossplate-windows-x64`

## Release Flow

Use the repo-local planner before any publish attempt:

```bash
cargo run --manifest-path core-rs/Cargo.toml -- publish --plan --json
```

That plan reports:

- the helper invocation `publish` will run
- which registries are selected
- whether the run is `--dry-run`
- whether `--skip-existing` recovery mode is enabled

The executable publish surface is:

```bash
cargo run --manifest-path core-rs/Cargo.toml -- publish [--dry-run] [--skip-existing] [--registry <cargo|npm|python|all>]
```

Use cases:

- `--dry-run` for local operator rehearsal without registry mutation
- `--registry <name>` to limit publication to one ecosystem
- `--skip-existing` for rerun-safe recovery when some artifacts already exist upstream

`publish` is intentionally source-checkout only. It orchestrates release helpers from the checked-out repo and does not mutate git state for you.

## Verification Before Publish

Run the full gate before releasing:

```bash
./scripts/verify.sh
cargo run --manifest-path core-rs/Cargo.toml -- publish --plan --json
cargo run --manifest-path core-rs/Cargo.toml -- verify --json
```

The current verification path also checks:

- generated-project dogfooding from the scaffold payload
- resolved versus placeholder JS lockfile state
- package dry-runs for the top-level JS package and runtime packages
- Python wheel build and wrapper tests

If a publish attempt needs recovery, rerun the planner first and then rerun `publish` with the narrowest registry scope and `--skip-existing` only when the upstream state already proves that artifact exists.
