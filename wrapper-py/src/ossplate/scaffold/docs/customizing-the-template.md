# Adopting The Scaffold

Use this checklist after creating or cloning a scaffold managed by `ossplate`. The goal is to adopt the scaffold under your own project identity and then use the tool to keep owned metadata in sync.

## Canonical Source Of Truth

`ossplate.toml` is the canonical source of truth for the shared project identity.

It currently owns:

- project name
- project description
- repository URL
- license
- author name/email
- Rust crate name
- npm package name
- Python package name
- CLI command name

`ossplate validate` checks that the owned surfaces match this config.

`ossplate sync` rewrites the owned surfaces back into alignment.

## Validation Policy

- Template-source identities are allowed only when the repository is discussing `ossplate` itself.
- Inherited identities are not allowed in shipping metadata or package-facing docs for an adopted project.
- Command and package naming must be chosen intentionally before release.
- Author, repository, and license fields must be set explicitly rather than inherited accidentally.

The current validator follows this policy through the Rust core rather than a standalone JS rule engine.

## Required Identity Changes

Replace these inherited defaults before reuse:

| Surface | Current scaffold value | Where it lives |
| --- | --- | --- |
| Rust crate name | `ossplate` | `core-rs/Cargo.toml` |
| npm package name | `ossplate` | `wrapper-js/package.json` |
| PyPI package name | `ossplate` | `wrapper-py/pyproject.toml` |
| CLI command | `ossplate` | `ossplate.toml`, `wrapper-js/package.json`, `wrapper-py/pyproject.toml` |
| Repository URL | `https://github.com/stefdevscore/ossplate` | Rust, npm, Python metadata |
| Author/email | `Stef <stefdevscore@github.com>` / `stefdevscore@github.com` | Rust, npm, Python metadata |
| Package-facing scaffold branding | `ossplate` identity in wrapper docs | `wrapper-js/README.md`, `wrapper-py/README.md` |

## Files To Review

- `README.md`
- `ossplate.toml`
- `core-rs/Cargo.toml`
- `wrapper-js/package.json`
- `wrapper-js/README.md`
- `wrapper-py/pyproject.toml`
- `wrapper-py/README.md`
- `.github/workflows/*.yml`

## What `ossplate validate` Enforces

The tool reports:

- drift between `ossplate.toml` and the owned manifest fields
- drift between `ossplate.toml` and wrapper package README identity
- missing or malformed owned metadata in Cargo, npm, or Python surfaces

The tool does not currently rewrite or own:

- CI workflow logic
- publish workflow auth logic
- the root README body beyond the marked identity block
- arbitrary docs prose

The root `README.md` now has a bounded identity section managed by `ossplate sync`. Content outside that marker block remains intentionally manual.

The workflow files now expose a similarly bounded identity surface:

- `.github/workflows/ci.yml`
- `.github/workflows/publish.yml`
- `.github/workflows/publish-npm.yml`

`sync` owns only the display name between `ossplate:workflow-name` markers. Trigger logic, jobs, auth, and shell steps remain manual.

## First-Run Sequence After Cloning

1. Either update `ossplate.toml` directly or use `create` / `init` with identity flags.
2. Run `cargo run --manifest-path core-rs/Cargo.toml -- sync`.
3. Run `cargo run --manifest-path core-rs/Cargo.toml -- validate`.
4. Run the layered verification flow from [Testing Guide](./testing.md).
5. Only then expand product code or publish configuration.

## Create Workflow

To scaffold a fresh target from the current template tree:

```bash
cargo run --manifest-path core-rs/Cargo.toml -- create ../my-new-project \
  --name "My Project" \
  --repository "https://github.com/acme/my-project" \
  --author-name "Acme OSS" \
  --author-email "oss@acme.dev" \
  --rust-crate "my-project-core" \
  --npm-package "@acme/my-project" \
  --python-package "my-project-py" \
  --command "my-project"
```

That copies the curated scaffold payload into the target directory, applies any identity overrides to `ossplate.toml`, then runs `sync` on the new target.

The packaged scaffold intentionally excludes wrapper test suites and maintainer-only validation scripts. Generated projects get the delivery baseline and operator docs, not the source repo's internal test harness.

## Init Workflow

To hydrate an existing directory in place:

```bash
cargo run --manifest-path core-rs/Cargo.toml -- init \
  --path ../existing-project \
  --name "Existing Project" \
  --command "existing-project"
```

`init` ensures the expected scaffold layout exists, copies any missing scaffold files, applies any requested identity overrides, and then runs `sync` so owned metadata matches `ossplate.toml`.

## Supported Identity Flags

- `--name`
- `--description`
- `--repository`
- `--license`
- `--author-name`
- `--author-email`
- `--rust-crate`
- `--npm-package`
- `--python-package`
- `--command`

## Why This Exists

This tool is trying to optimize for a real delivery baseline:

- CI should fail before inherited identity reaches a release path.
- package metadata should not be inherited by accident
- future phases can add richer scaffold creation and maintenance without first cleaning up metadata drift
