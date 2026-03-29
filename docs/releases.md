# Releases

Use this guide when cutting a new `ossplate` release.

## Current Registry Setup

- PyPI publishes from [`.github/workflows/publish.yml`](../.github/workflows/publish.yml) via GitHub OIDC trusted publishing.
- crates.io publishes from [`.github/workflows/publish.yml`](../.github/workflows/publish.yml) via OIDC trusted publishing with `CARGO_TOKEN` fallback.
- npm publishes from [`.github/workflows/publish-npm.yml`](../.github/workflows/publish-npm.yml) via OIDC trusted publishing with `NPM_TOKEN` fallback.

## Before Release

Run the full gate before creating a release:

```bash
./scripts/verify.sh
```

Optional local packaging confidence checks:

```bash
cargo package --manifest-path core-rs/Cargo.toml
cargo publish --manifest-path core-rs/Cargo.toml --dry-run
cd wrapper-js && npm pack --dry-run
cd ../wrapper-py && OSSPLATE_PY_TARGET=linux-x64 python -m build --wheel
cd ../wrapper-py && python -m build --sdist
```

## Versioning

Keep the registry versions aligned across:

- [`core-rs/Cargo.toml`](../core-rs/Cargo.toml)
- [`wrapper-js/package.json`](../wrapper-js/package.json)
- [`wrapper-py/pyproject.toml`](../wrapper-py/pyproject.toml)

After updating versions, rerun:

```bash
./scripts/verify.sh
```

## Release Flow

1. Merge or push work to `main`.
2. Let [`.github/workflows/ci.yml`](../.github/workflows/ci.yml) pass on that commit.
3. [`.github/workflows/release.yml`](../.github/workflows/release.yml) computes the next version from commit messages, bumps versions, commits the release, tags it, and creates a GitHub release.
4. Successful completion of the release workflow triggers:
   - [`.github/workflows/publish.yml`](../.github/workflows/publish.yml)
   - [`.github/workflows/publish-npm.yml`](../.github/workflows/publish-npm.yml)

Optional local hook setup uses [`pre-commit`](https://pre-commit.com/) through [`.pre-commit-config.yaml`](../.pre-commit-config.yaml). It is not required for CI or release automation.

## Bump Rules

- `feat:` => minor
- `fix:`, `docs:`, `refactor:`, `test:`, `chore:`, `build:`, `ci:` => patch
- `!` in the conventional commit header or `BREAKING CHANGE` in the body => major
- `[major]`, `[minor]`, `[patch]` override the inferred bump

## Rerun Behavior

The publish jobs are intentionally rerun-safe.

- Cargo checks whether the crate version already exists before attempting publish.
- npm checks whether the package version already exists before attempting publish.
- PyPI uses `skip-existing: true`.

So a second run for the same version should usually succeed by skipping work rather than failing destructively.

## Python Wheels

- PyPI publishes one wheel per supported target and one sdist.
- Current target runners are:
  - `ubuntu-latest` -> `linux-x64`
  - `macos-14` -> `darwin-arm64`
  - `macos-15-intel` -> `darwin-x64`
  - `windows-latest` -> `win32-x64`
- Each wheel bundles exactly one native `ossplate` executable for its target, so wheel filenames are platform-specific.

## Current Published Names

- crates.io: `ossplate`
- npm: `ossplate`
- PyPI: `ossplate`
- CLI: `ossplate`

## Maintenance

- Refresh GitHub Actions dependencies before the Node 20 runner deprecation becomes a problem.
- Keep the release auth docs aligned with real registry configuration whenever trusted publishing or token fallback changes.
