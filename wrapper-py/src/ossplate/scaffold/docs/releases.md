# Releases

Use this guide when cutting a new `ossplate` release.

## Current Registry Setup

- PyPI publishes from [`.github/workflows/publish.yml`](../.github/workflows/publish.yml) via GitHub OIDC trusted publishing.
- crates.io publishes from [`.github/workflows/publish.yml`](../.github/workflows/publish.yml) via OIDC trusted publishing with `CARGO_TOKEN` fallback.
- npm publishes from [`.github/workflows/publish-npm.yml`](../.github/workflows/publish-npm.yml) via OIDC trusted publishing with `NPM_TOKEN` fallback.
  Runtime packages publish first, then the top-level `ossplate` package publishes after they are available.

## Before Release

Run the full gate before creating a release:

```bash
./scripts/verify.sh
```

The release gate now includes `scripts/assert-release-state.mjs`, which fails if versions, scaffold snapshots, npm runtime package identities, or tracked generated binaries drift out of policy.

Optional local packaging confidence checks:

```bash
cargo package --manifest-path core-rs/Cargo.toml
cargo publish --manifest-path core-rs/Cargo.toml --dry-run
cd wrapper-js && npm pack --dry-run
cd wrapper-js/platform-packages/ossplate-<host-target> && npm pack --dry-run
cd ../wrapper-py && OSSPLATE_PY_TARGET=linux-x64 python -m build --wheel
cd ../wrapper-py && python -m build --sdist
```

Use the runtime package that matches the machine you are building on:

- macOS Apple Silicon: `@stefdevscore/ossplate-darwin-arm64`
- macOS Intel: `@stefdevscore/ossplate-darwin-x64`
- Linux x64: `@stefdevscore/ossplate-linux-x64`
- Windows x64: `@stefdevscore/ossplate-win32-x64`

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
4. The release workflow asserts release-state integrity before mutating `main`.
5. The release workflow dispatches the publish workflows and waits for both of them to finish successfully.
6. Successful completion of the release workflow means all intended registry publishes either completed or skipped safely.

The downstream publish workflows are:
   - [`.github/workflows/publish.yml`](../.github/workflows/publish.yml)
   - [`.github/workflows/publish-npm.yml`](../.github/workflows/publish-npm.yml)

## Workflow Friction To Expect

The release pipeline mutates `main` on GitHub.

- After CI passes, [`.github/workflows/release.yml`](../.github/workflows/release.yml) commits the version bump directly to `origin/main`.
- That means a local checkout that was clean before release can become behind `origin/main` without any new manual commits from you.
- Before pushing follow-up work, always refresh your branch first:

```bash
git fetch origin
git rebase origin/main
```

- If a push is rejected as non-fast-forward right after a green release, this version-bump commit is the expected cause.

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
- npm runtime packages check whether their package version already exists before attempting publish.
- PyPI uses `skip-existing: true`.

So a second run for the same version should usually succeed by skipping work rather than failing destructively.

If a release created the bump commit/tag but one registry publish failed, treat that as a partial release. Fix the failing workflow, then rerun only the publish workflow for that version rather than cutting a second version immediately.

## Python Wheels

- PyPI publishes one wheel per supported target and one sdist.
- Current target runners are:
  - `ubuntu-latest` -> `linux-x64`
  - `macos-14` -> `darwin-arm64`
  - `macos-15-intel` -> `darwin-x64`
  - `windows-latest` -> `win32-x64`
- Each wheel bundles exactly one native `ossplate` executable for its target, so wheel filenames are platform-specific.
- Wheel size is expected to be dominated by that single executable, not by scaffold duplication.

## JavaScript Runtime Packages

- npm publishes one thin top-level package: `ossplate`
- npm also publishes one platform runtime package per supported target:
  - `@stefdevscore/ossplate-linux-x64`
  - `@stefdevscore/ossplate-darwin-arm64`
  - `@stefdevscore/ossplate-darwin-x64`
  - `@stefdevscore/ossplate-win32-x64`
- Users still install `ossplate`; npm resolves the matching runtime package through `optionalDependencies`.
- The top-level npm publish now checks that every expected runtime package version is visible on npm before publishing `ossplate`.

## Current Published Names

- crates.io: `ossplate`
- npm: `ossplate`
- PyPI: `ossplate`
- CLI: `ossplate`

## Maintenance

- Refresh GitHub Actions dependencies before the Node 20 runner deprecation becomes a problem.
- Keep the release auth docs aligned with real registry configuration whenever trusted publishing or token fallback changes.
