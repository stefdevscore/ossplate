# Releases

Use this guide when cutting or recovering an `ossplate` release.

## REL-01 Current Registry Model

- crates.io publishes `ossplate`
- npm publishes `ossplate` plus runtime packages
- PyPI publishes `ossplate`
- the CLI name is `ossplate`

Current npm runtime package names are:

- `ossplate-darwin-arm64`
- `ossplate-darwin-x64`
- `ossplate-linux-x64`
- `ossplate-windows-x64`

`win32-x64` remains the internal target identifier. `ossplate-windows-x64` is the published Windows npm package name.

## REL-02 Required Preflight

Run the full gate before releasing:

```bash
./scripts/verify.sh
```

The release gate includes:

- `scripts/release-check.mjs release-state`
- `scripts/assert-js-lockfile-state.mjs resolved`
- `scripts/release-check.mjs publish-readiness publish`

## REL-03 Local Operator Publish

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

## REL-08 Maintenance Notes

- keep auth and trusted publishing docs aligned with real workflow configuration
- keep lockfile guidance aligned with the current resolved-vs-placeholder release model
- treat release docs as operator guidance, not a historical log
