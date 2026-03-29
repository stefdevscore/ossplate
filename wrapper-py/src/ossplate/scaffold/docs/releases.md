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

The release gate now includes:

- `scripts/assert-release-state.mjs` for internal version and artifact invariants
- `scripts/assert-js-lockfile-state.mjs` for the checked-in JS lockfile contract
- `scripts/assert-publish-readiness.mjs` for external npm publish readiness

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

- macOS Apple Silicon: `ossplate-darwin-arm64`
- macOS Intel: `ossplate-darwin-x64`
- Linux x64: `ossplate-linux-x64`
- Windows x64: `ossplate-win32-x64`

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
3. [`.github/workflows/release.yml`](../.github/workflows/release.yml) computes the next version from commit messages, runs release-state and publish-readiness preflight checks, then bumps versions, commits the release, and tags it.
4. The release workflow dispatches the publish workflows and waits for both of them to finish successfully.
5. After downstream publish success, the release workflow refreshes `wrapper-js/package-lock.json` against the now-published runtime packages and commits that lockfile sync back to `main`.
6. Only after downstream publish success does the release workflow create the GitHub release.
7. Successful completion of the release workflow means all intended registry publishes either completed or skipped safely.

The downstream publish workflows are:
   - [`.github/workflows/publish.yml`](../.github/workflows/publish.yml)
   - [`.github/workflows/publish-npm.yml`](../.github/workflows/publish-npm.yml)

## Workflow Friction To Expect

The release pipeline mutates `main` on GitHub.

- After CI passes, [`.github/workflows/release.yml`](../.github/workflows/release.yml) commits the version bump directly to `origin/main`.
- After npm publish succeeds, the same workflow may add a second commit that refreshes `wrapper-js/package-lock.json` to the published runtime package versions.
- That means a local checkout that was clean before release can become behind `origin/main` without any new manual commits from you.
- Before pushing follow-up work, always refresh your branch first:

```bash
git fetch origin
git rebase origin/main
```

- If a push is rejected as non-fast-forward right after a green release, this version-bump commit is the expected cause.
- If npm published successfully, the post-publish lockfile sync commit can also be the expected cause.

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

If a release created the bump commit/tag but one registry publish failed, treat that version as a failed release candidate. Fix the pipeline and cut the next patch release instead of trying to normalize the broken version in place.

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
  - `ossplate-linux-x64`
  - `ossplate-darwin-arm64`
  - `ossplate-darwin-x64`
  - `ossplate-windows-x64`
- Users still install `ossplate`; npm resolves the matching runtime package through `optionalDependencies`.
- The top-level npm publish now checks that every expected runtime package version is visible on npm before publishing `ossplate`.
- Release preflight fails if the next npm version is already partially published or the runtime package names are not publishable on the public registry.
- The checked-in `wrapper-js/package-lock.json` is a source/CI artifact. Release commits carry a placeholder-compatible lockfile, then the workflow refreshes it after the runtime packages are published so future CI can keep using `npm ci`.

## Current Published Names

- crates.io: `ossplate`
- npm: `ossplate`
- PyPI: `ossplate`
- CLI: `ossplate`

## Maintenance

- Refresh GitHub Actions dependencies before the Node 20 runner deprecation becomes a problem.
- Keep the release auth docs aligned with real registry configuration whenever trusted publishing or token fallback changes.
