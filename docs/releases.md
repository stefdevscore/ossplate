# Releases

Use this guide when cutting or recovering an `ossplate` release.

## REL-01 Current Registry Model

- crates.io publishes `ossplate`
- npm publishes `ossplate` plus runtime packages
- PyPI publishes `ossplate`
- the CLI name is `ossplate`

Current npm runtime package names for this repository are:

- `ossplate-darwin-arm64`
- `ossplate-darwin-x64`
- `ossplate-linux-x64`
- `ossplate-windows-x64`

`win32-x64` remains the internal target identifier. `ossplate-windows-x64` is the published Windows npm package name.

The checked-in runtime package folders still use target-oriented paths such as `wrapper-js/platform-packages/ossplate-win32-x64/`, but the published npm package name is derived from the public package identity plus the runtime suffix. For Windows that means:

- internal target and folder: `win32-x64`
- published runtime package: `ossplate-windows-x64`

For generated projects, the canonical rule is:

- the top-level npm package may be scoped or unscoped
- runtime package names are derived from that configured npm package plus the target suffix
- on-disk runtime package folders remain scaffold implementation details and are not the public package naming contract

## REL-02 Required Preflight

Run the full gate before releasing:

```bash
./scripts/verify.sh
```

For a structured planning/operator equivalent, use:

```bash
cargo run --manifest-path core-rs/Cargo.toml -- publish --plan --json
cargo run --manifest-path core-rs/Cargo.toml -- verify --json
```

The release gate includes:

- `scripts/release-check.mjs release-state`
- `scripts/assert-js-lockfile-state.mjs resolved`
- `scripts/release-check.mjs publish-readiness publish`
- generated-project dogfooding from the scaffold payload
- package dry-runs for the top-level JS package and runtime packages
- Python wheel build and wrapper tests

## REL-03 Local Operator Publish

Use the repo-local planner before any publish attempt:

```bash
cargo run --manifest-path core-rs/Cargo.toml -- publish --plan --json
```

That plan reports:

- the helper invocation `publish` will run
- which registries are selected
- whether the run is `--dry-run`
- whether `--skip-existing` recovery mode is enabled
- the current host target and local preflight state

The executable publish surface is:

```bash
cargo run --manifest-path core-rs/Cargo.toml -- publish [--dry-run] [--skip-existing] [--registry <cargo|npm|python|all>]
```

For source-based recovery or manual registry work, use:

```bash
cargo run --manifest-path core-rs/Cargo.toml -- publish --dry-run
```

Important facts:

- it publishes the current checked-out version only
- it does not bump versions, tag, or create a GitHub release
- it stops on first failure and prints recovery guidance
- it runs local preflight checks for required tools and detectable auth before publish work starts
- it uses local toolchain and local auth only
- it is host-limited: one machine can only build the current host npm runtime binary and current host Python wheel
- `publish --plan --json` exposes the same helper invocation, selected registries, host target, and local preflight state without mutating anything

Common local flags:

```bash
cargo run --manifest-path core-rs/Cargo.toml -- publish --dry-run
cargo run --manifest-path core-rs/Cargo.toml -- publish --registry npm --skip-existing
cargo run --manifest-path core-rs/Cargo.toml -- publish --registry pypi
```

Auth sources:

- npm: existing `npm login` state or `NPM_TOKEN`
- crates.io: existing cargo auth state or `CARGO_REGISTRY_TOKEN`
- PyPI: `TWINE_USERNAME=__token__` plus `TWINE_PASSWORD`, or equivalent local twine config

Manual publish safeguards:

- the PyPI path clears the host wheel and sdist output directories before build
- the PyPI path requires exactly one fresh host wheel and one fresh sdist before `twine check` or upload
- the npm path waits for runtime package visibility before publishing the top-level `ossplate` package
- npm timeout output calls out propagation delay and lists any missing runtime packages

## REL-04 Release Flow

1. Push releasable work to `main`.
2. Let CI pass on that commit.
3. `release.yml` computes the next version, runs preflight checks, bumps versions, commits the release, and tags it.
4. The release workflow dispatches downstream publish workflows and waits for success.
5. After downstream success, npm settlement is checked and `wrapper-js/package-lock.json` is repaired back into resolved state on `main`.
6. Only after downstream publish success does the workflow create the GitHub release.
7. The published release then triggers native-runner live install E2E through `.github/workflows/live-e2e-published.yml`.

The release workflow mutates `main`, so local branches can fall behind even without manual commits. Refresh before pushing follow-up work:

```bash
git fetch origin
git rebase origin/main
```

## REL-05 Bump Rules

- `feat:` => minor
- `fix:`, `docs:`, `refactor:`, `test:`, `chore:`, `build:`, `ci:` => patch
- `!` in the commit header or `BREAKING CHANGE` in the body => major
- `[major]`, `[minor]`, `[patch]` override the inferred bump

## REL-06 Rerun Behavior

Publish jobs are intentionally rerun-safe:

- cargo checks whether the crate version already exists
- npm checks whether the package version already exists
- npm runtime packages check whether their package version already exists
- PyPI uses `skip-existing`

If a release created the bump commit or tag but a registry publish failed, treat that version as failed. Fix the issue and cut the next patch release rather than trying to normalize the broken version in place.

## REL-07 Wheels And Runtime Packages

Python publishes:

- one wheel per supported target
- one sdist

Current target runners are:

- `ubuntu-latest` -> `linux-x64`
- `macos-14` -> `darwin-arm64`
- `macos-15-intel` -> `darwin-x64`
- `windows-latest` -> `win32-x64`

Each wheel bundles exactly one native executable for its target.

npm publishes:

- one thin top-level package: `ossplate`
- one platform runtime package per supported target

The top-level npm publish waits for runtime package visibility before publishing `ossplate`. Local publish timeout output now calls out npm propagation explicitly and lists any missing runtime packages.

Post-publish confidence also includes native-runner live install checks on the supported GitHub-hosted OS matrix, so registry installs are exercised on real Linux, macOS ARM, macOS Intel, and Windows hosts.

## REL-08 Maintenance Notes

- keep auth and trusted publishing docs aligned with real workflow configuration
- keep lockfile guidance aligned with the current resolved-vs-placeholder release model
- treat release docs as operator guidance, not a historical log
- for recovery runs, rerun the planner first and then rerun `publish` with the narrowest registry scope and `--skip-existing` only when the upstream state already proves that artifact exists
