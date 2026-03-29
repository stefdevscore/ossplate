# Testing

`ossplate` uses layered verification so the scaffold stays usable as both a source checkout and an installed wrapper distribution.

## Layers

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
- `npm pack --dry-run` in the current host runtime package under `wrapper-js/platform-packages/`
- `OSSPLATE_PY_TARGET=<target> python -m build --wheel` in `wrapper-py/`
- `python -m build --sdist` in `wrapper-py/`

JavaScript packaging stages distribution assets through `prepack`.

Python packaging stages distribution assets through the Hatch build hook in `wrapper-py/hatch_build.py`.

Artifact assertions are part of the required packaging layer:

- npm tarball content must include the curated scaffold files from `scaffold-manifest.json`
- npm top-level tarball must exclude bundled platform binaries and nested scaffold runtime binaries
- npm runtime package tarballs must contain exactly one target binary
- npm tarball content must exclude wrapper test files and repo-only validation scripts
- Python wheel content must include the curated scaffold files from `scaffold-manifest.json`
- Python wheel content must include exactly one runtime binary for its target
- Python wheel content must exclude binaries for all other targets
- Python wheel content must exclude wrapper test files and repo-only validation scripts
- Python wheel filenames must be platform-specific rather than `py3-none-any`
- Python wheel size must stay within the target-specific budget enforced by the artifact test

### Live Installed E2E

This layer exercises the published artifacts the way an operator or user actually installs them:

- `cargo install ossplate`
- `npm install ossplate`
- `pip install ossplate`

That flow is documented in [`docs/live-e2e.md`](./live-e2e.md).

This layer matters architecturally because it validates the wrappers as delivery adapters for the Rust core rather than alternate product implementations.

### Future Optional Layers

`ossplate` itself does not currently require browser automation or container orchestration, but the template should leave room for them:

- browser/Playwright E2E for frontend-heavy adopters
- Docker or container-matrix E2E for multi-ecosystem install verification

## Default Local Flow

Default path:

```bash
./scripts/verify.sh
```

That gate includes `scripts/assert-release-state.mjs`, which checks version alignment, scaffold snapshot parity, runtime package metadata, and the absence of tracked generated binaries.

It also includes `scripts/assert-js-lockfile-state.mjs resolved`, which checks that `wrapper-js/package-lock.json` matches the supported source-repo lockfile contract:

- root version matches `wrapper-js/package.json`
- root `optionalDependencies` match the runtime package set
- runtime package lock entries exist and stay optional
- runtime package lock entries include `version`, `resolved`, and `integrity`

Release verification uses `scripts/assert-js-lockfile-state.mjs placeholder` during the pre-publish bump window, where unresolved placeholder runtime entries are still expected.

Underlying command order:

1. `cargo fmt --check`
2. `cargo clippy --manifest-path core-rs/Cargo.toml -- -D warnings`
3. `cargo test`
4. `cargo run --quiet --manifest-path core-rs/Cargo.toml -- validate --json`
5. `cargo run --quiet --manifest-path core-rs/Cargo.toml -- sync --check`
6. `node --test scripts/release-plan.test.mjs`
7. `node --test scripts/publish-local.test.mjs`
8. `node scripts/assert-release-state.mjs`
9. `node scripts/assert-js-lockfile-state.mjs <resolved-or-placeholder>`
10. `node scripts/assert-publish-readiness.mjs publish`
11. `npm test`
12. `npm pack --dry-run`
13. `PYTHONPATH=src python3 -m unittest discover -s tests -p 'test_*.py'`

## Live Installed Flow

To test the published registries rather than the source checkout, run:

```bash
./scripts/live-e2e.sh
```

That covers the installed CLI through:

- `cargo install ossplate`
- `npm install ossplate`
- `pip install ossplate`

Each installed CLI must successfully run `version`, `create`, `init`, `validate --json`, and `sync --check` from an isolated temporary environment.

## CI

CI currently enforces:

- template readiness via `validate` and `sync --check`
- Rust formatting, clippy, and tests
- JS build, tests, and package dry-run
- JS runtime package dry-run on each supported target runner
- Python source tests on Linux using host-available runtime binary expectations
- Python wheel validation on `linux-x64`, `darwin-arm64`, `darwin-x64`, and `win32-x64`
- npm installed E2E from the current checkout's packed Linux runtime and top-level npm artifacts

Pushes to `dev` and `main` both run this CI workflow so release-facing breakage can show up before work is promoted to `main`.

The current artifact tests are the required release-confidence floor.

In the hexagonal shell, verification is its own architecture slice. These checks are not just QA coverage; they enforce the intended boundaries between the behavioral core, the wrappers, the scaffold projection, and the release surface.

For release-specific operator steps, version bumps, and rerun-safe publish expectations, see [`docs/releases.md`](./releases.md).
