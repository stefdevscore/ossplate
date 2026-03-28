# Ossplate Roadmap

## Current State

`ossplate` is no longer a placeholder scaffold. It now ships as a real multi-registry tool with:

- a canonical Rust CLI
- thin npm and Python wrappers
- real `validate`, `sync`, `create`, and `init` commands
- curated scaffold payloads in installed wrapper artifacts
- CI quality gates across Rust, JS, and Python
- OIDC-first publishing for PyPI, Cargo, and npm, with token fallbacks where configured

Published names are aligned as:

- crates.io: `ossplate`
- npm: `ossplate`
- PyPI: `ossplate`
- CLI: `ossplate`

## Completed

### Foundation

- Placeholder identity validation is wired into CI.
- `ossplate.toml` is the canonical identity source.
- Root docs, testing docs, customization docs, and release docs now exist.

### Product Surface

- Rust is the only source of product logic.
- JS and Python are thin wrappers around the packaged binary.
- The command surface is now real:
  `version`, `validate`, `sync`, `create`, `init`.

### Packaging And Artifact Reality

- Installed npm and Python artifacts carry the scaffold payload required by `create` and `init`.
- Scaffold payload staging is curated by manifest rather than broad repo-copy behavior.
- Artifact tests verify required content and exclude known non-shipping content.

### Quality Gates

- CI enforces Rust format, clippy, and tests.
- CI enforces JS build, tests, and package dry-run.
- CI enforces Python tests and wheel build.
- Wrapper parity and installed-artifact smoke paths are exercised.
- `./scripts/verify.sh` mirrors the local gate.

### Publishing

- Publish flows are rerun-safe.
- PyPI uses OIDC.
- Cargo uses OIDC with `CARGO_TOKEN` fallback.
- npm uses OIDC with `NPM_TOKEN` fallback.
- The release operator flow is documented in [`docs/releases.md`](./releases.md).

## Remaining Priorities

## P1

- Expand `sync` ownership carefully into a few more identity-only surfaces where bounded ownership is safe.
- Improve `validate` and `sync --check` output further if field-level drift becomes hard to read at scale.
- Add a short architecture note describing the current boundary:
  Rust core as product logic, JS/Python as adapters, scaffold payload as distribution asset.

## P2

- Add optional guidance for generated projects that need browser-based live end-to-end tests.
- Add examples of failure triage for packaging, publish, and parity regressions.
- Tighten workflow/docs ownership boundaries further if more fields become sync-managed.

## P3

- Add architecture-shell guidance for teams that want to scale generated projects into a fuller hexagonal layout.
- Add ADR guidance if the scaffold evolves into a broader product platform.

## Release Maintenance

- Refresh GitHub Actions dependencies ahead of the Node 20 runner deprecation.
- Keep trusted publishing and token fallback documentation aligned with real registry configuration.
- Keep release-version bumps aligned across Cargo, npm, and PyPI metadata.

## Suggested Next Work

1. Decide whether `sync` should own any additional identity-only fields beyond the current bounded set.
2. Improve `validate` and `sync --check` if larger repos need richer drift output.
3. Add optional browser-e2e guidance for generated projects that include a UI.
