# Testing Guide

`ossplate` uses layered verification so the scaffold stays usable as both a source checkout and an installed wrapper distribution.

## Default Layers

### Smoke

- `cargo run --manifest-path core-rs/Cargo.toml -- validate`
- `cargo run --manifest-path core-rs/Cargo.toml -- sync --check`
- `cargo run --manifest-path core-rs/Cargo.toml -- create ../tmp-project`
- `cargo run --manifest-path core-rs/Cargo.toml -- init --path ../tmp-project`

### Unit And Integration

- `cargo test`
- `npm test`
- `PYTHONPATH=src python3 -m unittest discover -s tests -p 'test_*.py'`

These suites cover:

- Rust command parsing and metadata sync behavior
- wrapper parity against the Rust core
- installed npm and wheel artifact smoke checks for `version`, `create`, and `validate`

### Packaging

- `npm pack --dry-run` in `wrapper-js/`
- `python -m build --wheel` in `wrapper-py/`

JavaScript packaging stages distribution assets through `prepack`.

Python packaging stages distribution assets through the Hatch build hook in `wrapper-py/hatch_build.py`.

Artifact assertions are part of the required packaging layer:

- npm tarball content must include the curated scaffold files from `scaffold-manifest.json`
- npm tarball content must exclude wrapper test files and repo-only validation scripts
- Python wheel content must include the curated scaffold files from `scaffold-manifest.json`
- Python wheel content must exclude wrapper test files and repo-only validation scripts

## Recommended Local Order

Default path:

```bash
./scripts/verify.sh
```

Underlying command order:

1. `cargo fmt --check`
2. `cargo clippy --manifest-path core-rs/Cargo.toml -- -D warnings`
3. `cargo test`
4. `cargo run --quiet --manifest-path core-rs/Cargo.toml -- validate --json`
5. `cargo run --quiet --manifest-path core-rs/Cargo.toml -- sync --check`
6. `npm test`
7. `npm pack --dry-run`
8. `PYTHONPATH=src python3 -m unittest discover -s tests -p 'test_*.py'`
9. `python -m build --wheel`

## CI Expectations

CI currently enforces:

- template readiness via `validate` and `sync --check`
- Rust formatting, clippy, and tests
- JS build, tests, and package dry-run
- Python tests and wheel build

The current artifact tests are the required release-confidence floor. Future phases can add broader platform coverage or slower end-to-end suites without changing the basic layered model.
