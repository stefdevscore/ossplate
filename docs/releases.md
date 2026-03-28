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
cd ../wrapper-py && python -m build --wheel
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

1. Commit the version bump and any release notes.
2. Push `dev`.
3. Create a GitHub release that targets `dev`, or manually dispatch the publish workflows.
4. Monitor:
   - [`.github/workflows/publish.yml`](../.github/workflows/publish.yml)
   - [`.github/workflows/publish-npm.yml`](../.github/workflows/publish-npm.yml)

## Rerun Behavior

The publish jobs are intentionally rerun-safe.

- Cargo checks whether the crate version already exists before attempting publish.
- npm checks whether the package version already exists before attempting publish.
- PyPI uses `skip-existing: true`.

So a second run for the same version should usually succeed by skipping work rather than failing destructively.

## Current Published Names

- crates.io: `ossplate`
- npm: `ossplate`
- PyPI: `ossplate`
- CLI: `ossplate`

## Maintenance

- Refresh GitHub Actions dependencies before the Node 20 runner deprecation becomes a problem.
- Keep the release auth docs aligned with real registry configuration whenever trusted publishing or token fallback changes.
