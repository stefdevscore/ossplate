# Testing

`ossplate` uses layered verification so the source checkout and installed wrapper artifacts stay aligned.

For generated or adopted repos, pair this guide with [Agent Operations](./agent-operations.md). The key agent-safe commands are `validate --json`, `inspect --json`, `upgrade --plan --json`, `sync --check --json`, `sync --plan --json`, `sync --json`, `publish --plan --json`, `verify --json`, and `verify.sh`.

## TEST-01 Verification Layers

### TEST-01A Core Smoke

- `cargo run --manifest-path core-rs/Cargo.toml -- validate`
- `cargo run --manifest-path core-rs/Cargo.toml -- sync --check`
- `cargo run --manifest-path core-rs/Cargo.toml -- create ../tmp-project`
- `cargo run --manifest-path core-rs/Cargo.toml -- init --path ../tmp-project`

### TEST-01B Unit And Integration

- `node scripts/stage-distribution-assets.mjs embedded-template`
- `cargo test`
- `npm test`
- `PYTHONPATH=src python3 -m unittest discover -s tests -p 'test_*.py'`

These cover:

- Rust command parsing and slice behavior
- authored scaffold-version classification and upgrade planning/apply behavior
- wrapper parity against the Rust core
- packaged artifact smoke checks for `version`, `create`, and `validate`
- create/init guardrails such as non-empty targets and source-tree rejection

### TEST-01C Packaging And Artifact Checks

- `node ../scripts/package-js.mjs dry-run-json` from `wrapper-js/`
- runtime package dry-runs under `wrapper-js/platform-packages/`
- `OSSPLATE_PY_TARGET=<target> python -m build --wheel` in `wrapper-py/`
- `python -m build --sdist` in `wrapper-py/`

Required assertions include:

- scaffold files come from `scaffold-payload.json`
- wrapper scaffold payloads are generated from the root payload and must not be tracked as source trees
- top-level npm packages exclude bundled runtime binaries
- runtime npm packages contain exactly one target binary
- Python wheels contain exactly one target binary
- wheels exclude binaries for other targets
- wrapper test files and repo-only validation scripts stay out of shipped artifacts

The embedded-template preparation step matters because `core-rs/build.rs` now consumes a prepared artifact. It no longer shells out to Node during `cargo build` or `cargo test`.

### TEST-01D Live Installed E2E

Published install flows are exercised through:

- `cargo install ossplate`
- `npm install ossplate`
- `pip install ossplate`

The documented operator flow lives in [Live E2E](./live-e2e.md).

This layer matters architecturally because it validates JS and Python as delivery adapters for the Rust core rather than alternate implementations. npm live install E2E is already part of the current confidence path.

The live installed flow now asserts more than installability. Each published-surface run must prove that:

- `create` and `init` both produce descendants that `inspect` classifies as `current`
- `sync --check --json` reports a clean managed surface
- the default shipped scaffold still reports the expected placeholder warnings
- a fully specified custom identity validates without warnings
- a non-default identity override still projects the expected Rust, npm, Python, command, and runtime package names from the shipped scaffold

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

For a structured machine-readable equivalent, use:

```bash
cargo run --manifest-path core-rs/Cargo.toml -- verify --json
```

That gate currently runs, in order:

1. `cargo fmt --check`
2. `node scripts/stage-distribution-assets.mjs embedded-template`
3. `cargo clippy --manifest-path core-rs/Cargo.toml -- -D warnings`
4. `cargo test --manifest-path core-rs/Cargo.toml`
5. `cargo run --quiet --manifest-path core-rs/Cargo.toml -- validate --path <repo> --json`
6. `cargo run --quiet --manifest-path core-rs/Cargo.toml -- sync --path <repo> --check --json`
7. `node --test scripts/release-plan.test.mjs`
8. `node --test scripts/release-check.test.mjs`
9. `node --test scripts/release-state.test.mjs`
10. `node --test scripts/bootstrap-pattern1.test.mjs`
11. `node --test scripts/publish-local.test.mjs`
12. `node scripts/release-check.mjs generated-project-dogfood`
13. `node scripts/release-check.mjs scaffold-assets`
14. `node scripts/release-check.mjs release-state`
15. `node scripts/assert-js-lockfile-state.mjs <resolved-or-placeholder>`
16. `node scripts/release-check.mjs publish-readiness publish`
17. `npm test`
18. `node scripts/package-js.mjs dry-run-json`
19. `PYTHONPATH=src python3 -m unittest discover -s tests -p 'test_*.py'`

The important JS release checks are:

- `scripts/bootstrap-pattern1.test.mjs`
- `scripts/release-check.mjs generated-project-dogfood`
- `scripts/release-check.mjs scaffold-assets`
- `scripts/release-check.mjs release-state`
- `scripts/assert-js-lockfile-state.mjs`
- `scripts/release-check.mjs publish-readiness publish`

Two parts of the gate are easy to miss if you only skim the shell script:

- `stage-distribution-assets.mjs embedded-template` refreshes the generated embedded template before Rust linting and tests run, so verification checks the packaged scaffold shape instead of stale generated artifacts.
- direct Rust builds in the canonical repo rely on that prepared artifact; they do not generate it implicitly anymore.
- `generated-project-dogfood` and `bootstrap-pattern1.test.mjs` validate that a freshly generated project is immediately buildable and wrapper-executable without manual repair.

If the current npm version is not yet published, `verify.sh` still keeps the lockfile in placeholder mode, but it continues to run the local JS wrapper checks. Registry visibility only affects the expected lockfile state, not whether local JS validation runs.

`verify --json` returns the same phase order as structured per-step results with:

- `name`
- `ok`
- `exitCode`
- `stdout`
- `stderr`
- `skipped`
- optional `reason`

## TEST-03 CI

CI currently enforces:

- template readiness via `validate` and `sync --check`
- template-readiness tests through `scripts/release-plan.test.mjs`, `scripts/release-check.test.mjs`, and `scripts/validate-template-readiness.test.mjs`
- scaffold generation integrity and no tracked wrapper mirrors through `scripts/release-check.mjs scaffold-assets`
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
