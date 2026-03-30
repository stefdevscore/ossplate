# Testing

`ossplate` uses layered verification so the source checkout and installed wrapper artifacts stay aligned.

For agent-operated repos, pair this guide with [Agents](./agents.md). The key agent-safe commands are `validate --json`, `sync --check`, and `verify.sh`.

## TEST-01 Verification Layers

### TEST-01A Core Smoke

- `cargo run --manifest-path core-rs/Cargo.toml -- validate`
- `cargo run --manifest-path core-rs/Cargo.toml -- sync --check`
- `cargo run --manifest-path core-rs/Cargo.toml -- create ../tmp-project`
- `cargo run --manifest-path core-rs/Cargo.toml -- init --path ../tmp-project`

### TEST-01B Unit And Integration

- `cargo test`
- `npm test`
- `PYTHONPATH=src python3 -m unittest discover -s tests -p 'test_*.py'`

These cover:

- Rust command parsing and slice behavior
- wrapper parity against the Rust core
- packaged artifact smoke checks for `version`, `create`, and `validate`
- create/init guardrails such as non-empty targets and source-tree rejection

### TEST-01C Packaging And Artifact Checks

- `npm pack --dry-run` in `wrapper-js/`
- runtime package dry-runs under `wrapper-js/platform-packages/`
- `OSSPLATE_PY_TARGET=<target> python -m build --wheel` in `wrapper-py/`
- `python -m build --sdist` in `wrapper-py/`

Required assertions include:

- scaffold files come from `scaffold-payload.json`
- scaffold mirrors under `wrapper-js/scaffold/` and `wrapper-py/src/ossplate/scaffold/` are generated copies of the root payload and must be regenerated, not edited as source
- top-level npm packages exclude bundled runtime binaries
- runtime npm packages contain exactly one target binary
- Python wheels contain exactly one target binary
- wheels exclude binaries for other targets
- wrapper test files and repo-only validation scripts stay out of shipped artifacts

### TEST-01D Live Installed E2E

Published install flows are exercised through:

- `cargo install ossplate`
- `npm install ossplate`
- `pip install ossplate`

The documented operator flow lives in [Live E2E](./live-e2e.md).

This layer matters architecturally because it validates JS and Python as delivery adapters for the Rust core rather than alternate implementations. npm live install E2E is already part of the current confidence path.

Published release confidence now also includes a native-runner post-publish matrix in `.github/workflows/live-e2e-published.yml` across:

- `ubuntu-latest`
- `macos-14`
- `macos-15-intel`
- `windows-latest`

## TEST-02 Default Local Gate

Run:

```bash
./scripts/verify.sh
```

That gate currently runs, in order:

1. `cargo fmt --check`
2. `cargo clippy --manifest-path core-rs/Cargo.toml -- -D warnings`
3. `cargo test`
4. `cargo run --quiet --manifest-path core-rs/Cargo.toml -- validate --json`
5. `cargo run --quiet --manifest-path core-rs/Cargo.toml -- sync --check`
6. `node --test scripts/release-plan.test.mjs`
7. `node --test scripts/release-check.test.mjs`
8. `node --test scripts/release-state.test.mjs`
9. `node --test scripts/publish-local.test.mjs`
10. `node scripts/release-check.mjs scaffold-mirrors`
11. `node scripts/release-check.mjs release-state`
12. `node scripts/assert-js-lockfile-state.mjs <resolved-or-placeholder>`
13. `node scripts/release-check.mjs publish-readiness publish`
14. `npm test` when the current npm version is already published
15. `npm pack --dry-run` when the current npm version is already published
16. `PYTHONPATH=src python3 -m unittest discover -s tests -p 'test_*.py'`

The important JS release checks are:

- `scripts/release-check.mjs scaffold-mirrors`
- `scripts/release-check.mjs release-state`
- `scripts/assert-js-lockfile-state.mjs`
- `scripts/release-check.mjs publish-readiness publish`

If the current npm version is not yet published, `verify.sh` keeps the lockfile in placeholder mode and skips install-based JS checks for that run.

## TEST-03 CI

CI currently enforces:

- template readiness via `validate` and `sync --check`
- template-readiness tests through `scripts/release-plan.test.mjs`, `scripts/release-check.test.mjs`, and `scripts/validate-template-readiness.test.mjs`
- scaffold-mirror integrity through `scripts/release-check.mjs scaffold-mirrors`
- Rust formatting, clippy, and tests
- JS lockfile assertions for resolved vs placeholder source state
- JS build, tests, and package dry-runs
- JS runtime package dry-runs on supported target runners
- Python source tests and target wheel validation
- release and publish readiness assertions before release automation runs
- npm installed E2E from packed Linux runtime and top-level npm artifacts

Pushes to `dev` and `main` both run the CI workflow so release-facing breakage can show up before promotion to `main`.

Published releases also trigger native-runner live installed E2E on the supported GitHub-hosted OS matrix for Cargo, npm, and PyPI installs. That is a post-publish confidence layer, not part of the push/PR CI gate.

## TEST-04 Architecture Meaning

In the hexagonal shell, verification is its own slice. These checks enforce the intended boundaries between:

- the Rust behavioral core
- the thin JS/Python wrappers
- the scaffold projection
- the release surface

For release operator steps and rerun-safe publish behavior, see [Releases](./releases.md).
