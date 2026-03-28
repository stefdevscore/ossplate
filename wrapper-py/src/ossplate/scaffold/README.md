<!-- ossplate:readme-identity:start -->
# Ossplate

A practical baseline for shipping one project across Cargo, npm, and PyPI without starting from scratch every time.
<!-- ossplate:readme-identity:end -->

## What This Tool Gives You

- a canonical Rust CLI in [`core-rs/`](./core-rs)
- a thin npm wrapper in [`wrapper-js/`](./wrapper-js)
- a thin Python wrapper in [`wrapper-py/`](./wrapper-py)
- normal `push` / `pull_request` CI
- rerun-safe publish workflows for npm, PyPI, and Cargo
- setup and upgrade docs in [`docs/`](./docs/README.md)

## Philosophy

This project is intentionally small.

It exists to validate and synchronize the shared identity of a multi-registry OSS scaffold before broader scaffolding features are added.

## Core Commands

```bash
cargo run --manifest-path core-rs/Cargo.toml -- validate
cargo run --manifest-path core-rs/Cargo.toml -- sync --check
cargo run --manifest-path core-rs/Cargo.toml -- create ../my-new-project
cargo run --manifest-path core-rs/Cargo.toml -- init --path ../existing-project
```

Wrapper installs expose the same command surface as `ossplate`.

`create` and `init` now work from packaged wrapper artifacts as well as a source checkout. Installed wrappers carry a curated scaffold payload rather than a broad repo snapshot. Use flags to set the project identity during scaffold creation instead of editing `ossplate.toml` by hand first:

```bash
cargo run --manifest-path core-rs/Cargo.toml -- create ../my-new-project \
  --name "My Project" \
  --repository "https://github.com/acme/my-project" \
  --author-name "Acme" \
  --author-email "oss@acme.dev" \
  --rust-crate "my-project" \
  --npm-package "@acme/my-project" \
  --python-package "my-project-py" \
  --command "my-project"
```

## Local Verify

Run the full local verification flow with:

```bash
./scripts/verify.sh
```

This is the recommended local mirror of the CI gate.

## Verification

- local workflow and layered testing guidance live in [docs/testing.md](./docs/testing.md)
- contributor workflow lives in [CONTRIBUTING.md](./CONTRIBUTING.md)
- `ossplate validate` reports owned metadata drift
- `ossplate sync --check` fails if owned metadata would be rewritten
- JS and Python artifact tests prove installed distributions can run `version`, `create`, and `validate`

## Release Auth

- PyPI publishes from [`.github/workflows/publish.yml`](./.github/workflows/publish.yml) via GitHub OIDC trusted publishing
- Cargo publishes from [`.github/workflows/publish.yml`](./.github/workflows/publish.yml) via OIDC trusted publishing with `secrets.CARGO_TOKEN` as fallback
- npm publishes from [`.github/workflows/publish-npm.yml`](./.github/workflows/publish-npm.yml) via OIDC trusted publishing with `secrets.NPM_TOKEN` as fallback

## License

Licensed under the [Unlicense](LICENSE).
